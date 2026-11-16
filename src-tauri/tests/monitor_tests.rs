use std::collections::HashMap;
use std::fs::{self, File};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use antigravity_agent_monitor_lib::collect_with_provider;
use antigravity_agent_monitor_lib::domain::classifier::{
    classify_transcript_step, SessionStateTracker,
};
use antigravity_agent_monitor_lib::domain::model::{
    AgentSession, AgentStatus, ConnectionState, StatusConfidence,
};
use antigravity_agent_monitor_lib::providers::composite::enrich_with_windows_and_uia;
use antigravity_agent_monitor_lib::providers::hooks::{
    append_hook_event, disable_hooks_at, enable_hooks_at, is_hook_registered, HookEventTailer,
    RecordedHookEvent,
};
use antigravity_agent_monitor_lib::providers::process::{
    extract_workspace_from_title, IdeWindowInfo,
};
use antigravity_agent_monitor_lib::providers::transcript::{
    infer_workspace_folder, parse_iso8601_utc_secs, TranscriptScanner,
};
use antigravity_agent_monitor_lib::providers::{
    AgentMonitorProvider, MonitorError, ProviderFuture, ProviderOutput,
};
use tokio::sync::Mutex;

struct ScriptedProvider {
    result: Result<ProviderOutput, MonitorError>,
}

impl AgentMonitorProvider for ScriptedProvider {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn collect(&self, _now_secs: u64) -> ProviderFuture<'_, Result<ProviderOutput, MonitorError>> {
        let res = self.result.clone();
        Box::pin(async move { res })
    }
}

fn temp_test_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("agm-test-{tag}-{nanos}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn test_status_classification_rules() {
    // Working
    let (status, conf, _) = classify_transcript_step(
        "RUNNING",
        "MODEL",
        "PLANNER_RESPONSE",
        true,
        &["run_command".to_string()],
        3,
        true,
    );
    assert_eq!(status, AgentStatus::Working);
    assert_eq!(conf, StatusConfidence::Inferred);

    // Waiting (interactive tool open past threshold)
    let (status, _, _) = classify_transcript_step(
        "DONE",
        "MODEL",
        "PLANNER_RESPONSE",
        true,
        &["run_command".to_string()],
        45,
        true,
    );
    assert_eq!(status, AgentStatus::Waiting);

    // Completed (recent model turn with no tool calls)
    let (status, _, _) =
        classify_transcript_step("DONE", "MODEL", "PLANNER_RESPONSE", false, &[], 20, true);
    assert_eq!(status, AgentStatus::Completed);

    // Idle (aged turn, not falsely left as Working or Waiting)
    let (status, _, _) =
        classify_transcript_step("DONE", "MODEL", "PLANNER_RESPONSE", false, &[], 1200, true);
    assert_eq!(status, AgentStatus::Idle);

    // Error
    let (status, _, _) =
        classify_transcript_step("ERROR", "MODEL", "PLANNER_RESPONSE", false, &[], 5, true);
    assert_eq!(status, AgentStatus::Error);

    // Unknown when unrecognized & unlocked
    let (status, conf, _) =
        classify_transcript_step("MYSTERY", "MODEL", "GENERIC", false, &[], 5, false);
    assert_eq!(status, AgentStatus::Unknown);
    assert_eq!(conf, StatusConfidence::Unknown);
}

#[test]
fn test_duplicate_session_merge_and_stale_detection() {
    let mut tracker = SessionStateTracker::new(60, 120);
    let t0 = 1_700_000_000;

    // Duplicate ID from transcript (Inferred) and hooks (Confirmed): hooks must win.
    let s_inferred = AgentSession {
        id: "abc-123".to_string(),
        title: "Session abc-123".to_string(),
        workspace: None,
        status: AgentStatus::Idle,
        activity: Some("Session open".to_string()),
        last_activity: Some(t0 - 10),
        last_status_change: Some(t0 - 10),
        source: "transcript".to_string(),
        confidence: StatusConfidence::Inferred,
        stale: false,
        disconnected: false,
    };
    let s_confirmed = AgentSession {
        id: "abc-123".to_string(),
        title: "MIRO (abc-12)".to_string(),
        workspace: Some("d:\\MIRO".to_string()),
        status: AgentStatus::Working,
        activity: Some("Generating model response".to_string()),
        last_activity: Some(t0),
        last_status_change: Some(t0),
        source: "hooks".to_string(),
        confidence: StatusConfidence::Confirmed,
        stale: false,
        disconnected: false,
    };

    let res = tracker.reconcile(vec![s_inferred, s_confirmed], t0, true);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].status, AgentStatus::Working);
    assert_eq!(res[0].confidence, StatusConfidence::Confirmed);
    assert_eq!(res[0].title, "MIRO (abc-12)");
    assert!(!res[0].stale);

    // Advance clock past stale threshold (60s) with no new activity;
    // status remains Working, last_status_change stays t0, stale becomes true & confidence downgraded.
    let stale_session = AgentSession {
        last_activity: Some(t0),
        ..res[0].clone()
    };
    let res2 = tracker.reconcile(vec![stale_session], t0 + 90, true);
    assert_eq!(res2.len(), 1);
    assert_eq!(res2[0].status, AgentStatus::Working);
    assert_eq!(res2[0].last_status_change, Some(t0));
    assert!(res2[0].stale);
    assert_eq!(res2[0].confidence, StatusConfidence::Inferred);
}

#[test]
fn test_transcript_discovery_incremental_tailing_and_corrupt_line_resilience() {
    let root = temp_test_dir("transcript");
    let session_id = "0a3f6dec-7949-4268-bafb-ac2e8f38b897";
    let logs_dir = root.join("brain").join(session_id).join(".system_generated").join("logs");
    let presence_dir = root.join("presence");
    fs::create_dir_all(&logs_dir).unwrap();
    fs::create_dir_all(&presence_dir).unwrap();

    // Hold an open file lock on Windows so is_lock_held returns true.
    let lock_path = presence_dir.join(format!("{session_id}.lock"));
    let _held_lock = File::create(&lock_path).unwrap();

    let transcript_path = logs_dir.join("transcript.jsonl");
    fs::write(
        &transcript_path,
        concat!(
            "{\"step_index\":0,\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"status\":\"DONE\",\"created_at\":\"2026-11-16T15:00:00Z\"}\n",
            "{invalid json line should be skipped\n",
            "{\"step_index\":1,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"RUNNING\",\"created_at\":\"2026-11-16T15:00:05Z\",\"tool_calls\":[{\"name\":\"read_file\"}]}\n"
        ),
    )
    .unwrap();

    let mut scanner = TranscriptScanner::default();
    let t_step1 = parse_iso8601_utc_secs("2026-11-16T15:00:05Z").unwrap();
    let sessions = scanner.scan_directory(&root, t_step1 + 2, true).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].status, AgentStatus::Working);

    // Append completion step and verify incremental tail picks it up.
    use std::io::Write;
    let mut f = fs::OpenOptions::new().append(true).open(&transcript_path).unwrap();
    writeln!(
        f,
        "{{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"created_at\":\"2026-11-16T15:00:25Z\",\"tool_calls\":[]}}"
    )
    .unwrap();
    drop(f);

    let sessions_after = scanner.scan_directory(&root, t_step1 + 30, true).unwrap();
    assert_eq!(sessions_after.len(), 1);
    assert_eq!(sessions_after[0].status, AgentStatus::Completed);

    // Releasing the presence lock (closed tab/window) removes the session immediately.
    drop(_held_lock);
    let sessions_closed = scanner.scan_directory(&root, t_step1 + 35, true).unwrap();
    assert!(sessions_closed.is_empty());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_hook_config_preserves_existing_hooks_and_tailer_reads_events() {
    let dir = temp_test_dir("hooks");
    let cfg_path = dir.join("hooks.json");
    fs::write(
        &cfg_path,
        r#"{"custom-linter":{"enabled":true,"PreToolUse":[]}}"#,
    )
    .unwrap();

    let exe_path = dir.join("Agent Monitor.exe");
    enable_hooks_at(&cfg_path, &exe_path).unwrap();
    assert!(is_hook_registered(&cfg_path));

    let raw = fs::read_to_string(&cfg_path).unwrap();
    let val: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(val.get("custom-linter").is_some());
    let ours = val.get("antigravity-agent-monitor").unwrap();
    assert!(ours.get("PreToolUse").is_none()); // Must never gate tool permissions
    assert!(ours.get("Stop").is_some());

    let log_path = dir.join("hook-events.jsonl");
    let now = 1_700_000_500;
    append_hook_event(
        &log_path,
        &RecordedHookEvent {
            conversation_id: "11111111-2222-3333-4444-555555555555".to_string(),
            hook_event_name: "Stop".to_string(),
            workspace_path: Some("d:\\MIRO".to_string()),
            model_name: Some("auto".to_string()),
            tool_name: None,
            termination_reason: Some("NO_TOOL_CALL".to_string()),
            fully_idle: Some(true),
            has_error: false,
            timestamp: now - 5,
        },
    )
    .unwrap();

    let mut tailer = HookEventTailer::default();
    let sessions = tailer.read_sessions(&log_path, now, true);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].status, AgentStatus::Completed);
    assert_eq!(sessions[0].confidence, StatusConfidence::Confirmed);

    disable_hooks_at(&cfg_path).unwrap();
    assert!(!is_hook_registered(&cfg_path));
    let raw_after = fs::read_to_string(&cfg_path).unwrap();
    let val_after: serde_json::Value = serde_json::from_str(&raw_after).unwrap();
    assert!(val_after.get("custom-linter").is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_window_title_workspace_extraction() {
    assert_eq!(
        extract_workspace_from_title("main.rs - MIRO - Antigravity IDE"),
        Some("MIRO".to_string())
    );
    assert_eq!(
        extract_workspace_from_title("FTune - Antigravity IDE"),
        Some("FTune".to_string())
    );
    assert_eq!(
        extract_workspace_from_title("Core (Workspace) - Antigravity"),
        Some("Core".to_string())
    );
    assert_eq!(extract_workspace_from_title("Antigravity IDE"), None);
    assert_eq!(extract_workspace_from_title("Welcome - Antigravity IDE"), None);

    assert_eq!(
        infer_workspace_folder(&[
            "d:\\MIRO\\src\\App.tsx".to_string(),
            "d:\\MIRO\\package.json".to_string(),
        ]),
        Some("MIRO".to_string())
    );
    assert_eq!(
        infer_workspace_folder(&["d:\\FTune\\FTune-mobile\\App.tsx".to_string()]),
        Some("FTune".to_string())
    );
    assert_eq!(
        infer_workspace_folder(&["C:\\Users\\skaf\\Desktop\\MyApp\\src\\App.tsx".to_string()]),
        Some("MyApp".to_string())
    );
    assert_eq!(
        infer_workspace_folder(&[
            "C:\\Users\\skaf\\OneDrive\\Documents\\ClientPortal\\index.ts".to_string()
        ]),
        Some("ClientPortal".to_string())
    );
    assert_eq!(
        infer_workspace_folder(&["D:\\Projects\\BackendApi\\main.rs".to_string()]),
        Some("BackendApi".to_string())
    );
}

#[test]
fn test_new_window_folder_matching_and_closed_session_removal() {
    let t0 = 1_700_002_000;

    // 1. Multi-window enrichment resolves nested folder via open window title and matches
    //    a brand-new unnamed session in a newly opened window.
    let mut sessions = vec![
        AgentSession {
            id: "sess-1111".to_string(),
            title: "Temporary Codebae".to_string(),
            workspace: Some("Temporary Codebae".to_string()),
            status: AgentStatus::Working,
            activity: Some("Reading files".to_string()),
            last_activity: Some(t0),
            last_status_change: Some(t0),
            source: "transcript".to_string(),
            confidence: StatusConfidence::Inferred,
            stale: false,
            disconnected: false,
        },
        AgentSession {
            id: "sess-2222".to_string(),
            title: "Session sess-222".to_string(),
            workspace: None,
            status: AgentStatus::Working,
            activity: Some("Processing user prompt".to_string()),
            last_activity: Some(t0),
            last_status_change: Some(t0),
            source: "transcript".to_string(),
            confidence: StatusConfidence::Inferred,
            stale: false,
            disconnected: false,
        },
    ];
    let windows = vec![
        IdeWindowInfo {
            hwnd_raw: 1,
            pid: 100,
            title: "Antigravity IDE".to_string(),
            workspace_name: None,
        },
        IdeWindowInfo {
            hwnd_raw: 2,
            pid: 100,
            title: "main.py - Core - Antigravity IDE".to_string(),
            workspace_name: Some("Core".to_string()),
        },
        IdeWindowInfo {
            hwnd_raw: 3,
            pid: 200,
            title: "FreshApp - Antigravity IDE".to_string(),
            workspace_name: Some("FreshApp".to_string()),
        },
    ];
    let mut observed_paths = HashMap::new();
    observed_paths.insert(
        "sess-1111".to_string(),
        vec!["f:\\Temporary Codebae\\Core\\backend\\main.py".to_string()],
    );

    enrich_with_windows_and_uia(&mut sessions, &windows, &[], &observed_paths);
    assert_eq!(sessions[0].title, "Core");
    assert_eq!(sessions[0].workspace.as_deref(), Some("Core"));
    assert_eq!(sessions[1].title, "FreshApp");
    assert_eq!(sessions[1].workspace.as_deref(), Some("FreshApp"));

    // 2. When Antigravity is still running and a window/tab is closed (disappears from observed),
    //    SessionStateTracker removes it immediately rather than leaving a disconnected/inactive card.
    let mut tracker = SessionStateTracker::new(180, 120);
    let initial = tracker.reconcile(sessions.clone(), t0, true);
    assert_eq!(initial.len(), 2);

    let after_closing_second_window = tracker.reconcile(vec![sessions[0].clone()], t0 + 2, true);
    assert_eq!(after_closing_second_window.len(), 1);
    assert_eq!(after_closing_second_window[0].id, "sess-1111");
}

#[test]
fn test_transcript_extracts_folder_from_active_document_and_run_command() {
    let root = temp_test_dir("transcript-paths");
    let session_id = "bbbbbbbb-cccc-dddd-eeee-ffffffffffff";
    let logs_dir = root.join("brain").join(session_id).join(".system_generated").join("logs");
    let presence_dir = root.join("presence");
    fs::create_dir_all(&logs_dir).unwrap();
    fs::create_dir_all(&presence_dir).unwrap();

    let lock_path = presence_dir.join(format!("{session_id}.lock"));
    let _held_lock = File::create(&lock_path).unwrap();

    let transcript_path = logs_dir.join("transcript.jsonl");
    fs::write(
        &transcript_path,
        concat!(
            "{\"step_index\":0,\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"status\":\"DONE\",\"created_at\":\"2026-11-16T15:00:00Z\",\"content\":\"Intro line with — em-dash\\nActive Document: C:\\\\Users\\\\skaf\\\\Desktop\\\\AlphaProject\\\\src\\\\main.ts (LANGUAGE_TYPESCRIPT)\"}\n",
            "{\"step_index\":1,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"RUNNING\",\"created_at\":\"2026-11-16T15:00:02Z\",\"tool_calls\":[{\"name\":\"run_command\",\"args\":{\"CommandLine\":\"npm test --prefix C:\\\\Users\\\\skaf\\\\Desktop\\\\AlphaProject\"}}]}\n"
        ),
    )
    .unwrap();

    let mut scanner = TranscriptScanner::default();
    let t_step1 = parse_iso8601_utc_secs("2026-11-16T15:00:02Z").unwrap();
    let sessions = scanner.scan_directory(&root, t_step1 + 1, true).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].title, "AlphaProject");
    assert_eq!(sessions[0].workspace.as_deref(), Some("AlphaProject"));

    drop(_held_lock);
    let _ = fs::remove_dir_all(&root);
}

#[tokio::test]
async fn test_integration_lifecycle_scenarios() {
    let tracker = Mutex::new(SessionStateTracker::new(180, 120));
    let t0 = 1_700_001_000;

    // 1. Antigravity not running
    let p_closed = ScriptedProvider {
        result: Ok(ProviderOutput {
            provider_name: "process",
            antigravity_running: false,
            connection: ConnectionState::AntigravityNotRunning,
            status_message: Some("Antigravity IDE is not running".to_string()),
            sessions: Vec::new(),
        }),
    };
    let snap = collect_with_provider(&p_closed, &tracker, t0, false).await;
    assert_eq!(snap.connection, ConnectionState::AntigravityNotRunning);
    assert!(snap.sessions.is_empty());

    // 2. Running with no active sessions
    let p_empty = ScriptedProvider {
        result: Ok(ProviderOutput {
            provider_name: "process+transcript",
            antigravity_running: true,
            connection: ConnectionState::Connected,
            status_message: None,
            sessions: Vec::new(),
        }),
    };
    let snap = collect_with_provider(&p_empty, &tracker, t0 + 5, false).await;
    assert_eq!(snap.connection, ConnectionState::Connected);
    assert!(snap.antigravity_running);
    assert!(snap.sessions.is_empty());

    // 3. One active session -> Multiple simultaneous sessions (Working, Waiting)
    let s1 = AgentSession {
        id: "s1".to_string(),
        title: "Backend API".to_string(),
        workspace: Some("d:\\api".to_string()),
        status: AgentStatus::Working,
        activity: Some("Writing middleware".to_string()),
        last_activity: Some(t0 + 10),
        last_status_change: Some(t0 + 10),
        source: "hooks".to_string(),
        confidence: StatusConfidence::Confirmed,
        stale: false,
        disconnected: false,
    };
    let s2 = AgentSession {
        id: "s2".to_string(),
        title: "Frontend UI".to_string(),
        workspace: Some("d:\\ui".to_string()),
        status: AgentStatus::Waiting,
        activity: Some("Waiting for permission".to_string()),
        last_activity: Some(t0 + 12),
        last_status_change: Some(t0 + 12),
        source: "uia".to_string(),
        confidence: StatusConfidence::Confirmed,
        stale: false,
        disconnected: false,
    };
    let p_multi = ScriptedProvider {
        result: Ok(ProviderOutput {
            provider_name: "process+transcript+hooks",
            antigravity_running: true,
            connection: ConnectionState::Connected,
            status_message: None,
            sessions: vec![s1.clone(), s2.clone()],
        }),
    };
    let snap = collect_with_provider(&p_multi, &tracker, t0 + 15, true).await;
    assert_eq!(snap.sessions.len(), 2);
    assert_eq!(snap.sessions[0].status, AgentStatus::Working);
    assert_eq!(snap.sessions[1].status, AgentStatus::Waiting);

    // 4. Session completion (s1 finishes)
    let s1_done = AgentSession {
        status: AgentStatus::Completed,
        activity: Some("Turn completed".to_string()),
        last_activity: Some(t0 + 25),
        last_status_change: Some(t0 + 25),
        ..s1.clone()
    };
    let p_done = ScriptedProvider {
        result: Ok(ProviderOutput {
            provider_name: "process+transcript+hooks",
            antigravity_running: true,
            connection: ConnectionState::Connected,
            status_message: None,
            sessions: vec![s1_done, s2.clone()],
        }),
    };
    let snap = collect_with_provider(&p_done, &tracker, t0 + 25, true).await;
    assert_eq!(snap.sessions[0].status, AgentStatus::Waiting); // Sorted above Completed
    assert_eq!(snap.sessions[1].status, AgentStatus::Completed);

    // 5. Monitoring interruption: Antigravity closes -> last known statuses preserved & marked disconnected, NEVER auto-completed.
    let snap = collect_with_provider(&p_closed, &tracker, t0 + 40, true).await;
    assert_eq!(snap.connection, ConnectionState::AntigravityNotRunning);
    assert_eq!(snap.sessions.len(), 2);
    assert!(snap.sessions.iter().all(|s| s.disconnected && s.stale));
    assert!(snap.sessions.iter().any(|s| s.id == "s2" && s.status == AgentStatus::Waiting));

    // 6. Recovery: Antigravity restarts and reports active session again
    let snap = collect_with_provider(&p_multi, &tracker, t0 + 50, true).await;
    assert_eq!(snap.connection, ConnectionState::Connected);
    assert!(snap.sessions.iter().all(|s| !s.disconnected && !s.stale));

    // 7. Provider error handling
    let p_err = ScriptedProvider {
        result: Err(MonitorError::PermissionDenied("Access denied".to_string())),
    };
    let snap = collect_with_provider(&p_err, &tracker, t0 + 55, false).await;
    assert_eq!(snap.connection, ConnectionState::Error);
    assert!(snap.status_message.unwrap().contains("Permission error"));
}

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::model::{AgentSession, AgentStatus, ConnectionState, StatusConfidence};
use crate::providers::hooks::{get_status, hook_events_log_path, HookEventTailer};
use crate::providers::process::{scan_antigravity_processes, IdeWindowInfo};
use crate::providers::transcript::{is_lock_held, TranscriptScanner};
use crate::providers::uia::{inspect_ide_windows, UiaWindowSignal};
use crate::providers::{AgentMonitorProvider, MonitorError, ProviderFuture, ProviderOutput};

/// Combines all local non-invasive Antigravity signal sources:
/// 1. Process & visible window enumeration (`Antigravity IDE.exe` / `agy.exe`)
/// 2. Documented `transcript.jsonl` metadata + `presence/*.lock` probe
/// 3. Bounded Windows UI Automation cues (optional enhancement)
/// 4. Optional opt-in official `hooks.json` event stream
pub struct CompositeAntigravityProvider {
    transcript_scanner: Arc<Mutex<TranscriptScanner>>,
    hook_tailer: Arc<Mutex<HookEventTailer>>,
}

impl Default for CompositeAntigravityProvider {
    fn default() -> Self {
        Self {
            transcript_scanner: Arc::new(Mutex::new(TranscriptScanner::default())),
            hook_tailer: Arc::new(Mutex::new(HookEventTailer::default())),
        }
    }
}

impl AgentMonitorProvider for CompositeAntigravityProvider {
    fn name(&self) -> &'static str {
        "antigravity-composite"
    }

    fn collect(&self, now_secs: u64) -> ProviderFuture<'_, Result<ProviderOutput, MonitorError>> {
        Box::pin(async move {
            // 1. Run Win32 process/window scan + bounded UIA on blocking pool
            let (proc_env, uia_signals) = tokio::task::spawn_blocking(|| {
                let proc_env = scan_antigravity_processes()?;
                let uia = if !proc_env.windows.is_empty() {
                    inspect_ide_windows(&proc_env.windows)
                } else {
                    Vec::new()
                };
                Ok::<_, String>((proc_env, uia))
            })
            .await
            .map_err(|e| MonitorError::Unavailable(format!("Process scan task failed: {e}")))?
            .map_err(MonitorError::Unavailable)?;

            let running = proc_env.antigravity_running();

            // 2. Scan documented transcript metadata + presence locks
            let mut transcript_sessions = Vec::new();
            let mut observed_paths_by_id: HashMap<String, Vec<String>> = HashMap::new();
            let mut degraded_reason: Option<String> = None;

            let antigravity_root = TranscriptScanner::antigravity_root();
            if let Some(ref root) = antigravity_root {
                let mut scanner = self.transcript_scanner.lock().await;
                match scanner.scan_directory(root, now_secs, running) {
                    Ok(list) => {
                        for s in &list {
                            let paths = scanner.observed_paths(&s.id);
                            if !paths.is_empty() {
                                observed_paths_by_id.insert(s.id.clone(), paths.to_vec());
                            }
                        }
                        transcript_sessions = list;
                    }
                    Err(err) => {
                        degraded_reason = Some(err);
                    }
                }
            }

            // 3. Read opt-in official hook events (dropping sessions whose presence lock was released)
            let hook_status = get_status();
            let mut hook_sessions = if hook_status.enabled {
                let mut tailer = self.hook_tailer.lock().await;
                tailer.read_sessions(&hook_events_log_path(), now_secs, running)
            } else {
                Vec::new()
            };
            if running {
                if let Some(ref root) = antigravity_root {
                    let presence_dir = root.join("presence");
                    hook_sessions.retain(|s| {
                        let lock_path = presence_dir.join(format!("{}.lock", s.id));
                        !lock_path.exists() || is_lock_held(&lock_path)
                    });
                }
            }

            // 4. Combine sessions + apply window title matching & UIA cues where applicable
            let mut combined = Vec::with_capacity(transcript_sessions.len() + hook_sessions.len());
            combined.extend(transcript_sessions);
            combined.extend(hook_sessions);

            enrich_with_windows_and_uia(
                &mut combined,
                &proc_env.windows,
                &uia_signals,
                &observed_paths_by_id,
            );

            let connection = if !running {
                ConnectionState::AntigravityNotRunning
            } else if degraded_reason.is_some() {
                ConnectionState::Degraded
            } else {
                ConnectionState::Connected
            };

            let status_message = if !running {
                Some("Antigravity IDE is not running".to_string())
            } else if let Some(err) = degraded_reason {
                Some(err)
            } else if combined.is_empty() {
                Some("IDE detected; waiting for active conversation metadata".to_string())
            } else {
                None
            };

            Ok(ProviderOutput {
                provider_name: if hook_status.enabled {
                    "process+transcript+hooks"
                } else {
                    "process+transcript"
                },
                antigravity_running: running,
                connection,
                status_message,
                sessions: combined,
            })
        })
    }
}

pub fn enrich_with_windows_and_uia(
    sessions: &mut [AgentSession],
    windows: &[IdeWindowInfo],
    uia_signals: &[UiaWindowSignal],
    observed_paths: &HashMap<String, Vec<String>>,
) {
    let mut unique_window_names: Vec<String> = Vec::new();
    for w in windows {
        if let Some(ws) = w.workspace_name.as_ref() {
            if !unique_window_names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(ws))
            {
                unique_window_names.push(ws.clone());
            }
        }
    }

    // 1. Match each session's observed file paths against open IDE window folder names
    //    so nested folders (e.g. `F:\Parent\Project\src\main.rs` with window `Project`) resolve accurately.
    for s in sessions.iter_mut() {
        if let Some(paths) = observed_paths.get(&s.id) {
            if let Some(matched_ws) = match_window_from_paths(paths, &unique_window_names) {
                let already_matches = s
                    .workspace
                    .as_deref()
                    .and_then(|w| std::path::Path::new(w).file_name().and_then(|n| n.to_str()))
                    .is_some_and(|name| name.eq_ignore_ascii_case(&matched_ws));
                if !already_matches {
                    s.workspace = Some(matched_ws.clone());
                }
                if s.title.starts_with("Session ") || !s.title.eq_ignore_ascii_case(&matched_ws) {
                    s.title = matched_ws;
                }
            }
        }
    }

    // 2. If only one workspace window is open, attach it to sessions lacking workspace names.
    if unique_window_names.len() == 1 {
        let ws = &unique_window_names[0];
        for s in sessions.iter_mut() {
            if s.workspace.is_none() {
                s.workspace = Some(ws.clone());
            }
            if s.title.starts_with("Session ") {
                s.title = ws.clone();
            }
        }
    } else if unique_window_names.len() > 1 {
        // 3. When a new window opens alongside existing windows and has a brand-new session
        //    without observed paths yet, match the single unclaimed window to the single unnamed session.
        let claimed: Vec<String> = sessions
            .iter()
            .filter_map(|s| {
                s.workspace
                    .as_deref()
                    .and_then(|w| {
                        std::path::Path::new(w)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .or(Some(w))
                    })
                    .map(|n| n.to_ascii_lowercase())
            })
            .collect();

        let unmatched: Vec<&String> = unique_window_names
            .iter()
            .filter(|ws| !claimed.iter().any(|c| c == &ws.to_ascii_lowercase()))
            .collect();

        let unnamed_indices: Vec<usize> = sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| s.workspace.is_none() || s.title.starts_with("Session "))
            .map(|(i, _)| i)
            .collect();

        if unmatched.len() == 1 && unnamed_indices.len() == 1 {
            let ws = unmatched[0].clone();
            let s = &mut sessions[unnamed_indices[0]];
            if s.workspace.is_none() {
                s.workspace = Some(ws.clone());
            }
            if s.title.starts_with("Session ") {
                s.title = ws;
            }
        }
    }

    // Apply UIA approval prompt cue when a matching workspace (or single window) shows an active prompt.
    for sig in uia_signals {
        if !sig.approval_prompt_visible {
            continue;
        }
        for s in sessions.iter_mut() {
            let workspace_matches = match (&sig.workspace_name, &s.workspace) {
                (Some(w), Some(sw)) => sw.to_ascii_lowercase().contains(&w.to_ascii_lowercase()),
                _ => uia_signals.len() == 1,
            };
            if workspace_matches && matches!(s.status, AgentStatus::Working | AgentStatus::Idle) {
                s.status = AgentStatus::Waiting;
                s.confidence = StatusConfidence::Confirmed;
                s.activity = sig
                    .detected_cue
                    .clone()
                    .or_else(|| Some("Waiting for approval in Antigravity IDE".to_string()));
            }
        }
    }
}

fn match_window_from_paths(paths: &[String], window_names: &[String]) -> Option<String> {
    if window_names.is_empty() || paths.is_empty() {
        return None;
    }
    let mut scores: HashMap<String, (usize, usize)> = HashMap::new();
    for raw in paths {
        let normalized = raw.replace('/', "\\");
        let segments: Vec<&str> = normalized
            .split('\\')
            .filter(|s| !s.is_empty() && !s.ends_with(':'))
            .collect();
        for (depth, seg) in segments.iter().enumerate() {
            for win in window_names {
                if seg.eq_ignore_ascii_case(win) {
                    let entry = scores.entry(win.clone()).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 = entry.1.max(depth);
                }
            }
        }
    }
    scores
        .into_iter()
        .max_by_key(|(_, (count, depth))| (*count, *depth))
        .map(|(name, _)| name)
}

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::config::settings_dir;
use crate::domain::classifier::humanize_tool_name;
use crate::domain::model::{
    AgentSession, AgentStatus, HookIntegrationStatus, StatusConfidence,
};
use crate::providers::now_unix_secs;
use crate::providers::transcript::{resolve_workspace_from_disk, short_session_title};

const HOOK_KEY_NAME: &str = "antigravity-agent-monitor";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordedHookEvent {
    pub conversation_id: String,
    pub hook_event_name: String,
    pub workspace_path: Option<String>,
    pub model_name: Option<String>,
    pub tool_name: Option<String>,
    pub termination_reason: Option<String>,
    pub fully_idle: Option<bool>,
    pub has_error: bool,
    pub timestamp: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StdinToolCall {
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StdinHookPayload {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    hook_event_name: String,
    #[serde(default)]
    workspace_paths: Vec<String>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    model_name: Option<String>,
    #[serde(default)]
    tool_call: Option<StdinToolCall>,
    #[serde(default)]
    termination_reason: Option<String>,
    #[serde(default)]
    fully_idle: Option<bool>,
    #[serde(default)]
    error: Option<String>,
}

pub fn hooks_config_path() -> Option<PathBuf> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))?;
    Some(PathBuf::from(home).join(".gemini").join("config").join("hooks.json"))
}

pub fn hook_events_log_path() -> PathBuf {
    settings_dir().join("hook-events.jsonl")
}

/// Fast-path entry point invoked when `Agent Monitor.exe --hook-ingest` is triggered by Antigravity.
/// Reads stdin, appends only safe metadata (never user prompts or tool args), and prints `{}`.
pub fn ingest_stdin_and_exit() {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    if let Ok(payload) = serde_json::from_str::<StdinHookPayload>(&input) {
        if !payload.conversation_id.is_empty() && !payload.hook_event_name.is_empty() {
            let workspace = payload
                .workspace_paths
                .into_iter()
                .next()
                .or(payload.cwd);
            let event = RecordedHookEvent {
                conversation_id: payload.conversation_id,
                hook_event_name: payload.hook_event_name,
                workspace_path: workspace,
                model_name: payload.model_name,
                tool_name: payload.tool_call.map(|t| t.name).filter(|s| !s.is_empty()),
                termination_reason: payload.termination_reason,
                fully_idle: payload.fully_idle,
                has_error: payload.error.as_ref().is_some_and(|e| !e.trim().is_empty()),
                timestamp: now_unix_secs(),
            };
            let _ = append_hook_event(&hook_events_log_path(), &event);
        }
    }

    // Required contract response for PostToolUse / PreInvocation / PostInvocation / SessionStart / Stop.
    let _ = std::io::stdout().write_all(b"{}");
    let _ = std::io::stdout().flush();
}

pub fn append_hook_event(log_path: &Path, event: &RecordedHookEvent) -> Result<(), String> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed creating hook log dir: {e}"))?;
    }
    let line = serde_json::to_string(event).map_err(|e| format!("Serialize hook event: {e}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Open hook log: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("Write hook event: {e}"))?;
    Ok(())
}

pub fn get_status() -> HookIntegrationStatus {
    let cfg_path = hooks_config_path();
    let enabled = cfg_path
        .as_ref()
        .is_some_and(|p| is_hook_registered(p));
    let log_path = hook_events_log_path();
    let last_event_at = fs::metadata(&log_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    HookIntegrationStatus {
        enabled,
        config_path: cfg_path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        event_log_path: log_path.to_string_lossy().into_owned(),
        last_event_at,
    }
}

pub fn enable_hooks(exe_path: &Path) -> Result<HookIntegrationStatus, String> {
    let cfg_path = hooks_config_path()
        .ok_or_else(|| "Could not determine user home directory for ~/.gemini/config/hooks.json".to_string())?;
    enable_hooks_at(&cfg_path, exe_path)?;
    Ok(get_status())
}

pub fn disable_hooks() -> Result<HookIntegrationStatus, String> {
    let cfg_path = hooks_config_path()
        .ok_or_else(|| "Could not determine user home directory for ~/.gemini/config/hooks.json".to_string())?;
    disable_hooks_at(&cfg_path)?;
    Ok(get_status())
}

pub fn is_hook_registered(cfg_path: &Path) -> bool {
    let Ok(raw) = fs::read_to_string(cfg_path) else {
        return false;
    };
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    map.get(HOOK_KEY_NAME)
        .and_then(|v| v.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| map.contains_key(HOOK_KEY_NAME))
}

pub fn enable_hooks_at(cfg_path: &Path, exe_path: &Path) -> Result<(), String> {
    if let Some(parent) = cfg_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }

    let mut root_map = read_hooks_object(cfg_path)?;
    let cmd = format!("\"{}\" --hook-ingest", exe_path.to_string_lossy());
    let flat_handler = json!([{
        "type": "command",
        "command": cmd,
        "timeout": 5
    }]);
    let tool_handler = json!([{
        "matcher": "*",
        "hooks": [{
            "type": "command",
            "command": cmd,
            "timeout": 5
        }]
    }]);

    // Register ONLY non-gating observation hooks (never PreToolUse so permission prompts are untouched).
    root_map.insert(
        HOOK_KEY_NAME.to_string(),
        json!({
            "enabled": true,
            "SessionStart": flat_handler,
            "PreInvocation": flat_handler,
            "PostToolUse": tool_handler,
            "PostInvocation": flat_handler,
            "Stop": flat_handler
        }),
    );

    write_hooks_object(cfg_path, &root_map)
}

pub fn disable_hooks_at(cfg_path: &Path) -> Result<(), String> {
    if !cfg_path.exists() {
        return Ok(());
    }
    let mut root_map = read_hooks_object(cfg_path)?;
    root_map.remove(HOOK_KEY_NAME);
    write_hooks_object(cfg_path, &root_map)
}

fn read_hooks_object(cfg_path: &Path) -> Result<Map<String, Value>, String> {
    if !cfg_path.exists() {
        return Ok(Map::new());
    }
    let raw = fs::read_to_string(cfg_path)
        .map_err(|e| format!("Failed to read {}: {e}", cfg_path.display()))?;
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err("Existing hooks.json root must be a JSON object".to_string()),
        Err(e) => Err(format!("Existing hooks.json contains invalid JSON: {e}")),
    }
}

fn write_hooks_object(cfg_path: &Path, map: &Map<String, Value>) -> Result<(), String> {
    let serialized = serde_json::to_string_pretty(&Value::Object(map.clone()))
        .map_err(|e| format!("Serialize hooks.json: {e}"))?;
    let tmp = cfg_path.with_extension("json.tmp");
    fs::write(&tmp, serialized).map_err(|e| format!("Write temp hooks.json: {e}"))?;
    fs::rename(&tmp, cfg_path).map_err(|e| format!("Commit hooks.json: {e}"))?;
    Ok(())
}

#[derive(Debug, Default)]
pub struct HookEventTailer {
    offset: u64,
    latest_by_session: HashMap<String, RecordedHookEvent>,
}

impl HookEventTailer {
    pub fn read_sessions(
        &mut self,
        log_path: &Path,
        now_secs: u64,
        antigravity_running: bool,
    ) -> Vec<AgentSession> {
        if !log_path.is_file() {
            return Vec::new();
        }

        if let Ok(meta) = fs::metadata(log_path) {
            let len = meta.len();
            if len < self.offset {
                self.offset = 0;
                self.latest_by_session.clear();
            }
            if len > self.offset {
                if let Ok(mut file) = File::open(log_path) {
                    if file.seek(SeekFrom::Start(self.offset)).is_ok() {
                        let mut reader = BufReader::new(file);
                        let mut line = String::new();
                        let mut new_offset = self.offset;
                        while let Ok(n) = reader.read_line(&mut line) {
                            if n == 0 || !line.ends_with('\n') {
                                break;
                            }
                            new_offset += n as u64;
                            if let Ok(evt) =
                                serde_json::from_str::<RecordedHookEvent>(line.trim())
                            {
                                self.latest_by_session
                                    .insert(evt.conversation_id.clone(), evt);
                            }
                            line.clear();
                        }
                        self.offset = new_offset;
                    }
                }
            }
        }

        let mut out = Vec::new();
        for (id, evt) in &self.latest_by_session {
            // Drop very old hook records (> 4 hours) so stale history doesn't accumulate.
            if now_secs.saturating_sub(evt.timestamp) > 14400 {
                continue;
            }
            let (status, activity) = classify_hook_event(evt, antigravity_running);
            let title = evt
                .workspace_path
                .as_deref()
                .and_then(|p| {
                    resolve_workspace_from_disk(&[p.to_string()]).or_else(|| {
                        Path::new(p)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .filter(|n| !n.is_empty())
                            .map(str::to_string)
                    })
                })
                .unwrap_or_else(|| short_session_title(id));

            out.push(AgentSession {
                id: id.clone(),
                title,
                workspace: evt.workspace_path.clone(),
                status,
                activity: Some(activity),
                last_activity: Some(evt.timestamp),
                last_status_change: Some(evt.timestamp),
                source: "hooks".to_string(),
                confidence: if antigravity_running {
                    StatusConfidence::Confirmed
                } else {
                    StatusConfidence::Inferred
                },
                stale: !antigravity_running,
                disconnected: !antigravity_running,
            });
        }
        out
    }
}

pub fn classify_hook_event(
    evt: &RecordedHookEvent,
    antigravity_running: bool,
) -> (AgentStatus, String) {
    if !antigravity_running {
        return (
            AgentStatus::Unknown,
            "Antigravity disconnected".to_string(),
        );
    }
    match evt.hook_event_name.as_str() {
        "SessionStart" => (AgentStatus::Idle, "Session started".to_string()),
        "PreInvocation" => (AgentStatus::Working, "Generating model response".to_string()),
        "PostToolUse" => {
            if evt.has_error {
                let tool = evt
                    .tool_name
                    .as_deref()
                    .map(humanize_tool_name)
                    .unwrap_or_else(|| "tool".to_string());
                (AgentStatus::Error, format!("Tool error in {tool}"))
            } else {
                let tool = evt
                    .tool_name
                    .as_deref()
                    .map(humanize_tool_name)
                    .unwrap_or_else(|| "tool".to_string());
                (AgentStatus::Working, format!("Finished {tool}"))
            }
        }
        "PostInvocation" => (AgentStatus::Working, "Processing tool results".to_string()),
        "Stop" => {
            let reason = evt.termination_reason.as_deref().unwrap_or("");
            if evt.has_error || reason == "ERROR" {
                (AgentStatus::Error, "Execution terminated with error".to_string())
            } else if reason == "USER_CANCELED" {
                (AgentStatus::Idle, "Stopped by user".to_string())
            } else if reason == "NO_TOOL_CALL" && evt.fully_idle.unwrap_or(true) {
                (AgentStatus::Completed, "Turn completed".to_string())
            } else {
                (AgentStatus::Idle, format!("Stopped ({reason})"))
            }
        }
        _ => (AgentStatus::Unknown, "Observed hook event".to_string()),
    }
}

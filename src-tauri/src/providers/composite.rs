use std::sync::Arc;
use tokio::sync::Mutex;

use crate::domain::model::{AgentSession, AgentStatus, ConnectionState, StatusConfidence};
use crate::providers::hooks::{get_status, hook_events_log_path, HookEventTailer};
use crate::providers::process::{scan_antigravity_processes, IdeWindowInfo};
use crate::providers::transcript::TranscriptScanner;
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
            let mut degraded_reason: Option<String> = None;

            if let Some(root) = TranscriptScanner::antigravity_root() {
                let mut scanner = self.transcript_scanner.lock().await;
                match scanner.scan_directory(&root, now_secs, running) {
                    Ok(list) => transcript_sessions = list,
                    Err(err) => {
                        degraded_reason = Some(err);
                    }
                }
            }

            // 3. Read opt-in official hook events
            let hook_status = get_status();
            let hook_sessions = if hook_status.enabled {
                let mut tailer = self.hook_tailer.lock().await;
                tailer.read_sessions(&hook_events_log_path(), now_secs, running)
            } else {
                Vec::new()
            };

            // 4. Combine sessions + apply UIA cues where applicable
            let mut combined = Vec::with_capacity(transcript_sessions.len() + hook_sessions.len());
            combined.extend(transcript_sessions);
            combined.extend(hook_sessions);

            enrich_with_windows_and_uia(&mut combined, &proc_env.windows, &uia_signals);

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

fn enrich_with_windows_and_uia(
    sessions: &mut [AgentSession],
    windows: &[IdeWindowInfo],
    uia_signals: &[UiaWindowSignal],
) {
    // If a single IDE window is open and sessions lack workspace names, attach its workspace tag.
    if windows.len() == 1 {
        if let Some(ws) = windows[0].workspace_name.as_ref() {
            for s in sessions.iter_mut() {
                if s.workspace.is_none() {
                    s.workspace = Some(ws.clone());
                }
                if s.title.starts_with("Session ") {
                    s.title = ws.clone();
                }
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

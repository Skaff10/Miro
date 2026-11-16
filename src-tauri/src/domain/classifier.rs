use std::collections::HashMap;

use super::model::{AgentSession, AgentStatus, StatusConfidence};

#[derive(Debug, Clone)]
struct TrackedEntry {
    session: AgentSession,
    last_seen_at: u64,
}

/// Maintains continuity across polling cycles:
/// - Tracks `last_status_change` accurately when status transitions.
/// - Deduplicates sessions by ID, preferring higher-confidence signals.
/// - Preserves last known status on disconnection/staleness without fabricating `Completed`.
#[derive(Debug, Default)]
pub struct SessionStateTracker {
    entries: HashMap<String, TrackedEntry>,
    stale_after_secs: u64,
    retain_disconnected_secs: u64,
}

impl SessionStateTracker {
    pub fn new(stale_after_secs: u64, retain_disconnected_secs: u64) -> Self {
        Self {
            entries: HashMap::new(),
            stale_after_secs,
            retain_disconnected_secs,
        }
    }

    /// Reconciles freshly observed sessions against prior state and returns
    /// a deterministically sorted, deduplicated list.
    pub fn reconcile(
        &mut self,
        observed: Vec<AgentSession>,
        now_secs: u64,
        antigravity_running: bool,
    ) -> Vec<AgentSession> {
        // 1. Deduplicate observed sessions by ID, merging highest confidence.
        let mut merged_observed: HashMap<String, AgentSession> = HashMap::new();
        for item in observed {
            match merged_observed.get_mut(&item.id) {
                Some(existing) => merge_session(existing, item),
                None => {
                    merged_observed.insert(item.id.clone(), item);
                }
            }
        }

        // 2. Update tracked entries for currently observed sessions.
        for (id, mut incoming) in merged_observed {
            if let Some(prev) = self.entries.get(&id) {
                if prev.session.status == incoming.status {
                    // Keep the original transition timestamp when status hasn't changed.
                    incoming.last_status_change = prev
                        .session
                        .last_status_change
                        .or(incoming.last_status_change)
                        .or(incoming.last_activity)
                        .or(Some(now_secs));
                } else {
                    incoming.last_status_change = incoming.last_status_change.or(Some(now_secs));
                }
                if incoming.activity.is_none() && !incoming.disconnected {
                    incoming.activity = prev.session.activity.clone();
                }
            } else {
                incoming.last_status_change = incoming
                    .last_status_change
                    .or(incoming.last_activity)
                    .or(Some(now_secs));
            }

            // Mark stale if a Working session has had no activity past the stale threshold.
            if incoming.status == AgentStatus::Working {
                if let Some(last_act) = incoming.last_activity {
                    if now_secs.saturating_sub(last_act) > self.stale_after_secs {
                        incoming.stale = true;
                        if incoming.confidence == StatusConfidence::Confirmed {
                            incoming.confidence = StatusConfidence::Inferred;
                        }
                    }
                }
            }

            self.entries.insert(
                id,
                TrackedEntry {
                    session: incoming,
                    last_seen_at: now_secs,
                },
            );
        }

        // 3. Handle previously known sessions missing from this observation cycle.
        let mut to_remove = Vec::new();
        for (id, entry) in self.entries.iter_mut() {
            if entry.last_seen_at < now_secs {
                let age = now_secs.saturating_sub(entry.last_seen_at);
                if age > self.retain_disconnected_secs {
                    to_remove.push(id.clone());
                    continue;
                }

                // Preserve last known status; never auto-promote to Completed on disconnect.
                entry.session.disconnected = true;
                entry.session.stale = true;
                if !antigravity_running {
                    entry.session.activity = Some("Antigravity disconnected".to_string());
                } else if entry.session.activity.is_none() {
                    entry.session.activity = Some("Session no longer active".to_string());
                }
            }
        }
        for id in to_remove {
            self.entries.remove(&id);
        }

        let mut list: Vec<AgentSession> = self
            .entries
            .values()
            .map(|e| e.session.clone())
            .collect();

        list.sort_by(|a, b| {
            a.disconnected
                .cmp(&b.disconnected)
                .then_with(|| a.status.sort_rank().cmp(&b.status.sort_rank()))
                .then_with(|| b.last_activity.unwrap_or(0).cmp(&a.last_activity.unwrap_or(0)))
                .then_with(|| a.title.cmp(&b.title))
        });

        list
    }
}

fn merge_session(target: &mut AgentSession, incoming: AgentSession) {
    let take_incoming_status = incoming.confidence > target.confidence
        || (incoming.confidence == target.confidence
            && incoming.last_activity.unwrap_or(0) >= target.last_activity.unwrap_or(0));

    if take_incoming_status {
        let status_changed = target.status != incoming.status;
        target.status = incoming.status;
        target.confidence = incoming.confidence;
        target.source = incoming.source;
        target.stale = incoming.stale;
        target.disconnected = incoming.disconnected;
        if status_changed {
            target.last_status_change = incoming.last_status_change.or(target.last_status_change);
        } else if target.last_status_change.is_none() {
            target.last_status_change = incoming.last_status_change;
        }
        if incoming.activity.is_some() {
            target.activity = incoming.activity;
        }
    } else if target.activity.is_none() && incoming.activity.is_some() {
        target.activity = incoming.activity;
    }

    if !incoming.title.starts_with("Session ")
        && (target.title.starts_with("Session ") || incoming.title.len() > target.title.len())
    {
        target.title = incoming.title;
    }
    if target.workspace.is_none() {
        target.workspace = incoming.workspace;
    }
    match (target.last_activity, incoming.last_activity) {
        (Some(a), Some(b)) => target.last_activity = Some(a.max(b)),
        (None, Some(b)) => target.last_activity = Some(b),
        _ => {}
    }
}

/// Classifies a transcript step into an honest `AgentStatus` + `StatusConfidence`.
///
/// Rules:
/// - `RUNNING` step -> `Working` (`Inferred` unless hook confirms)
/// - `ERROR` / error string -> `Error`
/// - `PLANNER_RESPONSE` with `DONE` + pending tool calls -> `Working` or `Waiting` depending on age
/// - `PLANNER_RESPONSE` with `DONE` + no tool calls -> `Completed` if recent, `Idle` if aged
/// - Anything unrecognized -> `Unknown`
pub fn classify_transcript_step(
    step_status: &str,
    step_source: &str,
    step_type: &str,
    has_tool_calls: bool,
    tool_names: &[String],
    age_secs: u64,
    session_locked: bool,
) -> (AgentStatus, StatusConfidence, Option<String>) {
    let status_upper = step_status.trim().to_ascii_uppercase();
    let type_upper = step_type.trim().to_ascii_uppercase();
    let source_upper = step_source.trim().to_ascii_uppercase();

    if status_upper == "ERROR" || status_upper == "FAILED" {
        return (
            AgentStatus::Error,
            StatusConfidence::Inferred,
            Some("Last step reported an error".to_string()),
        );
    }

    if !session_locked {
        // Session has no active OS presence lock: preserve calm Idle/Unknown rather than
        // claiming it is actively Working or Waiting.
        if status_upper == "DONE" && source_upper == "MODEL" && !has_tool_calls {
            return (
                AgentStatus::Idle,
                StatusConfidence::Inferred,
                Some("Session inactive (closed tab)".to_string()),
            );
        }
        return (
            AgentStatus::Unknown,
            StatusConfidence::Unknown,
            Some("Session lock not held".to_string()),
        );
    }

    if status_upper == "RUNNING" || status_upper == "IN_PROGRESS" {
        let activity = if let Some(tool) = tool_names.last() {
            Some(format!("Running {}", humanize_tool_name(tool)))
        } else {
            Some("Generating response".to_string())
        };
        return (AgentStatus::Working, StatusConfidence::Inferred, activity);
    }

    if status_upper == "DONE" {
        if has_tool_calls {
            // Tool calls were emitted and completed/in-flight before the final model turn.
            let last_tool = tool_names.last().map(|s| s.as_str()).unwrap_or("tool");
            if age_secs <= 15 {
                return (
                    AgentStatus::Working,
                    StatusConfidence::Inferred,
                    Some(format!("Executing {}", humanize_tool_name(last_tool))),
                );
            }
            if age_secs <= 180 && is_interactive_tool(last_tool) {
                return (
                    AgentStatus::Waiting,
                    StatusConfidence::Inferred,
                    Some(format!("May be waiting on {}", humanize_tool_name(last_tool))),
                );
            }
            return (
                AgentStatus::Idle,
                StatusConfidence::Inferred,
                Some(format!("Paused after {}", humanize_tool_name(last_tool))),
            );
        }

        if source_upper == "USER_EXPLICIT" || type_upper == "USER_INPUT" {
            if age_secs <= 15 {
                return (
                    AgentStatus::Working,
                    StatusConfidence::Inferred,
                    Some("Processing user prompt".to_string()),
                );
            }
            return (
                AgentStatus::Idle,
                StatusConfidence::Inferred,
                Some("Awaiting model response".to_string()),
            );
        }

        if source_upper == "MODEL" && type_upper == "PLANNER_RESPONSE" {
            // Model finished a turn without emitting tool calls.
            if age_secs <= 900 {
                return (
                    AgentStatus::Completed,
                    StatusConfidence::Inferred,
                    Some("Turn finished".to_string()),
                );
            }
            return (
                AgentStatus::Idle,
                StatusConfidence::Inferred,
                Some("No active turn".to_string()),
            );
        }

        if source_upper == "MODEL" && type_upper == "GENERIC" && age_secs <= 15 {
            return (
                AgentStatus::Working,
                StatusConfidence::Inferred,
                Some("Executing agent step".to_string()),
            );
        }

        return (
            AgentStatus::Idle,
            StatusConfidence::Inferred,
            Some("Session open".to_string()),
        );
    }

    (AgentStatus::Unknown, StatusConfidence::Unknown, None)
}

fn is_interactive_tool(name: &str) -> bool {
    matches!(
        name,
        "run_command" | "ask_question" | "send_command_input" | "write_file" | "edit_file"
    )
}

pub fn humanize_tool_name(name: &str) -> String {
    match name {
        "run_command" => "terminal command".to_string(),
        "read_file" => "file read".to_string(),
        "write_file" => "file write".to_string(),
        "edit_file" => "file edit".to_string(),
        "search_web" => "web search".to_string(),
        "ask_question" => "user prompt".to_string(),
        other => other.replace('_', " "),
    }
}

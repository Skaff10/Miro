use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::domain::classifier::classify_transcript_step;
use crate::domain::model::{AgentSession, AgentStatus, StatusConfidence};

#[derive(Debug, Clone)]
pub struct CachedTranscriptState {
    pub offset: u64,
    pub last_step: Option<TranscriptStepMeta>,
    pub observed_paths: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TranscriptStepMeta {
    pub step_index: u64,
    pub source: String,
    pub step_type: String,
    pub status: String,
    pub created_at_secs: Option<u64>,
    pub tool_names: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawToolCallMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    args: Option<serde_json::Value>,
}

/// Strictly reads structural metadata fields; ignores prompt/model text, thinking, args, and media.
#[derive(Debug, Deserialize)]
struct RawTranscriptMeta {
    #[serde(default)]
    step_index: u64,
    #[serde(default)]
    source: String,
    #[serde(rename = "type", default)]
    step_type: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    tool_calls: Vec<RawToolCallMeta>,
}

#[derive(Debug, Default)]
pub struct TranscriptScanner {
    cache: HashMap<String, CachedTranscriptState>,
}

impl TranscriptScanner {
    pub fn antigravity_root() -> Option<PathBuf> {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))?;
        Some(PathBuf::from(home).join(".gemini").join("antigravity"))
    }

    pub fn scan_directory(
        &mut self,
        root: &Path,
        now_secs: u64,
        antigravity_running: bool,
    ) -> Result<Vec<AgentSession>, String> {
        let brain_dir = root.join("brain");
        let presence_dir = root.join("presence");

        if !brain_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&brain_dir)
            .map_err(|e| format!("Failed to read Antigravity brain directory: {e}"))?;

        let mut sessions = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path.file_name().and_then(|n| n.to_str()).map(str::to_string) else {
                continue;
            };
            // Skip non-session utility folders like tempmediaStorage.
            if !looks_like_session_id(&id) {
                continue;
            }

            let transcript_path = path
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if !transcript_path.is_file() {
                continue;
            }

            let lock_path = presence_dir.join(format!("{id}.lock"));
            let locked = is_lock_held(&lock_path);

            let mtime_secs = fs::metadata(&transcript_path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            // Only include sessions that are currently locked by an active process, OR
            // had recent transcript activity within the last 30 minutes while IDE is running.
            let is_recent = mtime_secs
                .map(|m| now_secs.saturating_sub(m) <= 1800)
                .unwrap_or(false);
            if !locked && !(antigravity_running && is_recent) {
                continue;
            }

            self.tail_transcript(&id, &transcript_path)?;

            if let Some(cached) = self.cache.get(&id) {
                if let Some(meta) = cached.last_step.as_ref() {
                    let last_act = meta.created_at_secs.or(mtime_secs);
                    let age_secs = last_act.map(|t| now_secs.saturating_sub(t)).unwrap_or(0);
                    let (status, confidence, activity) = classify_transcript_step(
                        &meta.status,
                        &meta.source,
                        &meta.step_type,
                        !meta.tool_names.is_empty(),
                        &meta.tool_names,
                        age_secs,
                        locked,
                    );

                    // If Antigravity stopped running while a step still said RUNNING, do not claim Working.
                    let (final_status, final_conf, final_activity, stale, disconnected) =
                        if !antigravity_running && status == AgentStatus::Working {
                            (
                                AgentStatus::Unknown,
                                StatusConfidence::Unknown,
                                Some("Process ended before step completed".to_string()),
                                true,
                                true,
                            )
                        } else {
                            (status, confidence, activity, false, false)
                        };

                    let inferred_workspace = infer_workspace_folder(&cached.observed_paths);
                    let title = inferred_workspace
                        .clone()
                        .unwrap_or_else(|| short_session_title(&id));

                    sessions.push(AgentSession {
                        id: id.clone(),
                        title,
                        workspace: inferred_workspace,
                        status: final_status,
                        activity: final_activity,
                        last_activity: last_act,
                        last_status_change: last_act,
                        source: "transcript".to_string(),
                        confidence: final_conf,
                        stale,
                        disconnected,
                    });
                }
            }
        }

        Ok(sessions)
    }

    fn tail_transcript(&mut self, id: &str, path: &Path) -> Result<(), String> {
        let meta = fs::metadata(path)
            .map_err(|e| format!("Failed to stat {}: {e}", path.display()))?;
        let file_len = meta.len();

        let entry = self.cache.entry(id.to_string()).or_insert(CachedTranscriptState {
            offset: 0,
            last_step: None,
            observed_paths: Vec::new(),
        });

        if file_len < entry.offset {
            // File was truncated or rotated; reset offset.
            entry.offset = 0;
            entry.last_step = None;
            entry.observed_paths.clear();
        }
        if file_len == entry.offset {
            return Ok(());
        }

        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
        file.seek(SeekFrom::Start(entry.offset))
            .map_err(|e| format!("Failed to seek {}: {e}", path.display()))?;

        let mut reader = BufReader::new(file);
        let mut buf = String::new();
        let mut bytes_read_total = entry.offset;

        loop {
            buf.clear();
            let n = reader
                .read_line(&mut buf)
                .map_err(|e| format!("Failed reading {}: {e}", path.display()))?;
            if n == 0 {
                break;
            }
            // Wait for complete newline-terminated line so partial writes aren't misparsed.
            if !buf.ends_with('\n') {
                break;
            }
            bytes_read_total += n as u64;
            let trimmed = buf.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<RawTranscriptMeta>(trimmed) {
                let mut tool_names = Vec::new();
                for tc in parsed.tool_calls {
                    if !tc.name.is_empty() {
                        tool_names.push(tc.name);
                    }
                    if let Some(serde_json::Value::Object(map)) = tc.args {
                        for key in [
                            "Path",
                            "TargetFile",
                            "DirectoryPath",
                            "SearchPath",
                            "Cwd",
                            "ScriptPath",
                        ] {
                            if let Some(serde_json::Value::String(p)) = map.get(key) {
                                push_observed_path(&mut entry.observed_paths, p);
                            }
                        }
                    }
                }
                entry.last_step = Some(TranscriptStepMeta {
                    step_index: parsed.step_index,
                    source: parsed.source,
                    step_type: parsed.step_type,
                    status: parsed.status,
                    created_at_secs: parse_iso8601_utc_secs(&parsed.created_at),
                    tool_names,
                });
            }
        }

        entry.offset = bytes_read_total;
        Ok(())
    }
}

fn push_observed_path(list: &mut Vec<String>, raw_path: &str) {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return;
    }
    // Skip Antigravity internal .gemini/AppData/Program Files paths so project folders win.
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("\\.gemini\\")
        || lower.contains("/.gemini/")
        || lower.contains("\\appdata\\")
        || lower.contains("\\program files")
    {
        return;
    }
    if !list.iter().any(|existing| existing == trimmed) {
        if list.len() >= 24 {
            list.remove(0);
        }
        list.push(trimmed.to_string());
    }
}

/// Extracts the most likely project folder name from observed tool-call file paths
/// (e.g. `d:\MIRO\package.json` -> `MIRO`, `d:\FTune\FTune-mobile\App.tsx` -> `FTune`).
pub fn infer_workspace_folder(paths: &[String]) -> Option<String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in paths {
        let normalized = p.replace('/', "\\");
        let segments: Vec<&str> = normalized
            .split('\\')
            .filter(|s| !s.is_empty() && !s.ends_with(':'))
            .collect();
        if segments.is_empty() {
            continue;
        }
        // On Windows paths like `C:\Users\<user>\<folder>\...`, skip Users/<name>.
        let candidate = if segments.len() >= 4
            && segments[0].eq_ignore_ascii_case("Users")
        {
            segments[2]
        } else {
            segments[0]
        };
        if !candidate.is_empty() && !candidate.starts_with('.') {
            *counts.entry(candidate.to_string()).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, c)| *c)
        .map(|(name, _)| name)
}

pub fn short_session_title(id: &str) -> String {
    let prefix: String = id.chars().take(8).collect();
    format!("Session {prefix}")
}

fn looks_like_session_id(name: &str) -> bool {
    // UUID-style session folder (36 chars with hyphens) or >= 16 hex/alnum chars
    name.len() >= 16 && name.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

#[cfg(windows)]
pub fn is_lock_held(lock_path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    if !lock_path.exists() {
        return false;
    }
    match OpenOptions::new().read(true).share_mode(0).open(lock_path) {
        Ok(_file) => false, // Opened exclusively and immediately dropped -> not held.
        Err(err) => err.raw_os_error() == Some(32), // ERROR_SHARING_VIOLATION
    }
}

#[cfg(not(windows))]
pub fn is_lock_held(lock_path: &Path) -> bool {
    lock_path.exists()
}

/// Parses basic RFC3339 / ISO-8601 UTC timestamps (`YYYY-MM-DDTHH:MM:SSZ` or with fractional seconds)
/// into Unix epoch seconds without external date crates.
pub fn parse_iso8601_utc_secs(ts: &str) -> Option<u64> {
    let s = ts.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let min: i64 = s.get(14..16)?.parse().ok()?;
    let sec: i64 = s.get(17..19)?.parse().ok()?;

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&min)
        || !(0..=60).contains(&sec)
    {
        return None;
    }

    // Howard Hinnant's civil-from-days inverse (days since 1970-01-01).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    let total_secs = days * 86400 + hour * 3600 + min * 60 + sec;
    u64::try_from(total_secs).ok()
}

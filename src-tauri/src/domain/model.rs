use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    Working,
    Waiting,
    Error,
    Completed,
    Idle,
    Unknown,
}

impl AgentStatus {
    pub fn sort_rank(self) -> u8 {
        match self {
            AgentStatus::Working => 0,
            AgentStatus::Waiting => 1,
            AgentStatus::Error => 2,
            AgentStatus::Completed => 3,
            AgentStatus::Idle => 4,
            AgentStatus::Unknown => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StatusConfidence {
    Unknown,
    Inferred,
    Confirmed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Connected,
    AntigravityNotRunning,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderMode {
    Auto,
    Mock,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub title: String,
    pub workspace: Option<String>,
    pub status: AgentStatus,
    pub activity: Option<String>,
    pub last_activity: Option<u64>,
    pub last_status_change: Option<u64>,
    pub source: String,
    pub confidence: StatusConfidence,
    pub stale: bool,
    pub disconnected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub sessions: Vec<AgentSession>,
    pub connection: ConnectionState,
    pub antigravity_running: bool,
    pub status_message: Option<String>,
    pub active_providers: Vec<String>,
    pub hooks_enabled: bool,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookIntegrationStatus {
    pub enabled: bool,
    pub config_path: String,
    pub event_log_path: String,
    pub last_event_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub always_on_top: bool,
    pub start_minimized: bool,
    pub launch_at_startup: bool,
    pub theme: ThemeMode,
    pub poll_interval_ms: u64,
    pub hotkey: String,
    pub show_idle_sessions: bool,
    pub provider_mode: ProviderMode,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
    pub window_width: u32,
    pub window_height: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            always_on_top: true,
            start_minimized: false,
            launch_at_startup: false,
            theme: ThemeMode::Dark,
            poll_interval_ms: 2500,
            hotkey: "Ctrl+Shift+A".to_string(),
            show_idle_sessions: true,
            provider_mode: ProviderMode::Auto,
            window_x: None,
            window_y: None,
            window_width: 240,
            window_height: 112,
        }
    }
}

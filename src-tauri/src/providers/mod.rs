pub mod composite;
pub mod hooks;
pub mod mock;
pub mod process;
pub mod transcript;
pub mod uia;

use std::future::Future;
use std::pin::Pin;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::model::{AgentSession, ConnectionState};

pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub struct ProviderOutput {
    pub provider_name: &'static str,
    pub antigravity_running: bool,
    pub connection: ConnectionState,
    pub status_message: Option<String>,
    pub sessions: Vec<AgentSession>,
}

#[derive(Debug, Clone)]
pub enum MonitorError {
    Io(String),
    PermissionDenied(String),
    Unavailable(String),
}

impl std::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MonitorError::Io(msg) => write!(f, "I/O error: {msg}"),
            MonitorError::PermissionDenied(msg) => write!(f, "Permission error: {msg}"),
            MonitorError::Unavailable(msg) => write!(f, "Unavailable: {msg}"),
        }
    }
}

pub trait AgentMonitorProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn collect(&self, now_secs: u64) -> ProviderFuture<'_, Result<ProviderOutput, MonitorError>>;
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

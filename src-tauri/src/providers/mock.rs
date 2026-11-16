use crate::domain::model::{AgentSession, AgentStatus, ConnectionState, StatusConfidence};
use crate::providers::{AgentMonitorProvider, MonitorError, ProviderFuture, ProviderOutput};

/// Deterministic mock provider used in unit/integration tests and optional UI preview mode.
/// Never active in `ProviderMode::Auto`.
#[derive(Debug, Default)]
pub struct MockMonitorProvider {
    pub custom_output: Option<ProviderOutput>,
}

impl MockMonitorProvider {
    pub fn with_demo_sessions() -> Self {
        Self { custom_output: None }
    }

    pub fn build_demo_sessions(now_secs: u64) -> Vec<AgentSession> {
        vec![
            AgentSession {
                id: "mock-backend-api".to_string(),
                title: "Backend API".to_string(),
                workspace: Some("d:\\services\\auth-api".to_string()),
                status: AgentStatus::Working,
                activity: Some("Writing authentication middleware".to_string()),
                last_activity: Some(now_secs.saturating_sub(12)),
                last_status_change: Some(now_secs.saturating_sub(48)),
                source: "mock".to_string(),
                confidence: StatusConfidence::Confirmed,
                stale: false,
                disconnected: false,
            },
            AgentSession {
                id: "mock-frontend-ui".to_string(),
                title: "Frontend UI".to_string(),
                workspace: Some("d:\\apps\\web-console".to_string()),
                status: AgentStatus::Waiting,
                activity: Some("Waiting for terminal command approval".to_string()),
                last_activity: Some(now_secs.saturating_sub(120)),
                last_status_change: Some(now_secs.saturating_sub(120)),
                source: "mock".to_string(),
                confidence: StatusConfidence::Confirmed,
                stale: false,
                disconnected: false,
            },
            AgentSession {
                id: "mock-database".to_string(),
                title: "Database".to_string(),
                workspace: Some("d:\\services\\migrations".to_string()),
                status: AgentStatus::Completed,
                activity: Some("Turn finished".to_string()),
                last_activity: Some(now_secs.saturating_sub(300)),
                last_status_change: Some(now_secs.saturating_sub(300)),
                source: "mock".to_string(),
                confidence: StatusConfidence::Inferred,
                stale: false,
                disconnected: false,
            },
            AgentSession {
                id: "mock-docs".to_string(),
                title: "API Docs".to_string(),
                workspace: Some("d:\\docs\\openapi".to_string()),
                status: AgentStatus::Idle,
                activity: Some("Session open".to_string()),
                last_activity: Some(now_secs.saturating_sub(1420)),
                last_status_change: Some(now_secs.saturating_sub(1420)),
                source: "mock".to_string(),
                confidence: StatusConfidence::Inferred,
                stale: false,
                disconnected: false,
            },
        ]
    }
}

impl AgentMonitorProvider for MockMonitorProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn collect(&self, now_secs: u64) -> ProviderFuture<'_, Result<ProviderOutput, MonitorError>> {
        Box::pin(async move {
            if let Some(ref output) = self.custom_output {
                return Ok(output.clone());
            }

            Ok(ProviderOutput {
                provider_name: "mock",
                antigravity_running: true,
                connection: ConnectionState::Connected,
                status_message: Some("Preview data (mock provider)".to_string()),
                sessions: Self::build_demo_sessions(now_secs),
            })
        })
    }
}

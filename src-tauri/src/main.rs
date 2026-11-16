#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--hook-ingest") {
        antigravity_agent_monitor_lib::providers::hooks::ingest_stdin_and_exit();
        return;
    }

    if std::env::args().any(|arg| arg == "--check-live") {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        let snapshot = rt.block_on(antigravity_agent_monitor_lib::run_live_diagnostic());
        println!(
            "{}",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }

    antigravity_agent_monitor_lib::run();
}

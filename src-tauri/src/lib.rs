pub mod backdrop;
pub mod config;
pub mod domain;
pub mod fullscreen;
pub mod providers;
pub mod tray_window;

use std::sync::Arc;
use notify::{Event, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{watch, Mutex};
use tracing::{debug, info, warn};

use crate::backdrop::{
    sample_ambient_backdrop, sync_glass_regions as apply_glass_regions, BackdropSample,
    GlassRegion,
};
use crate::config::{load_settings, save_settings, settings_file_path};
use crate::domain::classifier::SessionStateTracker;
use crate::domain::model::{
    AppSettings, ConnectionState, DisplayMode, HookIntegrationStatus, MonitorSnapshot,
    ProviderMode,
};
use crate::fullscreen::{is_foreground_fullscreen, FullscreenHysteresis};
use crate::providers::composite::CompositeAntigravityProvider;
use crate::providers::hooks::{
    disable_hooks, enable_hooks, get_status as get_hooks_status, hook_events_log_path,
};
use crate::providers::mock::MockMonitorProvider;
use crate::providers::transcript::TranscriptScanner;
use crate::providers::{now_unix_secs, AgentMonitorProvider};
use crate::tray_window::{
    apply_autostart, apply_display_mode_geometry, apply_window_preset, hide_to_tray,
    minimize_to_taskbar, register_all_hotkeys, setup_window_and_tray, show_overlay_non_focusing,
    try_acquire_single_instance_or_activate, TrayControls, MAIN_WINDOW_LABEL,
};

pub const SNAPSHOT_EVENT: &str = "monitor://snapshot";
pub const FULLSCREEN_EVENT: &str = "monitor://fullscreen-changed";
pub const BACKDROP_SAMPLE_EVENT: &str = "monitor://backdrop-sample";

pub struct AppState {
    pub settings: Arc<Mutex<AppSettings>>,
    pub snapshot: Arc<Mutex<MonitorSnapshot>>,
    pub tracker: Arc<Mutex<SessionStateTracker>>,
    pub real_provider: Arc<CompositeAntigravityProvider>,
    pub tray_controls: Mutex<Option<TrayControls>>,
    pub refresh_tx: watch::Sender<u64>,
}

impl AppState {
    fn new(settings: AppSettings) -> Self {
        let initial_snapshot = MonitorSnapshot {
            sessions: Vec::new(),
            connection: ConnectionState::AntigravityNotRunning,
            antigravity_running: false,
            status_message: Some("Initializing monitor".to_string()),
            active_providers: Vec::new(),
            hooks_enabled: get_hooks_status().enabled,
            updated_at: now_unix_secs(),
        };
        let (refresh_tx, _) = watch::channel(0_u64);
        Self {
            settings: Arc::new(Mutex::new(settings)),
            snapshot: Arc::new(Mutex::new(initial_snapshot)),
            tracker: Arc::new(Mutex::new(SessionStateTracker::new(180, 120))),
            real_provider: Arc::new(CompositeAntigravityProvider::default()),
            tray_controls: Mutex::new(None),
            refresh_tx,
        }
    }
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<MonitorSnapshot, String> {
    Ok(state.snapshot.lock().await.clone())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
async fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    new_settings: AppSettings,
) -> Result<AppSettings, String> {
    let previous = state.settings.lock().await.clone();
    let clean = config::sanitize_settings(new_settings);
    save_settings(&settings_file_path(), &clean)?;

    {
        let mut guard = state.settings.lock().await;
        *guard = clean.clone();
    }

    if previous.always_on_top != clean.always_on_top {
        if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            let _ = win.set_always_on_top(clean.always_on_top);
        }
        if let Some(ref controls) = *state.tray_controls.lock().await {
            let _ = controls.always_on_top_item.set_checked(clean.always_on_top);
        }
    }

    if previous.launch_at_startup != clean.launch_at_startup {
        apply_autostart(&app, clean.launch_at_startup);
    }

    if previous.hotkey != clean.hotkey
        || previous.focus_mode_hotkey != clean.focus_mode_hotkey
        || previous.expand_collapse_hotkey != clean.expand_collapse_hotkey
        || previous.peek_panel_hotkey != clean.peek_panel_hotkey
    {
        register_all_hotkeys(&app, &clean)?;
    }

    let _ = state.refresh_tx.send(now_unix_secs());
    Ok(clean)
}

#[tauri::command]
async fn toggle_always_on_top(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let next = {
        let mut guard = state.settings.lock().await;
        guard.always_on_top = !guard.always_on_top;
        save_settings(&settings_file_path(), &guard)?;
        guard.always_on_top
    };

    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = win.set_always_on_top(next);
    }
    if let Some(ref controls) = *state.tray_controls.lock().await {
        let _ = controls.always_on_top_item.set_checked(next);
    }
    Ok(next)
}

#[tauri::command]
async fn hide_overlay_to_tray(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        hide_to_tray(&win);
    }
    Ok(())
}

#[tauri::command]
async fn minimize_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        minimize_to_taskbar(&win);
    }
    Ok(())
}

#[tauri::command]
async fn set_overlay_geometry(
    app: AppHandle,
    state: State<'_, AppState>,
    mode: DisplayMode,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        apply_display_mode_geometry(
            &win,
            mode,
            settings.indicator_position,
            width,
            height,
            settings.window_x,
            settings.window_y,
        );
    }
    Ok(())
}

#[tauri::command]
async fn sync_glass_regions(app: AppHandle, regions: Vec<GlassRegion>) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        apply_glass_regions(&win, regions);
    }
    Ok(())
}

#[tauri::command]
async fn get_backdrop_sample(app: AppHandle) -> Result<Option<BackdropSample>, String> {
    let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return Ok(None);
    };
    tokio::task::spawn_blocking(move || sample_ambient_backdrop(&win))
        .await
        .map_err(|e| format!("Backdrop sample failed: {e}"))
}

#[tauri::command]
async fn get_fullscreen_state() -> Result<bool, String> {
    Ok(is_foreground_fullscreen())
}

#[tauri::command]
async fn show_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        show_overlay_non_focusing(&win);
    }
    Ok(())
}

#[tauri::command]
async fn set_window_preset(app: AppHandle, preset: String) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        apply_window_preset(&win, &preset)?;
    }
    Ok(())
}

#[tauri::command]
async fn focus_session_window(workspace: Option<String>, title: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        crate::providers::process::focus_antigravity_window(workspace.as_deref(), &title)
    })
    .await
    .map_err(|e| format!("Focus task failed: {e}"))?
}

#[tauri::command]
async fn set_overlay_height(app: AppHandle, height: f64) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        crate::tray_window::resize_overlay_height(&win, height);
    }
    Ok(())
}

#[tauri::command]
async fn refresh_now(state: State<'_, AppState>) -> Result<(), String> {
    let _ = state.refresh_tx.send(now_unix_secs());
    Ok(())
}

#[tauri::command]
async fn quit_app(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
async fn get_hook_integration_status() -> Result<HookIntegrationStatus, String> {
    Ok(get_hooks_status())
}

#[tauri::command]
async fn enable_hook_integration(state: State<'_, AppState>) -> Result<HookIntegrationStatus, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Failed to locate current executable: {e}"))?;
    let status = enable_hooks(&exe)?;
    let _ = state.refresh_tx.send(now_unix_secs());
    Ok(status)
}

#[tauri::command]
async fn disable_hook_integration(state: State<'_, AppState>) -> Result<HookIntegrationStatus, String> {
    let status = disable_hooks()?;
    let _ = state.refresh_tx.send(now_unix_secs());
    Ok(status)
}

pub async fn collect_with_provider(
    provider: &dyn AgentMonitorProvider,
    tracker: &Mutex<SessionStateTracker>,
    now_secs: u64,
    hooks_enabled: bool,
) -> MonitorSnapshot {
    match provider.collect(now_secs).await {
        Ok(out) => {
            let sessions = tracker.lock().await.reconcile(
                out.sessions,
                now_secs,
                out.antigravity_running,
            );
            let providers = out
                .provider_name
                .split('+')
                .map(str::to_string)
                .collect();
            MonitorSnapshot {
                sessions,
                connection: out.connection,
                antigravity_running: out.antigravity_running,
                status_message: out.status_message,
                active_providers: providers,
                hooks_enabled,
                updated_at: now_secs,
            }
        }
        Err(e) => MonitorSnapshot {
            sessions: Vec::new(),
            connection: ConnectionState::Error,
            antigravity_running: false,
            status_message: Some(e.to_string()),
            active_providers: Vec::new(),
            hooks_enabled,
            updated_at: now_secs,
        },
    }
}

pub async fn collect_once(
    settings: &AppSettings,
    real_provider: &CompositeAntigravityProvider,
    tracker: &Mutex<SessionStateTracker>,
) -> MonitorSnapshot {
    let now = now_unix_secs();
    match settings.provider_mode {
        ProviderMode::Mock => {
            let mock = MockMonitorProvider::with_demo_sessions();
            collect_with_provider(&mock, tracker, now, false).await
        }
        ProviderMode::Auto => {
            let hooks_enabled = get_hooks_status().enabled;
            collect_with_provider(real_provider, tracker, now, hooks_enabled).await
        }
    }
}

fn start_fs_watchers(refresh_tx: watch::Sender<u64>) -> Option<notify::RecommendedWatcher> {
    let mut watcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if res.is_ok() {
                let _ = refresh_tx.send(now_unix_secs());
            }
        })
        .ok()?;

    if let Some(root) = TranscriptScanner::antigravity_root() {
        let brain = root.join("brain");
        if brain.exists() {
            if let Err(e) = watcher.watch(&brain, RecursiveMode::Recursive) {
                debug!("Could not watch brain directory: {e}");
            }
        }
        let presence = root.join("presence");
        if presence.exists() {
            if let Err(e) = watcher.watch(&presence, RecursiveMode::NonRecursive) {
                debug!("Could not watch presence directory: {e}");
            }
        }
    }

    if let Some(hook_dir) = hook_events_log_path().parent() {
        if hook_dir.exists() {
            let _ = watcher.watch(hook_dir, RecursiveMode::NonRecursive);
        }
    }

    Some(watcher)
}

pub async fn run_live_diagnostic() -> MonitorSnapshot {
    let settings = AppSettings::default();
    let provider = CompositeAntigravityProvider::default();
    let tracker = Mutex::new(SessionStateTracker::new(180, 120));
    collect_once(&settings, &provider, &tracker).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if !try_acquire_single_instance_or_activate() {
        return;
    }

    let loaded_settings = load_settings(&settings_file_path());
    let app_state = AppState::new(loaded_settings);

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            let state = app.state::<AppState>();
            match setup_window_and_tray(app.handle(), state.settings.clone()) {
                Ok(controls) => {
                    *state.tray_controls.blocking_lock() = Some(controls);
                }
                Err(err) => {
                    warn!("Window/tray setup warning: {err}");
                }
            }

            let app_handle = app.handle().clone();
            let settings_ref = state.settings.clone();
            let snapshot_ref = state.snapshot.clone();
            let tracker_ref = state.tracker.clone();
            let provider_ref = state.real_provider.clone();
            let refresh_tx = state.refresh_tx.clone();
            let mut refresh_rx = state.refresh_tx.subscribe();

            tauri::async_runtime::spawn(async move {
                // Keep watcher alive for the lifetime of the monitor loop.
                let _watcher = start_fs_watchers(refresh_tx);
                info!("Monitor background loop started");

                loop {
                    let current_settings = settings_ref.lock().await.clone();
                    let next_snapshot =
                        collect_once(&current_settings, &provider_ref, &tracker_ref).await;

                    let changed = {
                        let mut snap_guard = snapshot_ref.lock().await;
                        let diff = snap_guard.sessions != next_snapshot.sessions
                            || snap_guard.connection != next_snapshot.connection
                            || snap_guard.antigravity_running != next_snapshot.antigravity_running
                            || snap_guard.status_message != next_snapshot.status_message
                            || snap_guard.hooks_enabled != next_snapshot.hooks_enabled;
                        *snap_guard = next_snapshot.clone();
                        diff
                    };

                    if changed {
                        let _ = app_handle.emit(SNAPSHOT_EVENT, &next_snapshot);
                    }

                    let interval = tokio::time::Duration::from_millis(
                        current_settings.poll_interval_ms.clamp(1000, 30000),
                    );
                    tokio::select! {
                        _ = tokio::time::sleep(interval) => {}
                        res = refresh_rx.changed() => {
                            if res.is_err() {
                                break;
                            }
                            // Small debounce window so bursty file writes coalesce cleanly.
                            tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
                        }
                    }
                }
            });

            let fs_app_handle = app.handle().clone();
            let fs_settings_ref = state.settings.clone();
            tauri::async_runtime::spawn(async move {
                let mut hysteresis = FullscreenHysteresis::default();
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
                    let enabled = fs_settings_ref.lock().await.auto_fullscreen_focus;
                    if !enabled {
                        if hysteresis.is_fullscreen() {
                            hysteresis = FullscreenHysteresis::default();
                            let _ = fs_app_handle.emit(FULLSCREEN_EVENT, false);
                        }
                        continue;
                    }
                    let raw = is_foreground_fullscreen();
                    if let Some(changed) = hysteresis.observe(raw) {
                        let _ = fs_app_handle.emit(FULLSCREEN_EVENT, changed);
                    }
                }
            });

            let bg_app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_sample: Option<BackdropSample> = None;
                loop {
                    if let Some(win) = bg_app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                        let visible = win.is_visible().unwrap_or(false)
                            && !win.is_minimized().unwrap_or(false);
                        if visible {
                            let win_clone = win.clone();
                            if let Ok(Some(sample)) =
                                tokio::task::spawn_blocking(move || sample_ambient_backdrop(&win_clone))
                                    .await
                            {
                                let changed = match &last_sample {
                                    None => true,
                                    Some(prev) => {
                                        prev.transparency_effects_enabled
                                            != sample.transparency_effects_enabled
                                            || prev.r.abs_diff(sample.r) >= 4
                                            || prev.g.abs_diff(sample.g) >= 4
                                            || prev.b.abs_diff(sample.b) >= 4
                                    }
                                };
                                if changed {
                                    last_sample = Some(sample.clone());
                                    let _ = bg_app_handle.emit(BACKDROP_SAMPLE_EVENT, &sample);
                                }
                            }
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_settings,
            update_settings,
            toggle_always_on_top,
            hide_overlay_to_tray,
            minimize_overlay,
            show_overlay,
            set_window_preset,
            focus_session_window,
            set_overlay_height,
            set_overlay_geometry,
            sync_glass_regions,
            get_backdrop_sample,
            get_fullscreen_state,
            refresh_now,
            quit_app,
            get_hook_integration_status,
            enable_hook_integration,
            disable_hook_integration,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Agent Monitor");
}

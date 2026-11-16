use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::config::{save_settings, settings_file_path};
use crate::domain::model::AppSettings;

pub const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "agent-monitor-tray";
const MENU_TOGGLE_ID: &str = "tray-toggle";
const MENU_ON_TOP_ID: &str = "tray-on-top";
const MENU_QUIT_ID: &str = "tray-quit";

pub struct TrayControls {
    pub always_on_top_item: CheckMenuItem<tauri::Wry>,
}

pub fn setup_window_and_tray(
    app: &AppHandle,
    settings_state: Arc<Mutex<AppSettings>>,
) -> Result<TrayControls, String> {
    let initial_settings = settings_state.blocking_lock().clone();

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let bg = match initial_settings.theme {
            crate::domain::model::ThemeMode::Dark => tauri::window::Color(20, 20, 21, 255),
            crate::domain::model::ThemeMode::Light => tauri::window::Color(250, 250, 250, 255),
        };
        let _ = window.set_background_color(Some(bg));
        apply_saved_geometry(&window, &initial_settings);
        let _ = window.set_always_on_top(initial_settings.always_on_top);

        if initial_settings.start_minimized {
            hide_to_tray(&window);
        } else {
            show_overlay_non_focusing(&window);
        }

        let app_handle = app.clone();
        let settings_ref = settings_state.clone();
        window.on_window_event(move |event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    hide_to_tray(&win);
                }
            }
            WindowEvent::Moved(pos) => {
                let x = pos.x;
                let y = pos.y;
                if x <= -30000 || y <= -30000 {
                    return;
                }
                let settings_ref = settings_ref.clone();
                tauri::async_runtime::spawn(async move {
                    let mut guard = settings_ref.lock().await;
                    guard.window_x = Some(x);
                    guard.window_y = Some(y);
                    let _ = save_settings(&settings_file_path(), &guard);
                });
            }
            _ => {}
        });
    }

    // Build tray menu
    let toggle_item = MenuItem::with_id(app, MENU_TOGGLE_ID, "Show / Hide Overlay", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let on_top_item = CheckMenuItem::with_id(
        app,
        MENU_ON_TOP_ID,
        "Always on Top",
        true,
        initial_settings.always_on_top,
        None::<&str>,
    )
    .map_err(|e| e.to_string())?;
    let quit_item =
        MenuItem::with_id(app, MENU_QUIT_ID, "Quit Agent Monitor", true, None::<&str>)
            .map_err(|e| e.to_string())?;

    let menu = Menu::with_items(app, &[&toggle_item, &on_top_item, &quit_item])
        .map_err(|e| e.to_string())?;

    let mut tray_builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Antigravity Agent Monitor")
        .show_menu_on_left_click(false);

    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    let on_top_item_clone = on_top_item.clone();
    let settings_for_menu = settings_state.clone();
    let settings_for_click = settings_state.clone();

    tray_builder
        .on_menu_event(move |app_handle, event| {
            let id = event.id().as_ref();
            if id == MENU_TOGGLE_ID {
                toggle_overlay_visibility(app_handle);
            } else if id == MENU_ON_TOP_ID {
                let checked = on_top_item_clone.is_checked().unwrap_or(true);
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    let _ = win.set_always_on_top(checked);
                }
                let settings_ref = settings_for_menu.clone();
                tauri::async_runtime::spawn(async move {
                    let mut guard = settings_ref.lock().await;
                    guard.always_on_top = checked;
                    let _ = save_settings(&settings_file_path(), &guard);
                });
            } else if id == MENU_QUIT_ID {
                app_handle.exit(0);
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = &settings_for_click;
                toggle_overlay_visibility(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to build system tray: {e}"))?;

    // Sync autostart setting
    apply_autostart(app, initial_settings.launch_at_startup);

    // Register global hotkey
    if let Err(err) = register_hotkey(app, &initial_settings.hotkey) {
        warn!("Could not register initial hotkey {}: {}", initial_settings.hotkey, err);
    }

    Ok(TrayControls {
        always_on_top_item: on_top_item,
    })
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let currently_enabled = manager.is_enabled().unwrap_or(false);
    if enabled && !currently_enabled {
        if let Err(e) = manager.enable() {
            warn!("Failed to enable autostart: {e}");
        }
    } else if !enabled && currently_enabled {
        if let Err(e) = manager.disable() {
            warn!("Failed to disable autostart: {e}");
        }
    }
}

pub fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    let shortcut = hotkey.trim();
    if shortcut.is_empty() {
        return Ok(());
    }
    gs.on_shortcut(shortcut, move |app_handle, _shortcut, _event| {
        toggle_overlay_visibility(app_handle);
    })
    .map_err(|e| format!("Invalid or busy shortcut '{shortcut}': {e}"))?;
    info!("Registered global shortcut: {shortcut}");
    Ok(())
}

pub fn toggle_overlay_visibility(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let visible = win.is_visible().unwrap_or(false);
        let minimized = win.is_minimized().unwrap_or(false);
        if visible && !minimized {
            hide_to_tray(&win);
        } else {
            show_overlay_non_focusing(&win);
        }
    }
}

pub fn hide_to_tray(window: &WebviewWindow) {
    let _ = window.set_skip_taskbar(true);
    let _ = window.hide();
}

pub fn show_overlay_non_focusing(window: &WebviewWindow) {
    let _ = window.set_skip_taskbar(false);
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0), SW_SHOWNOACTIVATE);
            }
            return;
        }
    }

    let _ = window.show();
}

pub fn apply_window_preset(window: &WebviewWindow, preset: &str) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())
        .ok_or_else(|| "No monitor detected".to_string())?;

    let work_area = monitor.work_area();
    let win_size = window
        .outer_size()
        .unwrap_or(PhysicalSize::new(240, 112));
    let margin = 16_i32;

    let (x, y) = match preset {
        "top-left" => (work_area.position.x + margin, work_area.position.y + margin),
        "top-right" => (
            work_area.position.x + work_area.size.width as i32 - win_size.width as i32 - margin,
            work_area.position.y + margin,
        ),
        "bottom-left" => (
            work_area.position.x + margin,
            work_area.position.y + work_area.size.height as i32 - win_size.height as i32 - margin,
        ),
        "bottom-right" => (
            work_area.position.x + work_area.size.width as i32 - win_size.width as i32 - margin,
            work_area.position.y + work_area.size.height as i32 - win_size.height as i32 - margin,
        ),
        _ => return Err(format!("Unknown preset: {preset}")),
    };

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| format!("Failed to position window: {e}"))
}

pub fn resize_overlay_height(window: &WebviewWindow, logical_height: f64) {
    let clamped_h = logical_height.clamp(64.0, 680.0);
    let _ = window.set_size(LogicalSize::new(240.0, clamped_h));
}

fn apply_saved_geometry(window: &WebviewWindow, settings: &AppSettings) {
    let logical_h = settings.window_height.clamp(64, 680) as f64;
    let _ = window.set_size(LogicalSize::new(240.0, logical_h));
    let scale = window.scale_factor().unwrap_or(1.0);
    let width = (240.0 * scale).round() as u32;
    let height = (logical_h * scale).round() as u32;

    if let (Some(x), Some(y)) = (settings.window_x, settings.window_y) {
        if is_point_on_any_monitor(window, x, y, width, height) {
            let _ = window.set_position(PhysicalPosition::new(x, y));
            return;
        }
    }

    // Default to top-right corner of primary work area with comfortable margin.
    let _ = apply_window_preset(window, "top-right");
}

fn is_point_on_any_monitor(
    window: &WebviewWindow,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> bool {
    let Ok(monitors) = window.available_monitors() else {
        return false;
    };
    let right = x.saturating_add(width as i32);
    let bottom = y.saturating_add(height as i32);
    monitors.into_iter().any(|m| {
        let pos = m.position();
        let sz = m.size();
        let m_right = pos.x + sz.width as i32;
        let m_bottom = pos.y + sz.height as i32;
        // Require at least 80px overlap so window title/drag handle remains reachable.
        right > pos.x + 80 && x < m_right - 80 && bottom > pos.y + 48 && y < m_bottom - 48
    })
}

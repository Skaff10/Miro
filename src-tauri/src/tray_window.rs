use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize, WebviewWindow,
    WindowEvent,
};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::config::{save_settings, settings_file_path};
use crate::domain::model::{AppSettings, DisplayMode, IndicatorPosition};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const SHORTCUT_ACTION_EVENT: &str = "monitor://shortcut-action";
pub const SET_MODE_EVENT: &str = "monitor://set-mode";

const TRAY_ID: &str = "agent-monitor-tray";
const MENU_TOGGLE_ID: &str = "tray-toggle";
const MENU_MODE_NORMAL_ID: &str = "tray-mode-normal";
const MENU_MODE_COMPACT_ID: &str = "tray-mode-compact";
const MENU_MODE_FOCUS_ID: &str = "tray-mode-focus";
const MENU_ON_TOP_ID: &str = "tray-on-top";
const MENU_QUIT_ID: &str = "tray-quit";

/// Tracks current display mode (`0 = Normal`, `1 = Compact`, `2 = Focus`) so window move events
/// only persist `window_x`/`window_y` in Normal mode.
static CURRENT_MODE_FLAG: AtomicU8 = AtomicU8::new(0);

#[cfg(windows)]
static SINGLE_INSTANCE_EVENT_HANDLE: std::sync::OnceLock<isize> = std::sync::OnceLock::new();
#[cfg(windows)]
static SINGLE_INSTANCE_MUTEX_HANDLE: std::sync::OnceLock<isize> = std::sync::OnceLock::new();

pub struct TrayControls {
    pub always_on_top_item: CheckMenuItem<tauri::Wry>,
}

/// Acquires the Windows single-instance mutex. If another instance is already running,
/// signals it to reveal its window and returns `false` so this process can exit cleanly.
#[cfg(windows)]
pub fn try_acquire_single_instance_or_activate() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::{
        CreateEventW, CreateMutexW, OpenEventW, SetEvent, EVENT_MODIFY_STATE,
    };

    unsafe {
        let Ok(mutex) = CreateMutexW(None, false, w!("Local\\AntigravityAgentMonitorSingleton"))
        else {
            return true;
        };
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(mutex);
            if let Ok(ev) = OpenEventW(
                EVENT_MODIFY_STATE,
                false,
                w!("Local\\AntigravityAgentMonitorShowEvent"),
            ) {
                let _ = SetEvent(ev);
                let _ = CloseHandle(ev);
            }
            return false;
        }
        let _ = SINGLE_INSTANCE_MUTEX_HANDLE.set(mutex.0 as isize);

        if let Ok(ev) = CreateEventW(
            None,
            false,
            false,
            w!("Local\\AntigravityAgentMonitorShowEvent"),
        ) {
            let _ = SINGLE_INSTANCE_EVENT_HANDLE.set(ev.0 as isize);
        }
    }
    true
}

#[cfg(not(windows))]
pub fn try_acquire_single_instance_or_activate() -> bool {
    true
}

fn spawn_single_instance_listener(app: &AppHandle) {
    #[cfg(windows)]
    {
        if let Some(&ev_raw) = SINGLE_INSTANCE_EVENT_HANDLE.get() {
            let app_handle = app.clone();
            std::thread::spawn(move || {
                use std::ffi::c_void;
                use windows::Win32::Foundation::HANDLE;
                use windows::Win32::System::Threading::{WaitForSingleObject, INFINITE};

                let handle = HANDLE(ev_raw as *mut c_void);
                loop {
                    let wait_res = unsafe { WaitForSingleObject(handle, INFINITE) };
                    if wait_res.0 != 0 {
                        break;
                    }
                    if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                        show_overlay_non_focusing(&win);
                    }
                }
            });
        }
    }
    #[cfg(not(windows))]
    let _ = app;
}

fn enable_taskbar_minimize_style(window: &WebviewWindow) {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_MINIMIZEBOX,
        };
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let hwnd = HWND(hwnd.0);
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                if (style & (WS_MINIMIZEBOX.0 as isize)) == 0 {
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style | (WS_MINIMIZEBOX.0 as isize));
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                    );
                }
            }
        }
    }
    #[cfg(not(windows))]
    let _ = window;
}

pub fn setup_window_and_tray(
    app: &AppHandle,
    settings_state: Arc<Mutex<AppSettings>>,
) -> Result<TrayControls, String> {
    let initial_settings = settings_state.blocking_lock().clone();
    let is_autostart_launch = std::env::args().any(|a| a == "--autostart" || a == "--minimized");

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.set_skip_taskbar(true);
        enable_taskbar_minimize_style(&window);
        crate::backdrop::prepare_window_for_glass(&window);

        let initial_mode_flag = match initial_settings.display_mode {
            DisplayMode::Normal => 0,
            DisplayMode::Compact => 1,
            DisplayMode::Focus => 2,
        };
        CURRENT_MODE_FLAG.store(initial_mode_flag, Ordering::Relaxed);

        let (init_w, init_h) = match initial_settings.display_mode {
            DisplayMode::Normal => (304.0, initial_settings.window_height.clamp(64, 680) as f64),
            DisplayMode::Compact => (180.0, 48.0),
            DisplayMode::Focus => (64.0, 38.0),
        };
        apply_display_mode_geometry(
            &window,
            initial_settings.display_mode,
            initial_settings.indicator_position,
            init_w,
            init_h,
            initial_settings.window_x,
            initial_settings.window_y,
        );
        let _ = window.set_always_on_top(initial_settings.always_on_top);

        // Only start minimized to tray when launched automatically at OS startup with startMinimized enabled.
        // Direct manual launches (double-clicking the .exe) always open the window visibly.
        if initial_settings.start_minimized && is_autostart_launch {
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
                // Only persist window position in Normal mode so snapping Compact/Focus pills
                // doesn't overwrite the user's saved Normal panel position.
                if CURRENT_MODE_FLAG.load(Ordering::Relaxed) != 0 {
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

    spawn_single_instance_listener(app);

    // Build tray menu
    let toggle_item =
        MenuItem::with_id(app, MENU_TOGGLE_ID, "Show / Hide Overlay", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let mode_normal_item =
        MenuItem::with_id(app, MENU_MODE_NORMAL_ID, "Normal Mode", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let mode_compact_item =
        MenuItem::with_id(app, MENU_MODE_COMPACT_ID, "Compact Mode", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let mode_focus_item =
        MenuItem::with_id(app, MENU_MODE_FOCUS_ID, "Focus Mode", true, None::<&str>)
            .map_err(|e| e.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
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

    let menu = Menu::with_items(
        app,
        &[
            &toggle_item,
            &sep1,
            &mode_normal_item,
            &mode_compact_item,
            &mode_focus_item,
            &sep2,
            &on_top_item,
            &quit_item,
        ],
    )
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

    tray_builder
        .on_menu_event(move |app_handle, event| {
            let id = event.id().as_ref();
            if id == MENU_TOGGLE_ID {
                toggle_overlay_visibility(app_handle);
            } else if id == MENU_MODE_NORMAL_ID {
                let _ = app_handle.emit(SET_MODE_EVENT, "normal");
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    show_overlay_non_focusing(&win);
                }
            } else if id == MENU_MODE_COMPACT_ID {
                let _ = app_handle.emit(SET_MODE_EVENT, "compact");
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    show_overlay_non_focusing(&win);
                }
            } else if id == MENU_MODE_FOCUS_ID {
                let _ = app_handle.emit(SET_MODE_EVENT, "focus");
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    show_overlay_non_focusing(&win);
                }
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
                toggle_overlay_visibility(tray.app_handle());
            }
        })
        .build(app)
        .map_err(|e| format!("Failed to build system tray: {e}"))?;

    // Sync autostart setting (ensuring `--autostart` flag is registered when enabled)
    apply_autostart(app, initial_settings.launch_at_startup);

    // Register global hotkeys
    if let Err(err) = register_all_hotkeys(app, &initial_settings) {
        warn!("Could not register initial hotkeys: {err}");
    }

    Ok(TrayControls {
        always_on_top_item: on_top_item,
    })
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) {
    let manager = app.autolaunch();
    let currently_enabled = manager.is_enabled().unwrap_or(false);
    if enabled {
        if let Err(e) = manager.enable() {
            warn!("Failed to enable autostart: {e}");
        }
    } else if currently_enabled {
        if let Err(e) = manager.disable() {
            warn!("Failed to disable autostart: {e}");
        }
    }
}

pub fn register_all_hotkeys(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let entries = [
        ("toggle_visibility", settings.hotkey.trim()),
        ("toggle_focus_mode", settings.focus_mode_hotkey.trim()),
        ("toggle_expand_collapse", settings.expand_collapse_hotkey.trim()),
        ("peek_panel", settings.peek_panel_hotkey.trim()),
    ];

    let mut seen: HashSet<String> = HashSet::new();
    let mut errors: Vec<String> = Vec::new();

    for (action, shortcut) in entries {
        if shortcut.is_empty() {
            continue;
        }
        let norm = shortcut.to_ascii_lowercase();
        if !seen.insert(norm) {
            errors.push(format!("Duplicate shortcut '{shortcut}'"));
            continue;
        }
        let action_owned = action.to_string();
        if let Err(e) = gs.on_shortcut(shortcut, move |app_handle, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            if action_owned == "toggle_visibility" {
                toggle_overlay_visibility(app_handle);
            } else {
                if let Some(win) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                    let visible = win.is_visible().unwrap_or(false);
                    let minimized = win.is_minimized().unwrap_or(false);
                    if !visible || minimized {
                        show_overlay_non_focusing(&win);
                    }
                }
                let _ = app_handle.emit(SHORTCUT_ACTION_EVENT, &action_owned);
            }
        }) {
            errors.push(format!("'{shortcut}': {e}"));
        } else {
            info!("Registered global shortcut {action}: {shortcut}");
        }
    }

    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
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

pub fn minimize_to_taskbar(window: &WebviewWindow) {
    hide_to_tray(window);
}

pub fn hide_to_tray(window: &WebviewWindow) {
    let _ = window.set_skip_taskbar(true);
    let _ = window.hide();
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0), SW_HIDE);
            }
        }
    }
}

pub fn show_overlay_non_focusing(window: &WebviewWindow) {
    let _ = window.set_skip_taskbar(true);
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            SetWindowPos, ShowWindow, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
            SWP_SHOWWINDOW, SW_SHOWNOACTIVATE,
        };
        if let Ok(hwnd) = window.hwnd() {
            unsafe {
                let hwnd = HWND(hwnd.0);
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
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

    let (x, y) = corner_coordinates(
        work_area.position.x,
        work_area.position.y,
        work_area.size.width as i32,
        work_area.size.height as i32,
        win_size.width as i32,
        win_size.height as i32,
        margin,
        preset,
    )?;

    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|e| format!("Failed to position window: {e}"))
}

fn corner_coordinates(
    area_x: i32,
    area_y: i32,
    area_w: i32,
    area_h: i32,
    win_w: i32,
    win_h: i32,
    margin: i32,
    preset: &str,
) -> Result<(i32, i32), String> {
    match preset {
        "top-left" => Ok((area_x + margin, area_y + margin)),
        "top-right" => Ok((area_x + area_w - win_w - margin, area_y + margin)),
        "bottom-left" => Ok((area_x + margin, area_y + area_h - win_h - margin)),
        "bottom-right" => Ok((area_x + area_w - win_w - margin, area_y + area_h - win_h - margin)),
        _ => Err(format!("Unknown preset: {preset}")),
    }
}

pub fn apply_display_mode_geometry(
    window: &WebviewWindow,
    mode: DisplayMode,
    indicator_pos: IndicatorPosition,
    logical_width: f64,
    logical_height: f64,
    saved_x: Option<i32>,
    saved_y: Option<i32>,
) {
    let mode_flag = match mode {
        DisplayMode::Normal => 0,
        DisplayMode::Compact => 1,
        DisplayMode::Focus => 2,
    };
    let prev_flag = CURRENT_MODE_FLAG.swap(mode_flag, Ordering::Relaxed);

    let (clamped_w, clamped_h) = match mode {
        DisplayMode::Normal => (
            logical_width.clamp(220.0, 380.0),
            logical_height.clamp(64.0, 680.0),
        ),
        DisplayMode::Compact => (
            logical_width.clamp(120.0, 280.0),
            logical_height.clamp(32.0, 180.0),
        ),
        DisplayMode::Focus => (
            logical_width.clamp(44.0, 280.0),
            logical_height.clamp(24.0, 180.0),
        ),
    };

    let _ = window.set_size(LogicalSize::new(clamped_w, clamped_h));

    let scale = window.scale_factor().unwrap_or(1.0);
    let phys_w = (clamped_w * scale).round() as u32;
    let phys_h = (clamped_h * scale).round() as u32;

    match mode {
        DisplayMode::Normal => {
            if prev_flag != 0 || saved_x.is_some() {
                if let (Some(x), Some(y)) = (saved_x, saved_y) {
                    if is_point_on_any_monitor(window, x, y, phys_w, phys_h) {
                        let _ = window.set_position(PhysicalPosition::new(x, y));
                        return;
                    }
                }
                if prev_flag != 0 {
                    let _ = apply_window_preset(window, "top-right");
                }
            }
        }
        DisplayMode::Compact | DisplayMode::Focus => {
            let monitor = window
                .current_monitor()
                .ok()
                .flatten()
                .or_else(|| window.primary_monitor().ok().flatten());
            if let Some(m) = monitor {
                let (ax, ay, aw, ah) = if mode == DisplayMode::Focus {
                    let p = m.position();
                    let s = m.size();
                    (p.x, p.y, s.width as i32, s.height as i32)
                } else {
                    let wa = m.work_area();
                    (wa.position.x, wa.position.y, wa.size.width as i32, wa.size.height as i32)
                };
                let margin = if mode == DisplayMode::Focus { 4 } else { 8 };
                if let Ok((x, y)) = corner_coordinates(
                    ax,
                    ay,
                    aw,
                    ah,
                    phys_w as i32,
                    phys_h as i32,
                    margin,
                    indicator_pos.as_preset_str(),
                ) {
                    let _ = window.set_position(PhysicalPosition::new(x, y));
                }
            }
        }
    }
}

pub fn resize_overlay_height(window: &WebviewWindow, logical_height: f64) {
    let clamped_h = logical_height.clamp(64.0, 680.0);
    let _ = window.set_size(LogicalSize::new(304.0, clamped_h));
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

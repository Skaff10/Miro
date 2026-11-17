/// Checks whether `win_rect` `(left, top, right, bottom)` covers `monitor_rect` `(left, top, right, bottom)`.
/// Allows a 2px tolerance for borderless fullscreen windows with thin DWM frames.
pub fn is_rect_fullscreen(
    win_rect: (i32, i32, i32, i32),
    monitor_rect: (i32, i32, i32, i32),
) -> bool {
    let (wl, wt, wr, wb) = win_rect;
    let (ml, mt, mr, mb) = monitor_rect;
    if wr <= wl || wb <= wt || mr <= ml || mb <= mt {
        return false;
    }
    wl <= ml + 2 && wt <= mt + 2 && wr >= mr - 2 && wb >= mb - 2
}

pub fn is_shell_or_desktop_class(class_name: &str) -> bool {
    matches!(
        class_name,
        "Progman"
            | "WorkerW"
            | "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "Windows.UI.Core.CoreWindow"
    )
}

/// Hysteresis tracker so transient Alt-Tab frames do not flicker display modes.
#[derive(Debug, Default)]
pub struct FullscreenHysteresis {
    stable_fullscreen: bool,
    pending_state: Option<bool>,
    pending_count: u8,
}

impl FullscreenHysteresis {
    pub fn observe(&mut self, raw_fullscreen: bool) -> Option<bool> {
        if raw_fullscreen == self.stable_fullscreen {
            self.pending_state = None;
            self.pending_count = 0;
            return None;
        }
        match self.pending_state {
            Some(s) if s == raw_fullscreen => {
                self.pending_count = self.pending_count.saturating_add(1);
                if self.pending_count >= 2 {
                    self.stable_fullscreen = raw_fullscreen;
                    self.pending_state = None;
                    self.pending_count = 0;
                    return Some(raw_fullscreen);
                }
            }
            _ => {
                self.pending_state = Some(raw_fullscreen);
                self.pending_count = 1;
            }
        }
        None
    }

    pub fn is_fullscreen(&self) -> bool {
        self.stable_fullscreen
    }
}

#[cfg(windows)]
pub fn is_foreground_fullscreen() -> bool {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible,
    };

    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        if !IsWindowVisible(fg).as_bool() || IsIconic(fg).as_bool() {
            return false;
        }

        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }

        let mut class_buf = [0u16; 128];
        let class_len = GetClassNameW(fg, &mut class_buf);
        if class_len > 0 {
            let class_name = OsString::from_wide(&class_buf[..class_len as usize])
                .to_string_lossy()
                .into_owned();
            if is_shell_or_desktop_class(&class_name) {
                return false;
            }
        }

        let mut win_rect = RECT::default();
        if GetWindowRect(fg, &mut win_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        if monitor.is_invalid() {
            return false;
        }
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut mi).as_bool() {
            return false;
        }

        is_rect_fullscreen(
            (win_rect.left, win_rect.top, win_rect.right, win_rect.bottom),
            (
                mi.rcMonitor.left,
                mi.rcMonitor.top,
                mi.rcMonitor.right,
                mi.rcMonitor.bottom,
            ),
        )
    }
}

#[cfg(not(windows))]
pub fn is_foreground_fullscreen() -> bool {
    false
}

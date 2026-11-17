use serde::{Deserialize, Serialize};
#[cfg(not(windows))]
use tauri::WebviewWindow;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlassRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    #[serde(default)]
    pub blur: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackdropSample {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub luminance: f32,
    pub transparency_effects_enabled: bool,
}

#[cfg(windows)]
mod win_impl {
    use super::{BackdropSample, GlassRegion};
    use std::ffi::c_void;
    use std::mem;
    use tauri::WebviewWindow;
    use windows::UI::ViewManagement::UISettings;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE,
        DWMWCP_DONOTROUND,
    };
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, GetWindowRect, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
    };

    const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;

    pub fn is_transparency_effects_enabled() -> bool {
        UISettings::new()
            .and_then(|s| s.AdvancedEffectsEnabled())
            .unwrap_or(true)
    }

    /// Suppresses Windows 11 native DWM outer border ring and 8px corner rounding
    /// without calling `DwmExtendFrameIntoClientArea` or attaching a secondary
    /// composition target over WebView2, keeping corners 100% alpha-transparent.
    pub fn prepare_window_for_glass(window: &WebviewWindow) {
        let Ok(hwnd_handle) = window.hwnd() else {
            return;
        };
        let hwnd = HWND(hwnd_handle.0);

        unsafe {
            let no_border = DWMWA_COLOR_NONE;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &no_border as *const u32 as *const c_void,
                mem::size_of::<u32>() as u32,
            );

            let corner_pref = DWMWCP_DONOTROUND.0 as u32;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const u32 as *const c_void,
                mem::size_of::<u32>() as u32,
            );
        }
    }

    pub fn sync_glass_regions(_window: &WebviewWindow, _regions: Vec<GlassRegion>) {}

    /// Samples a thin strip strictly outside the overlay window perimeter
    /// (avoiding self-capture) so the glass UI adapts ink contrast and edge tint
    /// naturally to bright, dark, or colorful wallpapers.
    pub fn sample_ambient_backdrop(window: &WebviewWindow) -> Option<BackdropSample> {
        let transparency_effects_enabled = is_transparency_effects_enabled();
        let hwnd_handle = window.hwnd().ok()?;
        let hwnd = HWND(hwnd_handle.0);

        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut rect).ok()?;
        }

        let vx = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let vy = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let vw = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let vh = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
        if vw <= 0 || vh <= 0 {
            return None;
        }
        let vmax_x = vx + vw - 1;
        let vmax_y = vy + vh - 1;

        let strip_w = ((rect.right - rect.left) / 2).clamp(4, 16);
        let center_x =
            ((rect.left + rect.right) / 2 - strip_w / 2).clamp(vx, (vmax_x - strip_w).max(vx));

        let mut sample_points: Vec<(i32, i32, i32, i32)> = Vec::with_capacity(2);
        if rect.top - 5 >= vy {
            sample_points.push((center_x, rect.top - 5, strip_w, 1));
        }
        if rect.bottom + 5 <= vmax_y {
            sample_points.push((center_x, rect.bottom + 5, strip_w, 1));
        }
        if sample_points.is_empty() {
            let cy = ((rect.top + rect.bottom) / 2).clamp(vy, vmax_y);
            if rect.left - 5 >= vx {
                sample_points.push((rect.left - 5, cy, 1, 1));
            } else if rect.right + 5 <= vmax_x {
                sample_points.push((rect.right + 5, cy, 1, 1));
            }
        }
        if sample_points.is_empty() {
            return None;
        }

        unsafe {
            let screen_dc = GetDC(None);
            if screen_dc.is_invalid() {
                return None;
            }
            let mem_dc = CreateCompatibleDC(screen_dc);
            if mem_dc.is_invalid() {
                let _ = ReleaseDC(None, screen_dc);
                return None;
            }

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: 16,
                    biHeight: -1,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits_ptr: *mut c_void = std::ptr::null_mut();
            let dib = CreateDIBSection(mem_dc, &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0);
            let Ok(dib) = dib else {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(None, screen_dc);
                return None;
            };

            let old_obj = SelectObject(mem_dc, dib);
            let mut sum_r: u64 = 0;
            let mut sum_g: u64 = 0;
            let mut sum_b: u64 = 0;
            let mut count: u64 = 0;

            for (sx, sy, sw, sh) in sample_points {
                let w = sw.min(16);
                if BitBlt(mem_dc, 0, 0, w, sh, screen_dc, sx, sy, SRCCOPY).is_ok()
                    && !bits_ptr.is_null()
                {
                    let pixels =
                        std::slice::from_raw_parts(bits_ptr as *const u8, (w * 4) as usize);
                    for px in pixels.chunks_exact(4) {
                        sum_b += px[0] as u64;
                        sum_g += px[1] as u64;
                        sum_r += px[2] as u64;
                        count += 1;
                    }
                }
            }

            SelectObject(mem_dc, old_obj);
            let _ = DeleteObject(dib);
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(None, screen_dc);

            if count == 0 {
                return None;
            }

            let r = (sum_r / count) as u8;
            let g = (sum_g / count) as u8;
            let b = (sum_b / count) as u8;
            let luminance =
                (0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32)) / 255.0;

            Some(BackdropSample {
                r,
                g,
                b,
                luminance,
                transparency_effects_enabled,
            })
        }
    }
}

#[cfg(windows)]
pub use win_impl::{prepare_window_for_glass, sample_ambient_backdrop, sync_glass_regions};

#[cfg(not(windows))]
pub fn prepare_window_for_glass(_window: &WebviewWindow) {}

#[cfg(not(windows))]
pub fn sync_glass_regions(_window: &WebviewWindow, _regions: Vec<GlassRegion>) {}

#[cfg(not(windows))]
pub fn sample_ambient_backdrop(_window: &WebviewWindow) -> Option<BackdropSample> {
    None
}

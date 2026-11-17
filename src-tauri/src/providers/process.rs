use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdeWindowInfo {
    pub hwnd_raw: isize,
    pub pid: u32,
    pub title: String,
    pub workspace_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessEnvironment {
    pub antigravity_ide_pids: HashSet<u32>,
    pub agy_cli_pids: HashSet<u32>,
    pub windows: Vec<IdeWindowInfo>,
}

impl ProcessEnvironment {
    pub fn antigravity_running(&self) -> bool {
        !self.antigravity_ide_pids.is_empty() || !self.agy_cli_pids.is_empty()
    }
}

/// Parses standard VS Code / Antigravity IDE window titles such as
/// `"main.rs - MIRO - Antigravity IDE"` or `"MIRO - Antigravity IDE"`.
pub fn extract_workspace_from_title(title: &str) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.split(" - ").collect();
    if let Some(ag_idx) = parts.iter().rposition(|p| p.contains("Antigravity")) {
        if ag_idx >= 1 {
            let mut candidate = parts[ag_idx - 1].trim();
            for suffix in [" (Workspace)", " [Administrator]"] {
                if let Some(stripped) = candidate.strip_suffix(suffix) {
                    candidate = stripped.trim();
                }
            }
            if !candidate.is_empty()
                && !candidate.eq_ignore_ascii_case("Antigravity IDE")
                && !candidate.eq_ignore_ascii_case("Antigravity")
                && !candidate.eq_ignore_ascii_case("Welcome")
                && !candidate.eq_ignore_ascii_case("Get Started")
            {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

#[cfg(windows)]
pub fn scan_antigravity_processes() -> Result<ProcessEnvironment, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::{BOOL, CloseHandle, HWND, LPARAM};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    let mut env = ProcessEnvironment::default();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| format!("Failed to snapshot processes: {e}"))?;

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe = OsString::from_wide(&entry.szExeFile[..len])
                    .to_string_lossy()
                    .to_ascii_lowercase();

                if exe == "antigravity ide.exe" || exe == "antigravity.exe" {
                    env.antigravity_ide_pids.insert(entry.th32ProcessID);
                } else if exe == "agy.exe" {
                    env.agy_cli_pids.insert(entry.th32ProcessID);
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);

        struct EnumCtx<'a> {
            ide_pids: &'a HashSet<u32>,
            out: Vec<IdeWindowInfo>,
        }

        unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam.0 as *mut EnumCtx<'_>);
            if !IsWindowVisible(hwnd).as_bool() {
                return BOOL(1);
            }
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if !ctx.ide_pids.contains(&pid) {
                return BOOL(1);
            }
            let text_len = GetWindowTextLengthW(hwnd);
            if text_len <= 0 {
                return BOOL(1);
            }
            let mut buf = vec![0u16; (text_len + 1) as usize];
            let copied = GetWindowTextW(hwnd, &mut buf);
            if copied > 0 {
                let title = OsString::from_wide(&buf[..copied as usize])
                    .to_string_lossy()
                    .into_owned();
                let workspace_name = extract_workspace_from_title(&title);
                ctx.out.push(IdeWindowInfo {
                    hwnd_raw: hwnd.0 as isize,
                    pid,
                    title,
                    workspace_name,
                });
            }
            BOOL(1)
        }

        if !env.antigravity_ide_pids.is_empty() {
            let mut ctx = EnumCtx {
                ide_pids: &env.antigravity_ide_pids,
                out: Vec::new(),
            };
            let _ = EnumWindows(Some(enum_cb), LPARAM(&mut ctx as *mut EnumCtx<'_> as isize));
            env.windows = ctx.out;
        }
    }

    Ok(env)
}

#[cfg(not(windows))]
pub fn scan_antigravity_processes() -> Result<ProcessEnvironment, String> {
    Ok(ProcessEnvironment::default())
}

#[cfg(windows)]
pub fn focus_antigravity_window(workspace: Option<&str>, title: &str) -> Result<bool, String> {
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
    };

    let env = scan_antigravity_processes()?;
    if env.windows.is_empty() {
        return Ok(false);
    }

    let needle = workspace
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let t = title.trim();
            if t.is_empty() || t.starts_with("Session ") {
                None
            } else {
                Some(t)
            }
        });

    let matched = if let Some(target) = needle {
        let target_lower = target.to_ascii_lowercase();
        let target_base = std::path::Path::new(target)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_else(|| target_lower.clone());

        env.windows
            .iter()
            .find(|w| {
                w.workspace_name
                    .as_ref()
                    .is_some_and(|wn| wn.to_ascii_lowercase() == target_base)
            })
            .or_else(|| {
                env.windows.iter().find(|w| {
                    w.title.to_ascii_lowercase().contains(&target_base)
                        || w.title.to_ascii_lowercase().contains(&target_lower)
                })
            })
    } else if env.windows.len() == 1 {
        env.windows.first()
    } else {
        None
    };

    let Some(win) = matched.or_else(|| env.windows.first()) else {
        return Ok(false);
    };

    unsafe {
        let hwnd = HWND(win.hwnd_raw as *mut c_void);
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        let fg = GetForegroundWindow();
        let mut fg_tid = 0u32;
        if !fg.0.is_null() {
            fg_tid = GetWindowThreadProcessId(fg, None);
        }
        let cur_tid = GetCurrentThreadId();
        let attached = fg_tid != 0 && fg_tid != cur_tid
            && AttachThreadInput(cur_tid, fg_tid, true).as_bool();

        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);

        if attached {
            let _ = AttachThreadInput(cur_tid, fg_tid, false);
        }
    }

    Ok(true)
}

#[cfg(not(windows))]
pub fn focus_antigravity_window(_workspace: Option<&str>, _title: &str) -> Result<bool, String> {
    Ok(false)
}

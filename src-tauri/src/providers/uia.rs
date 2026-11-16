use std::time::{Duration, Instant};

use crate::providers::process::IdeWindowInfo;

#[derive(Debug, Clone, Default)]
pub struct UiaWindowSignal {
    pub hwnd_raw: isize,
    pub workspace_name: Option<String>,
    pub approval_prompt_visible: bool,
    pub stop_button_visible: bool,
    pub detected_cue: Option<String>,
}

/// Inspects visible Antigravity IDE windows via Windows UI Automation with strict
/// node and latency budgets so an unresponsive or massive Electron DOM never stalls monitoring.
#[cfg(windows)]
pub fn inspect_ide_windows(windows: &[IdeWindowInfo]) -> Vec<UiaWindowSignal> {
    use std::collections::VecDeque;
    use std::ffi::c_void;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_MULTITHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, UIA_ButtonControlTypeId,
    };

    if windows.is_empty() {
        return Vec::new();
    }

    let mut signals = Vec::with_capacity(windows.len());

    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let com_initialized = hr.is_ok();

        let Ok(automation): Result<IUIAutomation, _> =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            if com_initialized {
                CoUninitialize();
            }
            return signals;
        };

        let Ok(walker) = automation.ControlViewWalker() else {
            if com_initialized {
                CoUninitialize();
            }
            return signals;
        };

        for win in windows {
            let mut sig = UiaWindowSignal {
                hwnd_raw: win.hwnd_raw,
                workspace_name: win.workspace_name.clone(),
                ..Default::default()
            };

            let Ok(root) = automation.ElementFromHandle(HWND(win.hwnd_raw as *mut c_void)) else {
                signals.push(sig);
                continue;
            };

            let started = Instant::now();
            let budget = Duration::from_millis(90);
            let max_nodes = 140usize;
            let max_depth = 6u8;

            let mut queue: VecDeque<(IUIAutomationElement, u8)> = VecDeque::new();
            queue.push_back((root, 0));
            let mut visited = 0usize;

            while let Some((elem, depth)) = queue.pop_front() {
                if visited >= max_nodes || started.elapsed() > budget {
                    break;
                }
                visited += 1;

                if depth > 0 {
                    if let (Ok(ctrl_type), Ok(name_bstr)) =
                        (elem.CurrentControlType(), elem.CurrentName())
                    {
                        let name = name_bstr.to_string();
                        if ctrl_type == UIA_ButtonControlTypeId {
                            let lower = name.trim().to_ascii_lowercase();
                            if is_approval_button_label(&lower) {
                                sig.approval_prompt_visible = true;
                                sig.detected_cue = Some(format!("UI prompt: {name}"));
                            } else if is_stop_button_label(&lower) {
                                sig.stop_button_visible = true;
                                if sig.detected_cue.is_none() {
                                    sig.detected_cue = Some("Active agent turn in UI".to_string());
                                }
                            }
                        }
                    }
                }

                if depth < max_depth {
                    if let Ok(mut child) = walker.GetFirstChildElement(&elem) {
                        let mut siblings = 0usize;
                        while siblings < 24 && visited + queue.len() < max_nodes {
                            queue.push_back((child.clone(), depth + 1));
                            siblings += 1;
                            match walker.GetNextSiblingElement(&child) {
                                Ok(next) => child = next,
                                Err(_) => break,
                            }
                        }
                    }
                }
            }

            signals.push(sig);
        }

        if com_initialized {
            CoUninitialize();
        }
    }

    signals
}

#[cfg(not(windows))]
pub fn inspect_ide_windows(_windows: &[IdeWindowInfo]) -> Vec<UiaWindowSignal> {
    Vec::new()
}

pub fn is_approval_button_label(lower: &str) -> bool {
    matches!(
        lower,
        "accept" | "approve" | "allow" | "allow once" | "always allow" | "run command" | "proceed"
    )
}

pub fn is_stop_button_label(lower: &str) -> bool {
    matches!(lower, "stop" | "stop generating" | "cancel generation")
}

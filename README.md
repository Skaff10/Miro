# Antigravity Agent Monitor

A lightweight, read-only Windows desktop overlay built with **Tauri 2**, **Rust**, **React**, **TypeScript**, and **Tailwind CSS** that continuously monitors active Google Antigravity IDE agent sessions.

> **Scope (v1):** Monitoring and status display only. Does not send prompts, approve/reject tool permissions, modify Antigravity binaries, or inject into Antigravity processes.

---

## Integration Architecture & Limitations

Because Antigravity IDE does not expose a public external REST/IPC API for listing active tabs without internal CSRF tokens, Agent Monitor combines supported and non-invasive local signals behind the `AgentMonitorProvider` trait:

1. **Process & Window Enumeration (`process.rs`)**:
   - Uses Win32 Toolhelp32 and `EnumWindows` / `GetWindowTextW` to detect `Antigravity IDE.exe`, `agy.exe`, and open workspace window titles.
2. **Presence Locks & Documented Transcripts (`transcript.rs`)**:
   - Probes `%USERPROFILE%\.gemini\antigravity\presence\<conversation-id>.lock` (`ERROR_SHARING_VIOLATION` check) to confirm live open sessions.
   - Incrementally tails structural metadata (`step_index`, `source`, `type`, `status`, `created_at`, `tool_calls[].name`) from `%USERPROFILE%\.gemini\antigravity\brain\<conversation-id>\.system_generated\logs\transcript.jsonl` via `notify` filesystem events (`ReadDirectoryChangesW`) plus configurable polling fallback.
   - Never parses undocumented internal SQLite/protobuf files and never persists conversation prompts, code, or credentials.
3. **Bounded Windows UI Automation (`uia.rs`)**:
   - Runs strictly bounded `IUIAutomation` inspections (depth/node/time capped) on visible Antigravity IDE windows to surface approval prompts (`Waiting`) when Electron's accessibility tree is active. Gracefully no-ops if unavailable.
4. **Opt-In Official Hook Bridge (`hooks.rs`)**:
   - Can be toggled on/off in **Settings → Monitoring → Official hook bridge (opt-in)**.
   - Registers non-gating lifecycle hooks (`SessionStart`, `PreInvocation`, `PostToolUse`, `PostInvocation`, `Stop` — never `PreToolUse`) under `"antigravity-agent-monitor"` in `%USERPROFILE%\.gemini\config\hooks.json`, invoking `<exe> --hook-ingest` to log minimal metadata to `%APPDATA%\com.antigravity.agent-monitor\hook-events.jsonl` and returning `{}` immediately.

### Status & Confidence Model
- **Confirmed** (solid dot): Observed directly from opt-in official lifecycle hooks or active UIA prompt cues.
- **Inferred** (hollow ring): Derived from `transcript.jsonl` step metadata and OS presence locks.
- **Stale / Disconnected**: Preserves the last known status while clearly marking stale or disconnected sessions—never promoting stopped or disconnected sessions to `Completed`.

---

## Prerequisites

- **Node.js** 20+ and **npm**
- **Rust** (`rustup` stable) with either:
  - **MSVC (`x86_64-pc-windows-msvc`)**: Visual Studio 2022 Build Tools ("Desktop development with C++") + Windows 10/11 SDK, **or**
  - **GNU (`x86_64-pc-windows-gnu`)**: WinLibs MinGW-w64 (`winget install BrechtSanders.WinLibs.POSIX.UCRT`)
- **Microsoft Edge WebView2 Runtime** (preinstalled on Windows 11 / modern Windows 10)

---

## Development Mode

Install dependencies:

```powershell
npm install
```

Run tests:

```powershell
# Frontend unit & component tests (Vitest)
npm test

# Rust unit & integration tests
cargo test --manifest-path src-tauri/Cargo.toml
```

Run in desktop development mode:

```powershell
npx tauri dev
```

*(If building with the WinLibs GNU toolchain instead of MSVC, ensure WinLibs `mingw64\bin` is first in `PATH` and set `$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"`, then run `npx tauri dev --target x86_64-pc-windows-gnu`.)*

---

## Build & Installation

Build the production executable and Windows NSIS installer:

```powershell
# With MSVC toolchain:
npx tauri build --bundles nsis

# Or with WinLibs GNU toolchain:
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
npx tauri build --target x86_64-pc-windows-gnu --bundles nsis
```

Build outputs are generated under `src-tauri/target/`:
- **Portable executable:** `src-tauri/target/<triple>/release/antigravity-agent-monitor.exe` (keep `WebView2Loader.dll` alongside it if running standalone)
- **Windows installer:** `src-tauri/target/<triple>/release/bundle/nsis/Agent Monitor_0.1.0_x64-setup.exe`

Run the NSIS `-setup.exe` installer to install **Agent Monitor** to your user/program directory with Start Menu shortcuts.

---

## Usage & Startup Configuration

- **Floating overlay:** Drag by the top title bar, resize from any edge, or use **Settings → Snap position** (`Top left`, `Top right`, `Bottom left`, `Bottom right`). Window position and size are persisted in `%APPDATA%\com.antigravity.agent-monitor\settings.json`.
- **Always on top:** Toggle via the pin button in the header or the system tray menu.
- **System tray & focus-safe toggle:** Closing (`-` or `Alt+F4`) hides the window to the system tray and removes it from the taskbar. Left-click the tray icon or press the global shortcut (`Ctrl+Shift+A` by default, configurable in Settings) to show/hide without stealing keyboard focus.
- **Start with Windows & minimized launch:** Enable **Start minimized to tray** and **Launch with Windows** in Settings to keep the monitor running quietly in the background across reboots.

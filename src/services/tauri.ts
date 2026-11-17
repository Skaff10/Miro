import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  BackdropSample,
  DisplayMode,
  GlassRegion,
  HookIntegrationStatus,
  MonitorSnapshot,
} from "../types/monitor";

export const SNAPSHOT_EVENT = "monitor://snapshot";
export const FULLSCREEN_EVENT = "monitor://fullscreen-changed";
export const SHORTCUT_ACTION_EVENT = "monitor://shortcut-action";
export const SET_MODE_EVENT = "monitor://set-mode";
export const BACKDROP_SAMPLE_EVENT = "monitor://backdrop-sample";

export type ShortcutAction =
  | "toggle_focus_mode"
  | "toggle_expand_collapse"
  | "peek_panel";

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function fetchSnapshot(): Promise<MonitorSnapshot> {
  return invoke<MonitorSnapshot>("get_snapshot");
}

export async function fetchSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_settings");
}

export async function saveSettings(newSettings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>("update_settings", { newSettings });
}

export async function toggleAlwaysOnTop(): Promise<boolean> {
  return invoke<boolean>("toggle_always_on_top");
}

export async function hideOverlayToTray(): Promise<void> {
  return invoke<void>("hide_overlay_to_tray");
}

export async function minimizeOverlay(): Promise<void> {
  return invoke<void>("minimize_overlay");
}

export async function setOverlayGeometry(
  mode: DisplayMode,
  width: number,
  height: number
): Promise<void> {
  return invoke<void>("set_overlay_geometry", { mode, width, height });
}

export async function syncGlassRegions(regions: GlassRegion[]): Promise<void> {
  return invoke<void>("sync_glass_regions", { regions });
}

export async function fetchBackdropSample(): Promise<BackdropSample | null> {
  return invoke<BackdropSample | null>("get_backdrop_sample");
}

export async function fetchFullscreenState(): Promise<boolean> {
  return invoke<boolean>("get_fullscreen_state");
}

export async function setWindowPreset(
  preset: "top-left" | "top-right" | "bottom-left" | "bottom-right"
): Promise<void> {
  return invoke<void>("set_window_preset", { preset });
}

export async function focusSessionWindow(
  workspace: string | null,
  title: string
): Promise<boolean> {
  return invoke<boolean>("focus_session_window", { workspace, title });
}

export async function setOverlayHeight(height: number): Promise<void> {
  return invoke<void>("set_overlay_height", { height });
}

export async function refreshMonitorNow(): Promise<void> {
  return invoke<void>("refresh_now");
}

export async function quitApplication(): Promise<void> {
  return invoke<void>("quit_app");
}

export async function fetchHookStatus(): Promise<HookIntegrationStatus> {
  return invoke<HookIntegrationStatus>("get_hook_integration_status");
}

export async function enableHookIntegration(): Promise<HookIntegrationStatus> {
  return invoke<HookIntegrationStatus>("enable_hook_integration");
}

export async function disableHookIntegration(): Promise<HookIntegrationStatus> {
  return invoke<HookIntegrationStatus>("disable_hook_integration");
}

export async function subscribeSnapshot(
  handler: (snapshot: MonitorSnapshot) => void
): Promise<UnlistenFn> {
  return listen<MonitorSnapshot>(SNAPSHOT_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function subscribeFullscreenChanged(
  handler: (isFullscreen: boolean) => void
): Promise<UnlistenFn> {
  return listen<boolean>(FULLSCREEN_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function subscribeShortcutAction(
  handler: (action: ShortcutAction) => void
): Promise<UnlistenFn> {
  return listen<ShortcutAction>(SHORTCUT_ACTION_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function subscribeSetMode(
  handler: (mode: DisplayMode) => void
): Promise<UnlistenFn> {
  return listen<DisplayMode>(SET_MODE_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function subscribeBackdropSample(
  handler: (sample: BackdropSample) => void
): Promise<UnlistenFn> {
  return listen<BackdropSample>(BACKDROP_SAMPLE_EVENT, (event) => {
    handler(event.payload);
  });
}

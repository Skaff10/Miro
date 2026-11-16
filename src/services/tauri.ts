import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSettings,
  HookIntegrationStatus,
  MonitorSnapshot,
} from "../types/monitor";

export const SNAPSHOT_EVENT = "monitor://snapshot";

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

export type AgentStatus =
  | "working"
  | "waiting"
  | "completed"
  | "idle"
  | "unknown"
  | "error";

export type StatusConfidence = "confirmed" | "inferred" | "unknown";

export type ConnectionState =
  | "connected"
  | "antigravityNotRunning"
  | "degraded"
  | "error";

export type ThemeMode = "dark" | "light";

export type ProviderMode = "auto" | "mock";

export type DisplayMode = "normal" | "compact" | "focus";

export type IndicatorPosition =
  | "top-left"
  | "top-right"
  | "bottom-left"
  | "bottom-right";

export type NotificationEventType =
  | "completed"
  | "failed"
  | "attention"
  | "started";

export interface NotificationEvent {
  id: string;
  sessionId: string;
  agentName: string;
  eventType: NotificationEventType;
  message: string;
  timestamp: number;
  read: boolean;
}

export interface AgentSession {
  id: string;
  title: string;
  workspace: string | null;
  status: AgentStatus;
  activity: string | null;
  lastActivity: number | null;
  lastStatusChange: number | null;
  source: string;
  confidence: StatusConfidence;
  stale: boolean;
  disconnected: boolean;
}

export interface MonitorSnapshot {
  sessions: AgentSession[];
  connection: ConnectionState;
  antigravityRunning: boolean;
  statusMessage: string | null;
  activeProviders: string[];
  hooksEnabled: boolean;
  updatedAt: number;
}

export interface GlassRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  radius: number;
  blur?: number;
}

export interface BackdropSample {
  r: number;
  g: number;
  b: number;
  luminance: number;
  transparencyEffectsEnabled: boolean;
}

export interface HookIntegrationStatus {
  enabled: boolean;
  configPath: string;
  eventLogPath: string;
  lastEventAt: number | null;
}

export interface AppSettings {
  alwaysOnTop: boolean;
  startMinimized: boolean;
  launchAtStartup: boolean;
  theme: ThemeMode;
  pollIntervalMs: number;
  hotkey: string;
  showIdleSessions: boolean;
  providerMode: ProviderMode;
  displayMode: DisplayMode;
  autoFullscreenFocus: boolean;
  indicatorPosition: IndicatorPosition;
  notifyOnCompleted: boolean;
  notifyOnFailed: boolean;
  notifyOnAttention: boolean;
  notifyOnStarted: boolean;
  notificationDurationSecs: number;
  notificationSound: boolean;
  focusModeHotkey: string;
  expandCollapseHotkey: string;
  peekPanelHotkey: string;
  windowX: number | null;
  windowY: number | null;
  windowWidth: number;
  windowHeight: number;
}

export const DEFAULT_SETTINGS: AppSettings = {
  alwaysOnTop: true,
  startMinimized: false,
  launchAtStartup: false,
  theme: "dark",
  pollIntervalMs: 2500,
  hotkey: "Ctrl+Shift+A",
  showIdleSessions: true,
  providerMode: "auto",
  displayMode: "normal",
  autoFullscreenFocus: true,
  indicatorPosition: "bottom-right",
  notifyOnCompleted: true,
  notifyOnFailed: true,
  notifyOnAttention: true,
  notifyOnStarted: false,
  notificationDurationSecs: 5,
  notificationSound: false,
  focusModeHotkey: "Ctrl+Alt+F",
  expandCollapseHotkey: "Ctrl+Alt+M",
  peekPanelHotkey: "Ctrl+Alt+P",
  windowX: null,
  windowY: null,
  windowWidth: 304,
  windowHeight: 112,
};

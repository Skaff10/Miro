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
  windowX: null,
  windowY: null,
  windowWidth: 240,
  windowHeight: 112,
};

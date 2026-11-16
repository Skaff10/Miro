import { create } from "zustand";
import {
  DEFAULT_SETTINGS,
  type AppSettings,
  type HookIntegrationStatus,
  type MonitorSnapshot,
} from "../types/monitor";

export type SessionFilter = "all" | "active" | "waiting";

interface MonitorStoreState {
  snapshot: MonitorSnapshot;
  settings: AppSettings;
  hookStatus: HookIntegrationStatus | null;
  filter: SessionFilter;
  settingsOpen: boolean;
  loading: boolean;
  error: string | null;

  setSnapshot: (snapshot: MonitorSnapshot) => void;
  setSettings: (settings: AppSettings) => void;
  setHookStatus: (status: HookIntegrationStatus | null) => void;
  setFilter: (filter: SessionFilter) => void;
  setSettingsOpen: (open: boolean) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

const INITIAL_SNAPSHOT: MonitorSnapshot = {
  sessions: [],
  connection: "antigravityNotRunning",
  antigravityRunning: false,
  statusMessage: "Connecting to monitor",
  activeProviders: [],
  hooksEnabled: false,
  updatedAt: 0,
};

export const useMonitorStore = create<MonitorStoreState>((set) => ({
  snapshot: INITIAL_SNAPSHOT,
  settings: DEFAULT_SETTINGS,
  hookStatus: null,
  filter: "all",
  settingsOpen: false,
  loading: true,
  error: null,

  setSnapshot: (snapshot) => set({ snapshot, loading: false, error: null }),
  setSettings: (settings) => set({ settings }),
  setHookStatus: (hookStatus) => set({ hookStatus }),
  setFilter: (filter) => set({ filter }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error, loading: false }),
}));

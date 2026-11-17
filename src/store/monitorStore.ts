import { create } from "zustand";
import {
  DEFAULT_SETTINGS,
  type AppSettings,
  type BackdropSample,
  type DisplayMode,
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
  displayMode: DisplayMode;
  preFullscreenMode: DisplayMode | null;
  isFullscreenActive: boolean;
  peeking: boolean;
  backdropSample: BackdropSample | null;
  loading: boolean;
  error: string | null;

  setSnapshot: (snapshot: MonitorSnapshot) => void;
  setSettings: (settings: AppSettings) => void;
  setHookStatus: (status: HookIntegrationStatus | null) => void;
  setFilter: (filter: SessionFilter) => void;
  setSettingsOpen: (open: boolean) => void;
  setDisplayMode: (mode: DisplayMode) => void;
  setFullscreenActive: (active: boolean) => void;
  setPeeking: (peeking: boolean) => void;
  setBackdropSample: (sample: BackdropSample | null) => void;
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
  displayMode: DEFAULT_SETTINGS.displayMode,
  preFullscreenMode: null,
  isFullscreenActive: false,
  peeking: false,
  backdropSample: null,
  loading: true,
  error: null,

  setSnapshot: (snapshot) => set({ snapshot, loading: false, error: null }),
  setSettings: (settings) => set({ settings }),
  setHookStatus: (hookStatus) => set({ hookStatus }),
  setFilter: (filter) => set({ filter }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen }),
  setDisplayMode: (displayMode) =>
    set({
      displayMode,
      peeking: false,
    }),
  setFullscreenActive: (active) =>
    set((s) => {
      if (active === s.isFullscreenActive) {
        return s;
      }
      if (active) {
        if (s.displayMode === "focus") {
          return { isFullscreenActive: true };
        }
        return {
          isFullscreenActive: true,
          preFullscreenMode: s.displayMode,
          displayMode: "focus",
          peeking: false,
        };
      }
      if (s.preFullscreenMode !== null) {
        return {
          isFullscreenActive: false,
          displayMode: s.preFullscreenMode,
          preFullscreenMode: null,
          peeking: false,
        };
      }
      return { isFullscreenActive: false };
    }),
  setPeeking: (peeking) => set({ peeking }),
  setBackdropSample: (backdropSample) => set({ backdropSample }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error, loading: false }),
}));

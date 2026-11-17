import { create } from "zustand";
import {
  DEFAULT_SETTINGS,
  type AppSettings,
  type BackdropSample,
  type DisplayMode,
  type HookIntegrationStatus,
  type MonitorSnapshot,
  type NotificationEvent,
} from "../types/monitor";

export type SessionFilter = "all" | "active" | "waiting";

const HISTORY_STORAGE_KEY = "miro.notificationHistory.v1";
const MAX_HISTORY_ITEMS = 100;

function loadPersistedHistory(): NotificationEvent[] {
  if (typeof window === "undefined" || !window.localStorage) {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(HISTORY_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.slice(0, MAX_HISTORY_ITEMS) : [];
  } catch {
    return [];
  }
}

function savePersistedHistory(items: NotificationEvent[]): void {
  if (typeof window === "undefined" || !window.localStorage) {
    return;
  }
  try {
    window.localStorage.setItem(
      HISTORY_STORAGE_KEY,
      JSON.stringify(items.slice(0, MAX_HISTORY_ITEMS))
    );
  } catch {
    // Ignore storage quota failures
  }
}

interface MonitorStoreState {
  snapshot: MonitorSnapshot;
  settings: AppSettings;
  hookStatus: HookIntegrationStatus | null;
  filter: SessionFilter;
  settingsOpen: boolean;
  historyOpen: boolean;
  displayMode: DisplayMode;
  preFullscreenMode: DisplayMode | null;
  isFullscreenActive: boolean;
  peeking: boolean;
  activeToasts: NotificationEvent[];
  notificationHistory: NotificationEvent[];
  backdropSample: BackdropSample | null;
  loading: boolean;
  error: string | null;

  setSnapshot: (snapshot: MonitorSnapshot) => void;
  setSettings: (settings: AppSettings) => void;
  setHookStatus: (status: HookIntegrationStatus | null) => void;
  setFilter: (filter: SessionFilter) => void;
  setSettingsOpen: (open: boolean) => void;
  setHistoryOpen: (open: boolean) => void;
  setDisplayMode: (mode: DisplayMode) => void;
  setFullscreenActive: (active: boolean) => void;
  setPeeking: (peeking: boolean) => void;
  setBackdropSample: (sample: BackdropSample | null) => void;
  pushNotification: (event: NotificationEvent) => void;
  dismissToast: (id: string) => void;
  markAllHistoryRead: () => void;
  clearHistory: () => void;
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
  historyOpen: false,
  displayMode: DEFAULT_SETTINGS.displayMode,
  preFullscreenMode: null,
  isFullscreenActive: false,
  peeking: false,
  activeToasts: [],
  notificationHistory: loadPersistedHistory(),
  backdropSample: null,
  loading: true,
  error: null,

  setSnapshot: (snapshot) => set({ snapshot, loading: false, error: null }),
  setSettings: (settings) => set({ settings }),
  setHookStatus: (hookStatus) => set({ hookStatus }),
  setFilter: (filter) => set({ filter }),
  setSettingsOpen: (settingsOpen) =>
    set((s) => ({
      settingsOpen,
      historyOpen: settingsOpen ? false : s.historyOpen,
    })),
  setHistoryOpen: (historyOpen) =>
    set((s) => ({
      historyOpen,
      settingsOpen: historyOpen ? false : s.settingsOpen,
    })),
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
  pushNotification: (event) =>
    set((s) => {
      const nextHistory = [event, ...s.notificationHistory].slice(
        0,
        MAX_HISTORY_ITEMS
      );
      savePersistedHistory(nextHistory);
      const nextToasts = [event, ...s.activeToasts].slice(0, 3);
      return {
        notificationHistory: nextHistory,
        activeToasts: nextToasts,
      };
    }),
  dismissToast: (id) =>
    set((s) => ({
      activeToasts: s.activeToasts.filter((t) => t.id !== id),
    })),
  markAllHistoryRead: () =>
    set((s) => {
      const next = s.notificationHistory.map((item) =>
        item.read ? item : { ...item, read: true }
      );
      savePersistedHistory(next);
      return { notificationHistory: next };
    }),
  clearHistory: () => {
    savePersistedHistory([]);
    set({ notificationHistory: [] });
  },
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error, loading: false }),
}));

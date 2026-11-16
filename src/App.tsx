import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { EmptyState } from "./components/EmptyState";
import { Header } from "./components/Header";
import { SessionCard } from "./components/SessionCard";
import { SettingsView } from "./components/SettingsView";
import {
  disableHookIntegration,
  enableHookIntegration,
  fetchHookStatus,
  fetchSettings,
  fetchSnapshot,
  focusSessionWindow,
  hideOverlayToTray,
  isTauriRuntime,
  quitApplication,
  refreshMonitorNow,
  saveSettings,
  setOverlayHeight,
  setWindowPreset,
  subscribeSnapshot,
} from "./services/tauri";
import { useMonitorStore } from "./store/monitorStore";
import type { AgentSession, AppSettings } from "./types/monitor";

export function App() {
  const {
    snapshot,
    settings,
    hookStatus,
    settingsOpen,
    error,
    setSnapshot,
    setSettings,
    setHookStatus,
    setSettingsOpen,
    setError,
  } = useMonitorStore();

  const [nowSecs, setNowSecs] = useState(() => Math.floor(Date.now() / 1000));
  const shellRef = useRef<HTMLDivElement | null>(null);
  const lastHeightRef = useRef<number>(0);

  useEffect(() => {
    const root = document.documentElement;
    if (settings.theme === "light") {
      root.classList.add("light");
      root.classList.remove("dark");
    } else {
      root.classList.add("dark");
      root.classList.remove("light");
    }
  }, [settings.theme]);

  useEffect(() => {
    const tick = () => {
      if (!document.hidden) {
        setNowSecs(Math.floor(Date.now() / 1000));
      }
    };
    const timer = window.setInterval(tick, 1000);
    document.addEventListener("visibilitychange", tick);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", tick);
    };
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: (() => void) | null = null;
    let cancelled = false;

    (async () => {
      try {
        const [initialSnap, initialSettings, hooks] = await Promise.all([
          fetchSnapshot(),
          fetchSettings(),
          fetchHookStatus(),
        ]);
        if (cancelled) return;
        setSnapshot(initialSnap);
        setSettings(initialSettings);
        setHookStatus(hooks);

        unlisten = await subscribeSnapshot((next) => {
          setSnapshot(next);
          setNowSecs(Math.floor(Date.now() / 1000));
        });
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    })();

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [setError, setHookStatus, setSettings, setSnapshot]);

  const visibleSessions = useMemo(() => {
    return snapshot.sessions.filter((session) => {
      if (!settings.showIdleSessions && session.status === "idle") {
        return false;
      }
      return true;
    });
  }, [snapshot.sessions, settings.showIdleSessions]);

  // Automatically shrink/grow overlay window height to exactly fit current content (no scroll).
  useLayoutEffect(() => {
    const el = shellRef.current;
    if (!el || !isTauriRuntime()) {
      return;
    }

    const syncHeight = () => {
      const rectH = el.getBoundingClientRect().height;
      const scrollH = el.scrollHeight;
      const measured = Math.ceil(Math.max(rectH, scrollH));
      if (measured > 0 && measured !== lastHeightRef.current) {
        lastHeightRef.current = measured;
        void setOverlayHeight(measured);
      }
    };

    syncHeight();
    const rafId = window.requestAnimationFrame(syncHeight);

    if (typeof ResizeObserver === "undefined") {
      return () => window.cancelAnimationFrame(rafId);
    }
    const observer = new ResizeObserver(syncHeight);
    observer.observe(el);
    return () => {
      window.cancelAnimationFrame(rafId);
      observer.disconnect();
    };
  }, [visibleSessions.length, settingsOpen, error, snapshot.connection]);

  const handleUpdateSettings = async (next: AppSettings) => {
    if (!isTauriRuntime()) {
      setSettings(next);
      return;
    }
    const saved = await saveSettings(next);
    setSettings(saved);
  };

  const handleEnableHooks = async () => {
    if (!isTauriRuntime()) return;
    const status = await enableHookIntegration();
    setHookStatus(status);
    await refreshMonitorNow();
  };

  const handleDisableHooks = async () => {
    if (!isTauriRuntime()) return;
    const status = await disableHookIntegration();
    setHookStatus(status);
    await refreshMonitorNow();
  };

  const handleFocusSession = async (session: AgentSession) => {
    if (!isTauriRuntime()) return;
    try {
      await focusSessionWindow(session.workspace, session.title);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div
      ref={shellRef}
      className="w-full flex flex-col bg-[var(--bg-canvas)] text-[var(--text-primary)] border border-[var(--border-hairline)] overflow-hidden"
    >
      <Header
        settingsOpen={settingsOpen}
        onToggleSettings={() => setSettingsOpen(!settingsOpen)}
        onHideToTray={() => {
          if (isTauriRuntime()) {
            void hideOverlayToTray();
          }
        }}
      />

      {error && (
        <div className="mx-2 mt-1.5 px-2.5 py-1.5 rounded-[6px] bg-[var(--bg-card)] border border-[var(--status-error)] text-[11px] text-[var(--status-error)]">
          {error}
        </div>
      )}

      <main
        className={
          settingsOpen
            ? "max-h-[420px] overflow-y-auto overlay-scroll"
            : "overflow-hidden"
        }
      >
        {settingsOpen ? (
          <SettingsView
            settings={settings}
            hookStatus={hookStatus}
            onUpdateSettings={handleUpdateSettings}
            onSnapCorner={async (preset) => {
              if (isTauriRuntime()) {
                await setWindowPreset(preset);
              }
            }}
            onEnableHooks={handleEnableHooks}
            onDisableHooks={handleDisableHooks}
            onQuit={async () => {
              if (isTauriRuntime()) {
                await quitApplication();
              }
            }}
          />
        ) : visibleSessions.length === 0 ? (
          <EmptyState
            connection={snapshot.connection}
            statusMessage={snapshot.statusMessage}
          />
        ) : (
          <div className="p-2 space-y-1.5">
            {visibleSessions.map((session) => (
              <SessionCard
                key={session.id}
                session={session}
                nowSecs={nowSecs}
                onFocusSession={handleFocusSession}
              />
            ))}
          </div>
        )}
      </main>
    </div>
  );
}

export default App;

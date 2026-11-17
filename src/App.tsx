import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { CompactIndicator } from "./components/CompactIndicator";
import { EmptyState } from "./components/EmptyState";
import { FocusIndicator } from "./components/FocusIndicator";
import { Header } from "./components/Header";
import { LiquidGlassCapsule } from "./components/LiquidGlassCapsule";
import { NotificationHistoryView } from "./components/NotificationHistoryView";
import { NotificationToast } from "./components/NotificationToast";
import { SessionCard } from "./components/SessionCard";
import { SettingsView } from "./components/SettingsView";
import {
  disableHookIntegration,
  enableHookIntegration,
  fetchBackdropSample,
  fetchFullscreenState,
  fetchHookStatus,
  fetchSettings,
  fetchSnapshot,
  focusSessionWindow,
  hideOverlayToTray,
  isTauriRuntime,
  quitApplication,
  refreshMonitorNow,
  saveSettings,
  setOverlayGeometry,
  setWindowPreset,
  subscribeBackdropSample,
  subscribeFullscreenChanged,
  subscribeSetMode,
  subscribeShortcutAction,
  subscribeSnapshot,
  syncGlassRegions,
} from "./services/tauri";
import { useMonitorStore } from "./store/monitorStore";
import type {
  AgentSession,
  AgentStatus,
  AppSettings,
  DisplayMode,
  GlassRegion,
  NotificationEvent,
  NotificationEventType,
} from "./types/monitor";
import { playNotificationSound } from "./utils/sound";
import { basenameFromPath } from "./utils/time";

function getLayoutBox(node: HTMLElement): { x: number; y: number; width: number; height: number } {
  let x = 0;
  let y = 0;
  let cur: HTMLElement | null = node;
  while (cur) {
    x += cur.offsetLeft - cur.scrollLeft;
    y += cur.offsetTop - cur.scrollTop;
    cur = cur.offsetParent as HTMLElement | null;
  }
  if (node.offsetWidth > 0 && node.offsetHeight > 0) {
    return { x, y, width: node.offsetWidth, height: node.offsetHeight };
  }
  const rect = node.getBoundingClientRect();
  return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
}

export function App() {
  const {
    snapshot,
    settings,
    hookStatus,
    settingsOpen,
    historyOpen,
    displayMode,
    isFullscreenActive,
    peeking,
    activeToasts,
    notificationHistory,
    backdropSample,
    error,
    setSnapshot,
    setSettings,
    setHookStatus,
    setSettingsOpen,
    setHistoryOpen,
    setDisplayMode,
    setFullscreenActive,
    setPeeking,
    pushNotification,
    dismissToast,
    markAllHistoryRead,
    clearHistory,
    setBackdropSample,
    setError,
  } = useMonitorStore();

  const [nowSecs, setNowSecs] = useState(() => Math.floor(Date.now() / 1000));
  const shellRef = useRef<HTMLDivElement | null>(null);
  const lastGeomRef = useRef<string>("");
  const lastRegionsRef = useRef<string>("");
  const prevStatusesRef = useRef<Map<string, AgentStatus>>(new Map());
  const initializedRef = useRef<boolean>(false);
  const hoveringPeekRef = useRef<boolean>(false);
  const lastNonFocusModeRef = useRef<DisplayMode>("normal");
  const prevEffectiveModeRef = useRef<DisplayMode | null>(null);

  const effectiveMode: DisplayMode = peeking ? "normal" : displayMode;
  const transitionFrom =
    prevEffectiveModeRef.current !== effectiveMode
      ? prevEffectiveModeRef.current
      : null;

  useEffect(() => {
    prevEffectiveModeRef.current = effectiveMode;
  }, [effectiveMode]);

  useEffect(() => {
    if (displayMode !== "focus") {
      lastNonFocusModeRef.current = displayMode;
    }
  }, [displayMode]);

  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-glass-host", isTauriRuntime() ? "tauri" : "browser");
    if (settings.theme === "light") {
      root.classList.add("light");
      root.classList.remove("dark");
    } else {
      root.classList.add("dark");
      root.classList.remove("light");
    }
  }, [settings.theme]);

  useEffect(() => {
    const root = document.documentElement;
    if (!backdropSample) {
      root.removeAttribute("data-backdrop-tone");
      root.removeAttribute("data-reduce-transparency");
      return;
    }

    const tone =
      backdropSample.luminance >= 0.62
        ? "bright"
        : backdropSample.luminance >= 0.28
          ? "balanced"
          : "dark";

    root.setAttribute("data-backdrop-tone", tone);
    root.setAttribute(
      "data-reduce-transparency",
      backdropSample.transparencyEffectsEnabled ? "false" : "true"
    );
    root.style.setProperty("--miro-ambient-r", String(Math.round(backdropSample.r)));
    root.style.setProperty("--miro-ambient-g", String(Math.round(backdropSample.g)));
    root.style.setProperty("--miro-ambient-b", String(Math.round(backdropSample.b)));
    root.style.setProperty(
      "--miro-ambient-luma",
      backdropSample.luminance.toFixed(3)
    );
  }, [backdropSample]);

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

  // Auto-collapse temporary peek after 6 seconds when not hovering
  useEffect(() => {
    if (!peeking) return;
    const timer = window.setInterval(() => {
      if (!hoveringPeekRef.current) {
        setPeeking(false);
      }
    }, 6000);
    return () => window.clearInterval(timer);
  }, [peeking, setPeeking]);

  // Helper to evaluate state transitions and emit notifications
  const processSnapshotTransitions = (nextSessions: AgentSession[]) => {
    const currentSettings = useMonitorStore.getState().settings;
    const prevMap = prevStatusesRef.current;

    if (!initializedRef.current) {
      for (const s of nextSessions) {
        prevMap.set(s.id, s.status);
      }
      initializedRef.current = true;
      return;
    }

    const now = Math.floor(Date.now() / 1000);
    const nextMap = new Map<string, AgentStatus>();

    for (const s of nextSessions) {
      nextMap.set(s.id, s.status);
      if (s.disconnected) continue;

      const prev = prevMap.get(s.id);
      if (prev === s.status) continue;

      let eventType: NotificationEventType | null = null;
      let defaultMsg = "";

      if (s.status === "completed" && currentSettings.notifyOnCompleted) {
        eventType = "completed";
        defaultMsg = s.activity ?? "Task completed";
      } else if (s.status === "error" && currentSettings.notifyOnFailed) {
        eventType = "failed";
        defaultMsg = s.activity ?? "Task failed";
      } else if (s.status === "waiting" && currentSettings.notifyOnAttention) {
        eventType = "attention";
        defaultMsg = s.activity ?? "Requires your attention";
      } else if (s.status === "working" && currentSettings.notifyOnStarted) {
        eventType = "started";
        defaultMsg = s.activity ?? "Task started";
      }

      if (eventType) {
        const agentName =
          basenameFromPath(s.workspace) ?? basenameFromPath(s.title) ?? s.title;
        const evId = `${s.id}-${eventType}-${now}-${Math.random()
          .toString(36)
          .slice(2, 6)}`;
        pushNotification({
          id: evId,
          sessionId: s.id,
          agentName,
          eventType,
          message: defaultMsg,
          timestamp: now,
          read: false,
        });
        if (currentSettings.notificationSound) {
          playNotificationSound(eventType);
        }
        if (currentSettings.notificationDurationSecs > 0) {
          window.setTimeout(() => {
            useMonitorStore.getState().dismissToast(evId);
          }, currentSettings.notificationDurationSecs * 1000);
        }
      }
    }

    prevStatusesRef.current = nextMap;
  };

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlistenSnap: (() => void) | null = null;
    let unlistenFs: (() => void) | null = null;
    let unlistenShortcut: (() => void) | null = null;
    let unlistenMode: (() => void) | null = null;
    let unlistenBackdrop: (() => void) | null = null;
    let cancelled = false;

    (async () => {
      try {
        const [initialSnap, initialSettings, hooks, isFs, initialBackdrop] =
          await Promise.all([
            fetchSnapshot(),
            fetchSettings(),
            fetchHookStatus(),
            fetchFullscreenState(),
            fetchBackdropSample(),
          ]);
        if (cancelled) return;
        setSnapshot(initialSnap);
        processSnapshotTransitions(initialSnap.sessions);
        setSettings(initialSettings);
        setHookStatus(hooks);
        setDisplayMode(initialSettings.displayMode);
        if (initialBackdrop) {
          setBackdropSample(initialBackdrop);
        }
        if (initialSettings.autoFullscreenFocus && isFs) {
          setFullscreenActive(true);
        }

        unlistenSnap = await subscribeSnapshot((next) => {
          setSnapshot(next);
          processSnapshotTransitions(next.sessions);
          setNowSecs(Math.floor(Date.now() / 1000));
        });

        unlistenFs = await subscribeFullscreenChanged((fsActive) => {
          const st = useMonitorStore.getState();
          if (st.settings.autoFullscreenFocus) {
            st.setFullscreenActive(fsActive);
          }
        });

        unlistenMode = await subscribeSetMode((mode) => {
          useMonitorStore.getState().setDisplayMode(mode);
        });

        unlistenBackdrop = await subscribeBackdropSample((sample) => {
          useMonitorStore.getState().setBackdropSample(sample);
        });

        unlistenShortcut = await subscribeShortcutAction((action) => {
          const st = useMonitorStore.getState();
          if (action === "toggle_focus_mode") {
            if (st.displayMode === "focus") {
              st.setDisplayMode(lastNonFocusModeRef.current || "normal");
            } else {
              st.setDisplayMode("focus");
            }
          } else if (action === "toggle_expand_collapse") {
            if (st.peeking || st.displayMode === "normal") {
              st.setPeeking(false);
              st.setDisplayMode(st.isFullscreenActive ? "focus" : "compact");
            } else {
              st.setDisplayMode("normal");
            }
          } else if (action === "peek_panel") {
            if (st.displayMode !== "normal") {
              st.setPeeking(!st.peeking);
            }
          }
        });
      } catch (err) {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    })();

    return () => {
      cancelled = true;
      unlistenSnap?.();
      unlistenFs?.();
      unlistenShortcut?.();
      unlistenMode?.();
      unlistenBackdrop?.();
    };
  }, []);

  const visibleSessions = useMemo(() => {
    return snapshot.sessions.filter((session) => {
      if (!settings.showIdleSessions && session.status === "idle") {
        return false;
      }
      return true;
    });
  }, [snapshot.sessions, settings.showIdleSessions]);

  const unreadCount = useMemo(
    () => notificationHistory.filter((n) => !n.read).length,
    [notificationHistory]
  );

  const unreadAttentionOrError = useMemo(
    () =>
      notificationHistory.some(
        (n) => !n.read && (n.eventType === "failed" || n.eventType === "attention")
      ),
    [notificationHistory]
  );

  // Sync overlay window dimensions, corner positioning, and native DWM Liquid Glass regions across all modes
  useLayoutEffect(() => {
    const el = shellRef.current;
    if (!el || !isTauriRuntime()) {
      return;
    }

    const collectRegions = (): GlassRegion[] => {
      const nodes = el.querySelectorAll<HTMLElement>('[data-glass-backdrop="true"]');
      const regions: GlassRegion[] = [];
      nodes.forEach((node) => {
        const box = getLayoutBox(node);
        if (box.width < 1 || box.height < 1) {
          return;
        }
        const style = window.getComputedStyle(node);
        const rawRadius = Number.parseFloat(style.borderTopLeftRadius || "0");
        const maxRadius = Math.min(box.width, box.height) * 0.5;
        const radius = Number.isFinite(rawRadius)
          ? Math.min(Math.max(rawRadius, 0), maxRadius)
          : 0;
        const rawBlur = Number(node.dataset.glassBlur);
        const blur = Number.isFinite(rawBlur) && rawBlur > 0 ? rawBlur : undefined;
        regions.push({
          x: Math.round(box.x * 100) / 100,
          y: Math.round(box.y * 100) / 100,
          width: Math.round(box.width * 100) / 100,
          height: Math.round(box.height * 100) / 100,
          radius: Math.round(radius * 100) / 100,
          blur,
        });
      });
      return regions;
    };

    const syncGeometry = () => {
      let targetW = 304;
      let targetH = 128;

      if (effectiveMode === "normal") {
        const scrollH = el.scrollHeight;
        const offsetH = el.offsetHeight;
        targetW = 304;
        targetH = Math.ceil(Math.max(offsetH, scrollH, 80));
      } else if (effectiveMode === "compact") {
        if (activeToasts.length > 0) {
          targetW = 252;
          targetH = Math.ceil(Math.max(el.scrollHeight, el.offsetHeight, 48));
        } else {
          targetW = 180;
          targetH = 48;
        }
      } else {
        // Focus mode
        if (activeToasts.length > 0) {
          targetW = 252;
          targetH = Math.ceil(Math.max(el.scrollHeight, el.offsetHeight, 48));
        } else {
          targetW = 64;
          targetH = 38;
        }
      }

      const key = `${effectiveMode}:${settings.indicatorPosition}:${targetW}x${targetH}`;
      const geomChanged = key !== lastGeomRef.current;
      const regions = collectRegions();
      const regionsKey = `${effectiveMode}:${JSON.stringify(regions)}`;
      const regionsChanged = regionsKey !== lastRegionsRef.current;

      if (geomChanged) {
        lastGeomRef.current = key;
        lastRegionsRef.current = regionsKey;
        void setOverlayGeometry(effectiveMode, targetW, targetH).then(() => {
          void syncGlassRegions(regions);
        });
      } else if (regionsChanged) {
        lastRegionsRef.current = regionsKey;
        void syncGlassRegions(regions);
      }
    };

    syncGeometry();
    const rafId = window.requestAnimationFrame(syncGeometry);
    const settleId = window.setTimeout(syncGeometry, 340);

    if (typeof ResizeObserver === "undefined") {
      return () => {
        window.cancelAnimationFrame(rafId);
        window.clearTimeout(settleId);
      };
    }
    const observer = new ResizeObserver(syncGeometry);
    observer.observe(el);
    el.querySelectorAll<HTMLElement>('[data-glass-backdrop="true"]').forEach((node) => {
      observer.observe(node);
    });
    return () => {
      window.cancelAnimationFrame(rafId);
      window.clearTimeout(settleId);
      observer.disconnect();
    };
  }, [
    effectiveMode,
    settings.indicatorPosition,
    visibleSessions.length,
    activeToasts.length,
    settingsOpen,
    historyOpen,
    peeking,
    error,
    snapshot.connection,
  ]);

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

  const handleExpandToNormal = () => {
    setPeeking(false);
    setDisplayMode("normal");
  };

  const handleCycleMode = () => {
    const next: DisplayMode =
      displayMode === "normal"
        ? "compact"
        : displayMode === "compact"
          ? "focus"
          : "normal";
    setDisplayMode(next);
  };

  const handleSelectHistoryEvent = async (ev: NotificationEvent) => {
    markAllHistoryRead();
    const matching = snapshot.sessions.find((s) => s.id === ev.sessionId);
    if (matching && isTauriRuntime()) {
      await handleFocusSession(matching);
    }
    setHistoryOpen(false);
  };

  const alignHorizontal = settings.indicatorPosition.endsWith("left")
    ? "items-start"
    : "items-end";
  const anchorTop = settings.indicatorPosition.startsWith("top");

  if (effectiveMode === "focus") {
    const focusNode = (
      <div className="pointer-events-auto">
        <FocusIndicator
          sessions={visibleSessions}
          connection={snapshot.connection}
          unreadAttentionOrError={unreadAttentionOrError}
          transitionFrom={transitionFrom}
          indicatorPosition={settings.indicatorPosition}
          onExpand={handleExpandToNormal}
        />
      </div>
    );
    const toastNode = activeToasts.length > 0 && (
      <div className="w-[240px] pointer-events-auto">
        <NotificationToast
          toasts={activeToasts}
          onExpand={handleExpandToNormal}
          onDismiss={dismissToast}
        />
      </div>
    );

    return (
      <div
        ref={shellRef}
        className={`w-full p-[6px] flex flex-col ${alignHorizontal} gap-1.5 bg-transparent pointer-events-none`}
      >
        {anchorTop ? (
          <>
            {focusNode}
            {toastNode}
          </>
        ) : (
          <>
            {toastNode}
            {focusNode}
          </>
        )}
      </div>
    );
  }

  if (effectiveMode === "compact") {
    const compactNode = (
      <div className="pointer-events-auto">
        <CompactIndicator
          sessions={visibleSessions}
          connection={snapshot.connection}
          unreadCount={unreadCount}
          transitionFrom={transitionFrom}
          indicatorPosition={settings.indicatorPosition}
          onExpand={handleExpandToNormal}
        />
      </div>
    );
    const toastNode = activeToasts.length > 0 && (
      <div className="w-[240px] pointer-events-auto">
        <NotificationToast
          toasts={activeToasts}
          onExpand={handleExpandToNormal}
          onDismiss={dismissToast}
        />
      </div>
    );

    return (
      <div
        ref={shellRef}
        className={`w-full p-[6px] flex flex-col ${alignHorizontal} gap-1.5 bg-transparent pointer-events-none`}
      >
        {anchorTop ? (
          <>
            {compactNode}
            {toastNode}
          </>
        ) : (
          <>
            {toastNode}
            {compactNode}
          </>
        )}
      </div>
    );
  }

  return (
    <div
      ref={shellRef}
      className="w-[304px] p-[8px] flex flex-col bg-transparent text-[var(--text-primary)] select-none"
    >
      <LiquidGlassCapsule
        as="div"
        variant="panel"
        interactive={false}
        onMouseEnter={() => {
          hoveringPeekRef.current = true;
        }}
        onMouseLeave={() => {
          hoveringPeekRef.current = false;
        }}
        className="w-[288px] rounded-[20px] flex flex-col miro-morph-normal-enter"
      >
        <Header
          settingsOpen={settingsOpen}
          historyOpen={historyOpen}
          unreadCount={unreadCount}
          displayMode={displayMode}
          onToggleSettings={() => setSettingsOpen(!settingsOpen)}
          onToggleHistory={() => {
            const next = !historyOpen;
            setHistoryOpen(next);
            if (next) markAllHistoryRead();
          }}
          onCycleDisplayMode={handleCycleMode}
          onHideToTray={() => {
            if (isTauriRuntime()) {
              void hideOverlayToTray();
            }
          }}
        />

        {peeking && (
          <div className="mx-3 mb-1 px-2.5 py-1 rounded-[10px] bg-[var(--miro-control-bg)] flex items-center justify-between text-[10px] text-[var(--text-secondary)]">
            <span>
              {isFullscreenActive ? "Temporary peek (Fullscreen)" : "Temporary peek"}
            </span>
            <button
              type="button"
              onClick={() => setPeeking(false)}
              className="font-medium text-[var(--text-primary)] hover:underline cursor-pointer"
            >
              Collapse
            </button>
          </div>
        )}

        {error && (
          <div className="mx-3 mt-1 px-3 py-1.5 rounded-[12px] bg-[color-mix(in_srgb,var(--status-error)_14%,transparent)] text-[11px] text-[var(--status-error)]">
            {error}
          </div>
        )}

        <main
          className={
            settingsOpen || historyOpen
              ? "max-h-[440px] overflow-y-auto overlay-scroll"
              : "overflow-hidden"
          }
        >
          {settingsOpen ? (
            <SettingsView
              settings={settings}
              hookStatus={hookStatus}
              onUpdateSettings={handleUpdateSettings}
              onSelectDisplayMode={(mode) => setDisplayMode(mode)}
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
          ) : historyOpen ? (
            <NotificationHistoryView
              events={notificationHistory}
              nowSecs={nowSecs}
              onMarkAllRead={markAllHistoryRead}
              onClear={clearHistory}
              onSelectEvent={handleSelectHistoryEvent}
            />
          ) : visibleSessions.length === 0 ? (
            <EmptyState
              connection={snapshot.connection}
              statusMessage={snapshot.statusMessage}
            />
          ) : (
            <div className="px-3 pb-3 pt-1 space-y-2">
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
      </LiquidGlassCapsule>

      {!settingsOpen && !historyOpen && activeToasts.length > 0 && (
        <div className="w-[288px] pt-2">
          <NotificationToast
            toasts={activeToasts}
            onExpand={handleExpandToNormal}
            onDismiss={dismissToast}
          />
        </div>
      )}
    </div>
  );
}

export default App;

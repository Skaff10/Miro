import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { CompactIndicator } from "./components/CompactIndicator";
import { EmptyState } from "./components/EmptyState";
import { FocusIndicator } from "./components/FocusIndicator";
import { Header } from "./components/Header";
import { LiquidGlassCapsule } from "./components/LiquidGlassCapsule";
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
  AppSettings,
  DisplayMode,
  GlassRegion,
} from "./types/monitor";

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
    displayMode,
    isFullscreenActive,
    peeking,
    backdropSample,
    error,
    setSnapshot,
    setSettings,
    setHookStatus,
    setSettingsOpen,
    setDisplayMode,
    setFullscreenActive,
    setPeeking,
    setBackdropSample,
    setError,
  } = useMonitorStore();

  const [nowSecs, setNowSecs] = useState(() => Math.floor(Date.now() / 1000));
  const shellRef = useRef<HTMLDivElement | null>(null);
  const lastGeomRef = useRef<string>("");
  const lastRegionsRef = useRef<string>("");
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
    root.classList.add("dark");
    root.classList.remove("light");
  }, []);

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
        targetW = 180;
        targetH = 48;
      } else {
        // Focus mode
        targetW = 64;
        targetH = 38;
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
    settingsOpen,
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

  const alignHorizontal = settings.indicatorPosition.endsWith("left")
    ? "items-start"
    : "items-end";

  if (effectiveMode === "focus") {
    return (
      <div
        ref={shellRef}
        className={`w-full p-[6px] flex flex-col ${alignHorizontal} gap-1.5 bg-transparent pointer-events-none`}
      >
        <div className="pointer-events-auto">
          <FocusIndicator
            sessions={visibleSessions}
            connection={snapshot.connection}
            transitionFrom={transitionFrom}
            indicatorPosition={settings.indicatorPosition}
            onExpand={handleExpandToNormal}
          />
        </div>
      </div>
    );
  }

  if (effectiveMode === "compact") {
    return (
      <div
        ref={shellRef}
        className={`w-full p-[6px] flex flex-col ${alignHorizontal} gap-1.5 bg-transparent pointer-events-none`}
      >
        <div className="pointer-events-auto">
          <CompactIndicator
            sessions={visibleSessions}
            connection={snapshot.connection}
            transitionFrom={transitionFrom}
            indicatorPosition={settings.indicatorPosition}
            onExpand={handleExpandToNormal}
          />
        </div>
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
          displayMode={displayMode}
          onToggleSettings={() => setSettingsOpen(!settingsOpen)}
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
            settingsOpen
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
    </div>
  );
}

export default App;

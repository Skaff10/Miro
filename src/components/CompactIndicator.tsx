import { Maximize2 } from "lucide-react";
import { useEffect, useRef, useState, type PointerEvent } from "react";
import type {
  AgentSession,
  AgentStatus,
  ConnectionState,
  DisplayMode,
  IndicatorPosition,
} from "../types/monitor";
import { basenameFromPath } from "../utils/time";
import { LiquidGlassCapsule, triggerGlassBloom } from "./LiquidGlassCapsule";

interface CompactIndicatorProps {
  sessions: AgentSession[];
  connection: ConnectionState;
  transitionFrom?: DisplayMode | null;
  indicatorPosition?: IndicatorPosition;
  onExpand: () => void;
}

const ORB_COLOR: Record<AgentStatus, string> = {
  working: "var(--lg-orb-working)",
  waiting: "var(--lg-orb-waiting)",
  error: "var(--lg-orb-error)",
  completed: "var(--lg-orb-completed)",
  idle: "var(--lg-orb-idle)",
  unknown: "var(--lg-orb-unknown)",
};

export function CompactIndicator({
  sessions,
  connection,
  transitionFrom = null,
  indicatorPosition = "bottom-right",
  onExpand,
}: CompactIndicatorProps) {
  const capsuleRef = useRef<HTMLDivElement | null>(null);
  const prevStatusMapRef = useRef<Map<string, AgentStatus>>(new Map());
  const [justCompletedIds, setJustCompletedIds] = useState<Set<string>>(() => new Set());
  const [mountOrigin] = useState<DisplayMode | null>(() => transitionFrom);

  useEffect(() => {
    const prevMap = prevStatusMapRef.current;
    const newlyCompleted: string[] = [];

    for (const s of sessions) {
      const prev = prevMap.get(s.id);
      if (prev !== undefined && prev !== "completed" && s.status === "completed" && !s.disconnected) {
        newlyCompleted.push(s.id);
      }
      prevMap.set(s.id, s.status);
    }

    if (newlyCompleted.length === 0) return;

    setJustCompletedIds((curr) => {
      const next = new Set(curr);
      for (const id of newlyCompleted) next.add(id);
      return next;
    });

    const timer = window.setTimeout(() => {
      setJustCompletedIds((curr) => {
        const next = new Set(curr);
        for (const id of newlyCompleted) next.delete(id);
        return next;
      });
    }, 900);

    return () => window.clearTimeout(timer);
  }, [sessions]);

  const visibleDots = sessions.slice(0, 6);
  const overflowCount = Math.max(0, sessions.length - visibleDots.length);
  const dense = visibleDots.length >= 6 || overflowCount > 0;
  const medium = visibleDots.length === 5 && overflowCount === 0;

  const orbSizeClass = dense
    ? "w-2 h-2"
    : medium
      ? "w-[8.5px] h-[8.5px]"
      : "w-[9.5px] h-[9.5px]";
  const dotGapClass = dense || medium ? "gap-1.5" : "gap-2";

  const originClass = indicatorPosition.endsWith("left")
    ? "origin-left"
    : "origin-right";
  const morphClass =
    mountOrigin === "focus"
      ? `miro-morph-compact-focus ${originClass}`
      : mountOrigin === "normal"
        ? `miro-morph-compact-normal ${originClass}`
        : "";

  const handleInteractiveDown = (e: PointerEvent<HTMLButtonElement>) => {
    const host = capsuleRef.current;
    if (!host) return;
    host.dataset.pressed = "true";
    const bloomEl = host.querySelector<HTMLElement>("[data-glass-bloom]");
    triggerGlassBloom(bloomEl, host, e.clientX, e.clientY);
  };

  const handleInteractiveRelease = () => {
    if (capsuleRef.current) {
      capsuleRef.current.dataset.pressed = "false";
    }
  };

  return (
    <LiquidGlassCapsule
      ref={capsuleRef}
      as="div"
      variant="compact"
      interactive
      data-testid="compact-indicator"
      className={`w-[168px] h-9 flex items-center justify-between select-none ${morphClass}`}
    >
      {/* Left drag grip */}
      <div
        data-tauri-drag-region
        title="Drag compact indicator"
        className="group/drag flex items-center gap-[3.5px] pl-3.5 pr-1.5 h-full cursor-grab active:cursor-grabbing shrink-0"
      >
        <span className="w-[3px] h-[3px] rounded-full bg-[var(--text-primary)]/40 group-hover/drag:bg-[var(--text-primary)]/75 transition-colors duration-150 pointer-events-none" />
        <span className="w-[3px] h-[3px] rounded-full bg-[var(--text-primary)]/40 group-hover/drag:bg-[var(--text-primary)]/75 transition-colors duration-150 pointer-events-none" />
      </div>

      {/* Center agent status indicators */}
      <button
        type="button"
        onClick={onExpand}
        onPointerDown={handleInteractiveDown}
        onPointerUp={handleInteractiveRelease}
        onPointerLeave={handleInteractiveRelease}
        onPointerCancel={handleInteractiveRelease}
        onBlur={handleInteractiveRelease}
        aria-label="Expand monitor panel"
        title="Click to expand monitor"
        className={`miro-compact-chrome flex-1 min-w-0 h-full px-1 flex items-center justify-center ${dotGapClass} cursor-pointer focus:outline-none`}
      >
        {visibleDots.length === 0 ? (
          <span
            data-status={connection === "antigravityNotRunning" ? "unknown" : "idle"}
            className="miro-status-orb w-2 h-2"
            style={{
              backgroundColor:
                connection === "antigravityNotRunning"
                  ? "var(--lg-orb-unknown)"
                  : "var(--lg-orb-idle)",
            }}
            title={
              connection === "antigravityNotRunning"
                ? "Antigravity IDE is not running"
                : "No active sessions"
            }
          />
        ) : (
          visibleDots.map((session) => {
            const name =
              basenameFromPath(session.workspace) ??
              basenameFromPath(session.title) ??
              session.title;
            const orbColor = ORB_COLOR[session.status];
            return (
              <span
                key={session.id}
                data-status={session.status}
                data-disconnected={session.disconnected ? "true" : "false"}
                data-just-completed={justCompletedIds.has(session.id) ? "true" : "false"}
                data-interactive="true"
                title={`${name} — ${session.status}${
                  session.activity ? `: ${session.activity}` : ""
                }`}
                className={`miro-status-orb ${orbSizeClass}`}
                style={{ backgroundColor: orbColor }}
              />
            );
          })
        )}

        {overflowCount > 0 && (
          <span className="font-mono-meta text-[9.5px] leading-none text-[var(--text-secondary)]">
            +{overflowCount}
          </span>
        )}
      </button>

      {/* Right expand control */}
      <button
        type="button"
        onClick={onExpand}
        onPointerDown={handleInteractiveDown}
        onPointerUp={handleInteractiveRelease}
        onPointerLeave={handleInteractiveRelease}
        onPointerCancel={handleInteractiveRelease}
        onBlur={handleInteractiveRelease}
        aria-label="Expand to normal view"
        title="Expand panel"
        className="miro-compact-chrome miro-expand-btn w-6 h-6 mr-1.5 rounded-full inline-flex items-center justify-center text-[var(--text-secondary)] shrink-0 cursor-pointer focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--border-strong)]"
      >
        <Maximize2 className="w-[10px] h-[10px]" strokeWidth={1.6} />
      </button>
    </LiquidGlassCapsule>
  );
}

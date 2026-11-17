import { useEffect, useRef, useState } from "react";
import type {
  AgentSession,
  AgentStatus,
  ConnectionState,
  DisplayMode,
  IndicatorPosition,
} from "../types/monitor";
import { LiquidGlassCapsule } from "./LiquidGlassCapsule";

interface FocusIndicatorProps {
  sessions: AgentSession[];
  connection: ConnectionState;
  unreadAttentionOrError: boolean;
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

function priorityRank(status: AgentStatus): number {
  switch (status) {
    case "error":
      return 0;
    case "waiting":
      return 1;
    case "working":
      return 2;
    case "completed":
      return 3;
    case "idle":
      return 4;
    default:
      return 5;
  }
}

export function FocusIndicator({
  sessions,
  connection,
  unreadAttentionOrError,
  transitionFrom = null,
  indicatorPosition = "bottom-right",
  onExpand,
}: FocusIndicatorProps) {
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

  const sorted = [...sessions].sort(
    (a, b) => priorityRank(a.status) - priorityRank(b.status)
  );
  const topDots = sorted.slice(0, 3);
  const crowded = topDots.length >= 3 || (topDots.length >= 2 && unreadAttentionOrError);

  const originClass = indicatorPosition.endsWith("left")
    ? "origin-left"
    : "origin-right";
  const morphClass =
    mountOrigin === "compact"
      ? `miro-morph-focus-compact ${originClass}`
      : mountOrigin === "normal"
        ? `miro-morph-focus-normal ${originClass}`
        : "";

  return (
    <LiquidGlassCapsule
      as="button"
      variant="focus"
      interactive
      data-testid="focus-indicator"
      onClick={onExpand}
      aria-label="Expand from Focus Mode"
      title="Focus Mode — click to expand"
      className={`w-[52px] h-[26px] px-2 flex items-center justify-center ${
        crowded ? "gap-[4.5px]" : "gap-[6.5px]"
      } cursor-pointer select-none focus:outline-none ${morphClass}`}
    >
      {topDots.length === 0 ? (
        <span
          data-status={connection === "antigravityNotRunning" ? "unknown" : "idle"}
          className="miro-status-orb w-2 h-2"
          style={{
            backgroundColor:
              connection === "antigravityNotRunning"
                ? "var(--lg-orb-unknown)"
                : "var(--lg-orb-idle)",
          }}
        />
      ) : (
        topDots.map((s) => (
          <span
            key={s.id}
            data-status={s.status}
            data-disconnected={s.disconnected ? "true" : "false"}
            data-just-completed={justCompletedIds.has(s.id) ? "true" : "false"}
            className="miro-status-orb w-2 h-2"
            style={{ backgroundColor: ORB_COLOR[s.status] }}
          />
        ))
      )}

      {unreadAttentionOrError && (
        <span
          aria-hidden="true"
          className="w-[5px] h-[5px] rounded-full shrink-0 bg-[var(--lg-orb-waiting)] opacity-90"
        />
      )}
    </LiquidGlassCapsule>
  );
}

import { useEffect, useRef, useState } from "react";
import type { AgentSession, AgentStatus } from "../types/monitor";
import { basenameFromPath, formatRelativeTime } from "../utils/time";
import { LiquidGlassCapsule } from "./LiquidGlassCapsule";

interface SessionCardProps {
  session: AgentSession;
  nowSecs: number;
  onFocusSession: (session: AgentSession) => void;
}

const STATUS_META: Record<
  AgentStatus,
  { label: string; color: string; orbColor: string }
> = {
  working: {
    label: "Working",
    color: "var(--status-working)",
    orbColor: "var(--lg-orb-working)",
  },
  waiting: {
    label: "Waiting",
    color: "var(--status-waiting)",
    orbColor: "var(--lg-orb-waiting)",
  },
  completed: {
    label: "Done",
    color: "var(--status-completed)",
    orbColor: "var(--lg-orb-completed)",
  },
  idle: {
    label: "Idle",
    color: "var(--status-idle)",
    orbColor: "var(--lg-orb-idle)",
  },
  unknown: {
    label: "Unknown",
    color: "var(--status-unknown)",
    orbColor: "var(--lg-orb-unknown)",
  },
  error: {
    label: "Error",
    color: "var(--status-error)",
    orbColor: "var(--lg-orb-error)",
  },
};

export function SessionCard({
  session,
  nowSecs,
  onFocusSession,
}: SessionCardProps) {
  const meta = STATUS_META[session.status];
  const displayName =
    basenameFromPath(session.workspace) ??
    basenameFromPath(session.title) ??
    session.title;
  const lastUpdate = formatRelativeTime(session.lastActivity, nowSecs);

  const prevStatusRef = useRef<AgentStatus>(session.status);
  const [flashKey, setFlashKey] = useState<number>(0);

  useEffect(() => {
    if (
      session.status === "completed" &&
      prevStatusRef.current !== "completed" &&
      !session.disconnected
    ) {
      setFlashKey((k) => k + 1);
    }
    prevStatusRef.current = session.status;
  }, [session.status, session.disconnected]);

  const isWorking = session.status === "working" && !session.disconnected;
  const isCompleted = session.status === "completed" && !session.disconnected;
  const isWaiting = session.status === "waiting" && !session.disconnected;
  const isError = session.status === "error" && !session.disconnected;

  let statusClass = "";
  if (isWorking) {
    statusClass = "card-working-glow";
  } else if (isCompleted) {
    statusClass = "card-completed-flash";
  } else if (isWaiting) {
    statusClass = "card-waiting-accent";
  } else if (isError) {
    statusClass = "card-error-accent";
  }

  return (
    <LiquidGlassCapsule
      key={isCompleted ? `done-${flashKey}-${session.lastStatusChange ?? 0}` : undefined}
      as="button"
      variant="card"
      nativeBackdrop={false}
      interactive
      data-testid={`session-card-${session.id}`}
      onClick={() => onFocusSession(session)}
      title={`Open ${displayName} in Antigravity IDE`}
      className={`w-full text-left rounded-[14px] px-3.5 py-2.5 cursor-pointer focus:outline-none ${statusClass} ${
        session.disconnected ? "opacity-65" : ""
      }`}
    >
      <div className="flex items-center justify-between gap-2.5">
        <div className="flex items-center gap-2 min-w-0">
          <span
            aria-hidden="true"
            data-status={session.status}
            data-disconnected={session.disconnected ? "true" : "false"}
            className="miro-status-orb w-2.5 h-2.5"
            style={{ backgroundColor: meta.orbColor }}
          />
          <span className="text-[13px] font-medium tracking-[-0.005em] text-[var(--text-primary)] truncate">
            {displayName}
          </span>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <span
            className="text-[10.5px] font-medium"
            style={{ color: meta.color }}
          >
            {session.disconnected ? "Disconnected" : meta.label}
          </span>
          <span className="font-mono-meta text-[10px] text-[var(--text-muted)]">
            {lastUpdate}
          </span>
        </div>
      </div>

      {session.activity && (
        <p className="mt-1.5 pl-[18px] text-[11.5px] leading-[16px] text-[var(--text-secondary)] truncate">
          {session.activity}
        </p>
      )}
    </LiquidGlassCapsule>
  );
}

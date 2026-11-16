import { useEffect, useRef, useState } from "react";
import type { AgentSession, AgentStatus } from "../types/monitor";
import { basenameFromPath, formatRelativeTime } from "../utils/time";

interface SessionCardProps {
  session: AgentSession;
  nowSecs: number;
  onFocusSession: (session: AgentSession) => void;
}

const STATUS_META: Record<AgentStatus, { label: string; color: string }> = {
  working: { label: "Working", color: "var(--status-working)" },
  waiting: { label: "Waiting", color: "var(--status-waiting)" },
  completed: { label: "Done", color: "var(--status-completed)" },
  idle: { label: "Idle", color: "var(--status-idle)" },
  unknown: { label: "Unknown", color: "var(--status-unknown)" },
  error: { label: "Error", color: "var(--status-error)" },
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

  let borderClass = "border-[var(--border-hairline)] hover:border-[var(--border-strong)]";
  if (isWorking) {
    borderClass = "card-working-glow";
  } else if (isCompleted) {
    borderClass = "card-completed-flash";
  } else if (isWaiting) {
    borderClass = "border-[#e5933a]/60";
  } else if (isError) {
    borderClass = "border-[#e5484d]/60";
  }

  return (
    <button
      key={isCompleted ? `done-${flashKey}-${session.lastStatusChange ?? 0}` : undefined}
      type="button"
      data-testid={`session-card-${session.id}`}
      onClick={() => onFocusSession(session)}
      title={`Open ${displayName} in Antigravity IDE`}
      className={`w-full text-left rounded-[8px] bg-[var(--bg-card)] border px-2.5 py-2 transition-colors duration-150 hover:bg-[var(--bg-raised)] cursor-pointer focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--border-strong)] ${borderClass} ${
        session.disconnected ? "opacity-65" : ""
      }`}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5 min-w-0">
          <span
            aria-hidden="true"
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: meta.color }}
          />
          <span className="text-[12px] font-medium text-[var(--text-primary)] truncate">
            {displayName}
          </span>
        </div>

        <div className="flex items-center gap-1.5 shrink-0">
          <span
            className="text-[10px] font-medium"
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
        <p className="mt-1 pl-3.5 text-[11px] leading-[15px] text-[var(--text-secondary)] truncate">
          {session.activity}
        </p>
      )}
    </button>
  );
}

import { AlertTriangle, BellOff, BellRing, CheckCircle2, Play, Trash2 } from "lucide-react";
import type { NotificationEvent, NotificationEventType } from "../types/monitor";
import { formatRelativeTime } from "../utils/time";
import { LiquidGlassCapsule } from "./LiquidGlassCapsule";

interface NotificationHistoryViewProps {
  events: NotificationEvent[];
  nowSecs: number;
  onMarkAllRead: () => void;
  onClear: () => void;
  onSelectEvent: (event: NotificationEvent) => void;
}

const EVENT_BADGE: Record<
  NotificationEventType,
  { label: string; color: string; Icon: typeof CheckCircle2 }
> = {
  completed: {
    label: "Completed",
    color: "var(--status-completed)",
    Icon: CheckCircle2,
  },
  failed: {
    label: "Failed",
    color: "var(--status-error)",
    Icon: AlertTriangle,
  },
  attention: {
    label: "Attention",
    color: "var(--status-waiting)",
    Icon: BellRing,
  },
  started: {
    label: "Started",
    color: "var(--status-working)",
    Icon: Play,
  },
};

export function NotificationHistoryView({
  events,
  nowSecs,
  onMarkAllRead,
  onClear,
  onSelectEvent,
}: NotificationHistoryViewProps) {
  return (
    <div
      data-testid="notification-history-view"
      className="px-3.5 pb-3.5 pt-1 space-y-2.5 text-[11.5px]"
    >
      <div className="flex items-center justify-between">
        <h2 className="text-[12px] font-medium text-[var(--text-secondary)]">
          Event history
        </h2>
        {events.length > 0 && (
          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={onMarkAllRead}
              className="miro-glass-pill-btn px-2.5 py-1 text-[10.5px] font-medium text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
            >
              Mark read
            </button>
            <button
              type="button"
              onClick={onClear}
              aria-label="Clear event history"
              title="Clear history"
              className="miro-glass-pill-btn w-6 h-6 inline-flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--status-error)] cursor-pointer"
            >
              <Trash2 className="w-3 h-3" strokeWidth={1.6} />
            </button>
          </div>
        )}
      </div>

      {events.length === 0 ? (
        <div className="miro-glass-section py-7 px-4 flex flex-col items-center justify-center text-center">
          <div className="w-8 h-8 rounded-full miro-glass-track flex items-center justify-center mb-2.5">
            <BellOff className="w-4 h-4 text-[var(--text-secondary)]" strokeWidth={1.6} />
          </div>
          <p className="text-[12px] font-medium text-[var(--text-primary)]">
            No events recorded yet
          </p>
          <p className="text-[10.5px] text-[var(--text-muted)] mt-1">
            Completions, failures, and prompts stay saved here
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {events.map((ev) => {
            const meta = EVENT_BADGE[ev.eventType];
            const Icon = meta.Icon;
            return (
              <LiquidGlassCapsule
                key={ev.id}
                as="button"
                variant="card"
                nativeBackdrop={false}
                interactive
                onClick={() => onSelectEvent(ev)}
                className={`w-full text-left rounded-[14px] px-3.5 py-2.5 cursor-pointer focus:outline-none ${
                  ev.read ? "opacity-75" : ""
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <Icon
                      className="w-3.5 h-3.5 shrink-0"
                      style={{ color: meta.color }}
                      strokeWidth={1.75}
                    />
                    <span className="font-medium text-[var(--text-primary)] truncate">
                      {ev.agentName}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span
                      className="text-[10.5px] font-medium"
                      style={{ color: meta.color }}
                    >
                      {meta.label}
                    </span>
                    <span className="font-mono-meta text-[10px] text-[var(--text-muted)]">
                      {formatRelativeTime(ev.timestamp, nowSecs)}
                    </span>
                  </div>
                </div>
                <p className="mt-1 pl-[22px] text-[10.5px] text-[var(--text-secondary)] truncate">
                  {ev.message}
                </p>
              </LiquidGlassCapsule>
            );
          })}
        </div>
      )}
    </div>
  );
}

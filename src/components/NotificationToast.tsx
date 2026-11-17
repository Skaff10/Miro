import { AlertTriangle, BellRing, CheckCircle2, Play, X } from "lucide-react";
import type { NotificationEvent, NotificationEventType } from "../types/monitor";
import { LiquidGlassCapsule } from "./LiquidGlassCapsule";

interface NotificationToastProps {
  toasts: NotificationEvent[];
  onExpand: () => void;
  onDismiss: (id: string) => void;
}

const EVENT_STYLE: Record<
  NotificationEventType,
  { accent: string; color: string; Icon: typeof CheckCircle2 }
> = {
  completed: {
    accent: "rgba(88, 166, 255, 0.28)",
    color: "var(--lg-orb-completed)",
    Icon: CheckCircle2,
  },
  failed: {
    accent: "rgba(242, 92, 102, 0.32)",
    color: "var(--lg-orb-error)",
    Icon: AlertTriangle,
  },
  attention: {
    accent: "rgba(240, 160, 75, 0.32)",
    color: "var(--lg-orb-waiting)",
    Icon: BellRing,
  },
  started: {
    accent: "rgba(61, 214, 140, 0.28)",
    color: "var(--lg-orb-working)",
    Icon: Play,
  },
};

export function NotificationToast({
  toasts,
  onExpand,
  onDismiss,
}: NotificationToastProps) {
  if (toasts.length === 0) {
    return null;
  }

  return (
    <div
      data-testid="notification-toast-stack"
      className="w-full space-y-1.5 pointer-events-auto"
    >
      {toasts.map((toast) => {
        const meta = EVENT_STYLE[toast.eventType];
        const Icon = meta.Icon;
        return (
          <LiquidGlassCapsule
            key={toast.id}
            as="div"
            variant="toast"
            interactive
            role="status"
            onClick={() => {
              onDismiss(toast.id);
              onExpand();
            }}
            className="toast-enter w-full px-3 py-2 flex items-center justify-between gap-2 cursor-pointer"
          >
            <div className="flex items-center gap-2 min-w-0">
              <Icon
                className="w-3.5 h-3.5 shrink-0"
                style={{ color: meta.color }}
                strokeWidth={1.75}
              />
              <div className="min-w-0">
                <div className="text-[11px] font-medium text-[var(--text-primary)] truncate">
                  {toast.agentName}
                </div>
                <div className="text-[10px] text-[var(--text-secondary)] truncate">
                  {toast.message}
                </div>
              </div>
            </div>
            <button
              type="button"
              aria-label="Dismiss notification"
              onClick={(e) => {
                e.stopPropagation();
                onDismiss(toast.id);
              }}
              className="miro-expand-btn w-5 h-5 rounded-full inline-flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] shrink-0 cursor-pointer"
            >
              <X className="w-3 h-3" strokeWidth={1.6} />
            </button>
          </LiquidGlassCapsule>
        );
      })}
    </div>
  );
}

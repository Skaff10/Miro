import { AlertTriangle, MonitorOff, Radio } from "lucide-react";
import type { ConnectionState } from "../types/monitor";

interface EmptyStateProps {
  connection: ConnectionState;
  statusMessage: string | null;
}

export function EmptyState({ connection, statusMessage }: EmptyStateProps) {
  if (connection === "antigravityNotRunning") {
    return (
      <div className="px-3 pb-3 pt-1">
        <div className="miro-glass-section flex flex-col items-center justify-center text-center px-4 py-6">
          <div className="w-8 h-8 rounded-full miro-glass-track flex items-center justify-center mb-2.5">
            <MonitorOff
              className="w-4 h-4 text-[var(--text-secondary)]"
              strokeWidth={1.6}
            />
          </div>
          <p className="text-[12.5px] font-medium text-[var(--text-primary)]">
            Antigravity IDE is not running
          </p>
          <p className="mt-1 text-[11px] leading-[15px] text-[var(--text-muted)]">
            Open a workspace to monitor agents
          </p>
        </div>
      </div>
    );
  }

  if (connection === "error") {
    return (
      <div className="px-3 pb-3 pt-1">
        <div className="miro-glass-section flex flex-col items-center justify-center text-center px-4 py-6">
          <div className="w-8 h-8 rounded-full miro-glass-track flex items-center justify-center mb-2.5">
            <AlertTriangle
              className="w-4 h-4 text-[var(--status-error)]"
              strokeWidth={1.6}
            />
          </div>
          <p className="text-[12.5px] font-medium text-[var(--text-primary)]">
            Monitoring unavailable
          </p>
          <p className="mt-1 text-[11px] leading-[15px] text-[var(--text-muted)]">
            {statusMessage ?? "Unable to read session state"}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="px-3 pb-3 pt-1">
      <div className="miro-glass-section flex flex-col items-center justify-center text-center px-4 py-6">
        <div className="w-8 h-8 rounded-full miro-glass-track flex items-center justify-center mb-2.5">
          <Radio
            className="w-4 h-4 text-[var(--text-secondary)]"
            strokeWidth={1.6}
          />
        </div>
        <p className="text-[12.5px] font-medium text-[var(--text-primary)]">
          No active sessions
        </p>
        <p className="mt-1 text-[11px] leading-[15px] text-[var(--text-muted)]">
          {statusMessage ?? "Start a conversation in Antigravity"}
        </p>
      </div>
    </div>
  );
}

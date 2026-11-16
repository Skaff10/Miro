import { AlertTriangle, MonitorOff, Radio } from "lucide-react";
import type { ConnectionState } from "../types/monitor";

interface EmptyStateProps {
  connection: ConnectionState;
  statusMessage: string | null;
}

export function EmptyState({ connection, statusMessage }: EmptyStateProps) {
  if (connection === "antigravityNotRunning") {
    return (
      <div className="flex flex-col items-center justify-center text-center px-3 py-4">
        <MonitorOff
          className="w-4 h-4 text-[var(--text-muted)] mb-1"
          strokeWidth={1.5}
        />
        <p className="text-[11px] font-medium text-[var(--text-primary)]">
          Antigravity IDE is not running
        </p>
        <p className="mt-0.5 text-[10px] text-[var(--text-muted)]">
          Open a workspace to monitor agents
        </p>
      </div>
    );
  }

  if (connection === "error") {
    return (
      <div className="flex flex-col items-center justify-center text-center px-3 py-4">
        <AlertTriangle
          className="w-4 h-4 text-[var(--status-error)] mb-1"
          strokeWidth={1.5}
        />
        <p className="text-[11px] font-medium text-[var(--text-primary)]">
          Monitoring unavailable
        </p>
        <p className="mt-0.5 text-[10px] text-[var(--text-muted)]">
          {statusMessage ?? "Unable to read session state"}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col items-center justify-center text-center px-3 py-4">
      <Radio
        className="w-4 h-4 text-[var(--text-muted)] mb-1"
        strokeWidth={1.5}
      />
      <p className="text-[11px] font-medium text-[var(--text-primary)]">
        No active sessions
      </p>
      <p className="mt-0.5 text-[10px] text-[var(--text-muted)]">
        {statusMessage ?? "Start a conversation in Antigravity"}
      </p>
    </div>
  );
}

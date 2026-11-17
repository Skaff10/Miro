import { EyeOff, Minus, Settings, X } from "lucide-react";
import type { DisplayMode } from "../types/monitor";

interface HeaderProps {
  settingsOpen: boolean;
  displayMode: DisplayMode;
  onToggleSettings: () => void;
  onCycleDisplayMode: () => void;
  onHideToTray: () => void;
}

export function Header({
  settingsOpen,
  displayMode,
  onToggleSettings,
  onCycleDisplayMode,
  onHideToTray,
}: HeaderProps) {
  return (
    <header className="select-none shrink-0">
      <div className="flex items-center justify-between h-10 px-3.5 pt-0.5 gap-2">
        <div
          data-tauri-drag-region
          className="flex items-center min-w-0 flex-1 h-full cursor-move"
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 16 16"
            fill="none"
            aria-label="Logo"
            className="shrink-0 pointer-events-none"
          >
            <rect
              x="1"
              y="2"
              width="14"
              height="3.5"
              rx="1.25"
              fill="var(--text-primary)"
              fillOpacity="0.88"
            />
            <rect
              x="1"
              y="6.75"
              width="10"
              height="3.5"
              rx="1.25"
              fill="var(--status-working)"
            />
            <rect
              x="1"
              y="11.5"
              width="12"
              height="3.5"
              rx="1.25"
              fill="var(--text-muted)"
            />
          </svg>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <button
            type="button"
            onClick={onCycleDisplayMode}
            aria-label="Switch display mode"
            title={`Display mode: ${displayMode} (click to switch to Compact / Focus)`}
            className="miro-expand-btn w-7 h-7 rounded-full inline-flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--border-strong)]"
          >
            <EyeOff className="w-3.5 h-3.5" strokeWidth={1.6} />
          </button>

          <button
            type="button"
            onClick={onToggleSettings}
            aria-label={settingsOpen ? "Close settings" : "Open settings"}
            title="Settings"
            className={`miro-expand-btn w-7 h-7 rounded-full inline-flex items-center justify-center cursor-pointer focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--border-strong)] ${
              settingsOpen
                ? "bg-[var(--miro-control-hover)] text-[var(--text-primary)] shadow-[inset_0_0.5px_0_rgb(255_255_255_/_0.35),inset_0_0_0_0.5px_var(--border-hairline)]"
                : "text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
            }`}
          >
            {settingsOpen ? (
              <X className="w-3.5 h-3.5" strokeWidth={1.6} />
            ) : (
              <Settings className="w-3.5 h-3.5" strokeWidth={1.6} />
            )}
          </button>

          <button
            type="button"
            onClick={onHideToTray}
            aria-label="Minimize to system tray"
            title="Minimize to tray"
            className="miro-expand-btn w-7 h-7 rounded-full inline-flex items-center justify-center text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--border-strong)]"
          >
            <Minus className="w-3.5 h-3.5" strokeWidth={1.6} />
          </button>
        </div>
      </div>
    </header>
  );
}

import { Minus, Settings, X } from "lucide-react";

interface HeaderProps {
  settingsOpen: boolean;
  onToggleSettings: () => void;
  onHideToTray: () => void;
}

export function Header({
  settingsOpen,
  onToggleSettings,
  onHideToTray,
}: HeaderProps) {
  return (
    <header className="bg-[var(--bg-chrome)] border-b border-[var(--border-hairline)] select-none shrink-0">
      <div className="flex items-center justify-between h-8 px-2.5 gap-2">
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
              rx="1"
              fill="var(--text-primary)"
              fillOpacity="0.88"
            />
            <rect
              x="1"
              y="6.75"
              width="10"
              height="3.5"
              rx="1"
              fill="var(--status-working)"
            />
            <rect
              x="1"
              y="11.5"
              width="12"
              height="3.5"
              rx="1"
              fill="var(--text-muted)"
            />
          </svg>
        </div>

        <div className="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            onClick={onToggleSettings}
            aria-label={settingsOpen ? "Close settings" : "Open settings"}
            title="Settings"
            className={`w-6 h-6 rounded-[5px] inline-flex items-center justify-center transition-colors duration-150 ${
              settingsOpen
                ? "bg-[var(--bg-raised)] text-[var(--text-primary)]"
                : "text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-raised)]"
            }`}
          >
            {settingsOpen ? (
              <X className="w-3.5 h-3.5" strokeWidth={1.5} />
            ) : (
              <Settings className="w-3.5 h-3.5" strokeWidth={1.5} />
            )}
          </button>

          <button
            type="button"
            onClick={onHideToTray}
            aria-label="Minimize to system tray"
            title="Minimize to tray"
            className="w-6 h-6 rounded-[5px] inline-flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text-primary)] hover:bg-[var(--bg-raised)] transition-colors duration-150"
          >
            <Minus className="w-3.5 h-3.5" strokeWidth={1.5} />
          </button>
        </div>
      </div>
    </header>
  );
}

import { useState } from "react";
import type {
  AppSettings,
  HookIntegrationStatus,
  ProviderMode,
  ThemeMode,
} from "../types/monitor";

interface SettingsViewProps {
  settings: AppSettings;
  hookStatus: HookIntegrationStatus | null;
  onUpdateSettings: (next: AppSettings) => Promise<void>;
  onSnapCorner: (
    preset: "top-left" | "top-right" | "bottom-left" | "bottom-right"
  ) => Promise<void>;
  onEnableHooks: () => Promise<void>;
  onDisableHooks: () => Promise<void>;
  onQuit: () => Promise<void>;
}

export function SettingsView({
  settings,
  hookStatus,
  onUpdateSettings,
  onSnapCorner,
  onEnableHooks,
  onDisableHooks,
  onQuit,
}: SettingsViewProps) {
  const [hotkeyDraft, setHotkeyDraft] = useState(settings.hotkey);
  const [savingMsg, setSavingMsg] = useState<string | null>(null);

  const patch = async (partial: Partial<AppSettings>) => {
    setSavingMsg(null);
    try {
      await onUpdateSettings({ ...settings, ...partial });
    } catch (err) {
      setSavingMsg(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="p-2.5 space-y-3 text-[11px]">
      {savingMsg && (
        <div className="px-3 py-2 rounded-[8px] bg-[var(--bg-raised)] border border-[var(--status-error)] text-[var(--status-error)]">
          {savingMsg}
        </div>
      )}

      {/* Window & Overlay */}
      <section className="space-y-2">
        <h2 className="text-[12px] font-medium text-[var(--text-secondary)]">
          Overlay window
        </h2>
        <div className="rounded-[10px] bg-[var(--bg-card)] border border-[var(--border-hairline)] divide-y divide-[var(--border-hairline)]">
          <ToggleRow
            label="Always on top"
            checked={settings.alwaysOnTop}
            onChange={(v) => patch({ alwaysOnTop: v })}
          />
          <ToggleRow
            label="Start minimized to tray"
            checked={settings.startMinimized}
            onChange={(v) => patch({ startMinimized: v })}
          />
          <ToggleRow
            label="Launch with Windows"
            checked={settings.launchAtStartup}
            onChange={(v) => patch({ launchAtStartup: v })}
          />

          <div className="p-3 space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-[var(--text-primary)]">Snap position</span>
            </div>
            <div className="grid grid-cols-2 gap-1.5">
              {(
                [
                  { id: "top-left", label: "Top left" },
                  { id: "top-right", label: "Top right" },
                  { id: "bottom-left", label: "Bottom left" },
                  { id: "bottom-right", label: "Bottom right" },
                ] as const
              ).map((corner) => (
                <button
                  key={corner.id}
                  type="button"
                  onClick={() => onSnapCorner(corner.id)}
                  className="px-2.5 py-1.5 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] text-[var(--text-secondary)] hover:text-[var(--text-primary)] hover:border-[var(--border-strong)] transition-colors duration-150"
                >
                  {corner.label}
                </button>
              ))}
            </div>
          </div>

          <div className="p-2.5 flex flex-wrap items-center justify-between gap-2">
            <span className="text-[var(--text-primary)]">Toggle shortcut</span>
            <div className="flex items-center gap-1.5">
              <input
                type="text"
                aria-label="Global toggle shortcut"
                value={hotkeyDraft}
                onChange={(e) => setHotkeyDraft(e.target.value)}
                className="w-24 px-2 py-1 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] font-mono-meta text-[11px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--border-strong)]"
              />
              <button
                type="button"
                onClick={() => patch({ hotkey: hotkeyDraft })}
                className="px-2 py-1 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] text-[11px] text-[var(--text-primary)] hover:border-[var(--border-strong)] transition-colors duration-150"
              >
                Apply
              </button>
            </div>
          </div>
        </div>
      </section>

      {/* Appearance & Display */}
      <section className="space-y-2">
        <h2 className="text-[12px] font-medium text-[var(--text-secondary)]">
          Display
        </h2>
        <div className="rounded-[10px] bg-[var(--bg-card)] border border-[var(--border-hairline)] divide-y divide-[var(--border-hairline)]">
          <div className="px-3 py-2.5 flex items-center justify-between">
            <span className="text-[var(--text-primary)]">Theme</span>
            <div className="flex items-center p-0.5 rounded-[6px] bg-[var(--bg-canvas)] border border-[var(--border-hairline)]">
              {(["dark", "light"] as ThemeMode[]).map((mode) => (
                <button
                  key={mode}
                  type="button"
                  onClick={() => patch({ theme: mode })}
                  className={`px-2.5 py-0.5 rounded-[4px] capitalize text-[11px] transition-colors duration-150 ${
                    settings.theme === mode
                      ? "bg-[var(--bg-raised)] text-[var(--text-primary)] font-medium"
                      : "text-[var(--text-muted)] hover:text-[var(--text-secondary)]"
                  }`}
                >
                  {mode}
                </button>
              ))}
            </div>
          </div>

          <ToggleRow
            label="Show idle and closed sessions"
            checked={settings.showIdleSessions}
            onChange={(v) => patch({ showIdleSessions: v })}
          />
        </div>
      </section>

      {/* Monitoring & Hooks */}
      <section className="space-y-2">
        <h2 className="text-[12px] font-medium text-[var(--text-secondary)]">
          Monitoring
        </h2>
        <div className="rounded-[10px] bg-[var(--bg-card)] border border-[var(--border-hairline)] divide-y divide-[var(--border-hairline)]">
          <div className="px-3 py-2.5 flex items-center justify-between">
            <span className="text-[var(--text-primary)]">Polling fallback</span>
            <select
              aria-label="Polling fallback interval"
              value={settings.pollIntervalMs}
              onChange={(e) =>
                patch({ pollIntervalMs: Number(e.target.value) })
              }
              className="px-2 py-1 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] font-mono-meta text-[11px] text-[var(--text-primary)]"
            >
              <option value={1000}>1.0s</option>
              <option value={2500}>2.5s</option>
              <option value={5000}>5.0s</option>
              <option value={10000}>10.0s</option>
            </select>
          </div>

          <div className="p-3 space-y-2">
            <div className="flex items-center justify-between gap-2">
              <div>
                <div className="text-[var(--text-primary)]">
                  Official hook bridge (opt-in)
                </div>
                <div className="text-[11px] text-[var(--text-muted)] mt-0.5">
                  Registers a lightweight observer in ~/.gemini/config/hooks.json
                  for confirmed turn and tool events
                </div>
              </div>
              <button
                type="button"
                onClick={hookStatus?.enabled ? onDisableHooks : onEnableHooks}
                className="px-2.5 py-1 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] text-[11px] text-[var(--text-primary)] hover:border-[var(--border-strong)] shrink-0 transition-colors duration-150"
              >
                {hookStatus?.enabled ? "Disable" : "Enable"}
              </button>
            </div>
          </div>

          <div className="px-3 py-2.5 flex items-center justify-between">
            <span className="text-[var(--text-primary)]">Source mode</span>
            <div className="flex items-center p-0.5 rounded-[6px] bg-[var(--bg-canvas)] border border-[var(--border-hairline)]">
              {(
                [
                  { id: "auto", label: "Live" },
                  { id: "mock", label: "Mock" },
                ] as { id: ProviderMode; label: string }[]
              ).map((m) => (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => patch({ providerMode: m.id })}
                  className={`px-2.5 py-0.5 rounded-[4px] text-[11px] transition-colors duration-150 ${
                    settings.providerMode === m.id
                      ? "bg-[var(--bg-raised)] text-[var(--text-primary)] font-medium"
                      : "text-[var(--text-muted)] hover:text-[var(--text-secondary)]"
                  }`}
                >
                  {m.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      <div className="pt-1 flex justify-end">
        <button
          type="button"
          onClick={onQuit}
          className="px-3 py-1.5 rounded-[6px] bg-[var(--bg-raised)] border border-[var(--border-hairline)] text-[12px] text-[var(--status-error)] hover:border-[var(--status-error)] transition-colors duration-150"
        >
          Quit Agent Monitor
        </button>
      </div>
    </div>
  );
}

function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <label className="px-3 py-2.5 flex items-center justify-between cursor-pointer">
      <span className="text-[var(--text-primary)]">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="w-4 h-4 accent-[#ededed] cursor-pointer"
      />
    </label>
  );
}

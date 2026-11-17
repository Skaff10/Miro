import { useEffect, useState } from "react";
import type {
  AppSettings,
  DisplayMode,
  HookIntegrationStatus,
  IndicatorPosition,
  ProviderMode,
} from "../types/monitor";

interface SettingsViewProps {
  settings: AppSettings;
  hookStatus: HookIntegrationStatus | null;
  onUpdateSettings: (next: AppSettings) => Promise<void>;
  onSelectDisplayMode: (mode: DisplayMode) => void;
  onSnapCorner: (preset: IndicatorPosition) => Promise<void>;
  onEnableHooks: () => Promise<void>;
  onDisableHooks: () => Promise<void>;
  onQuit: () => Promise<void>;
}

export function SettingsView({
  settings,
  hookStatus,
  onUpdateSettings,
  onSelectDisplayMode,
  onSnapCorner,
  onEnableHooks,
  onDisableHooks,
  onQuit,
}: SettingsViewProps) {
  const [hotkeyDraft, setHotkeyDraft] = useState(settings.hotkey);
  const [focusHotkeyDraft, setFocusHotkeyDraft] = useState(
    settings.focusModeHotkey
  );
  const [expandHotkeyDraft, setExpandHotkeyDraft] = useState(
    settings.expandCollapseHotkey
  );
  const [peekHotkeyDraft, setPeekHotkeyDraft] = useState(
    settings.peekPanelHotkey
  );
  const [savingMsg, setSavingMsg] = useState<string | null>(null);

  useEffect(() => {
    setHotkeyDraft(settings.hotkey);
    setFocusHotkeyDraft(settings.focusModeHotkey);
    setExpandHotkeyDraft(settings.expandCollapseHotkey);
    setPeekHotkeyDraft(settings.peekPanelHotkey);
  }, [
    settings.hotkey,
    settings.focusModeHotkey,
    settings.expandCollapseHotkey,
    settings.peekPanelHotkey,
  ]);

  const patch = async (partial: Partial<AppSettings>) => {
    setSavingMsg(null);
    try {
      await onUpdateSettings({ ...settings, ...partial });
    } catch (err) {
      setSavingMsg(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    <div className="px-3.5 pb-3.5 pt-1 space-y-3.5 text-[11.5px]">
      {savingMsg && (
        <div className="miro-glass-section px-3 py-2 text-[var(--status-error)] shadow-[inset_0_0_0_0.5px_var(--status-error)]">
          {savingMsg}
        </div>
      )}

      {/* Focus Mode & Display */}
      <section className="space-y-1.5">
        <h2 className="px-1 text-[11px] font-medium text-[var(--text-secondary)]">
          Focus Mode & Display
        </h2>
        <div className="miro-glass-section divide-y divide-[var(--border-hairline)]">
          <div className="p-3 space-y-2">
            <span className="text-[var(--text-primary)] font-medium block">
              Display mode
            </span>
            <div className="miro-glass-track grid grid-cols-3 gap-1 p-1">
              {(
                [
                  { id: "normal", label: "Normal" },
                  { id: "compact", label: "Compact" },
                  { id: "focus", label: "Focus" },
                ] as { id: DisplayMode; label: string }[]
              ).map((m) => (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => {
                    onSelectDisplayMode(m.id);
                    void patch({ displayMode: m.id });
                  }}
                  className={`px-2 py-1 rounded-full text-[11px] transition-all duration-150 cursor-pointer ${
                    settings.displayMode === m.id
                      ? "bg-[var(--miro-control-active)] text-[var(--text-primary)] font-medium shadow-[inset_0_1px_0_rgb(255_255_255_/_0.38),0_1px_3px_rgb(0_0_0_/_0.18)]"
                      : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
                  }`}
                >
                  {m.label}
                </button>
              ))}
            </div>
          </div>

          <div className="p-3 space-y-1.5">
            <ToggleRowInline
              label="Auto Focus Mode in fullscreen"
              checked={settings.autoFullscreenFocus}
              onChange={(v) => patch({ autoFullscreenFocus: v })}
            />
            <p className="text-[10.5px] leading-[14px] text-[var(--text-muted)]">
              Switches to Focus Mode for borderless games and fullscreen video.
              Exclusive DirectX/Vulkan fullscreen bypasses desktop overlays.
            </p>
          </div>

          <div className="px-3 py-2.5 flex items-center justify-between gap-2">
            <span className="text-[var(--text-primary)]">
              Indicator position
            </span>
            <select
              aria-label="Indicator position"
              value={settings.indicatorPosition}
              onChange={(e) =>
                patch({
                  indicatorPosition: e.target.value as IndicatorPosition,
                })
              }
              className="px-2.5 py-1 rounded-full bg-[var(--miro-control-bg)] border border-[var(--border-hairline)] text-[11px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--border-strong)] cursor-pointer"
            >
              <option value="top-left" className="bg-[#1e2026] text-[#f5f6f8]">
                Top left
              </option>
              <option value="top-right" className="bg-[#1e2026] text-[#f5f6f8]">
                Top right
              </option>
              <option value="bottom-left" className="bg-[#1e2026] text-[#f5f6f8]">
                Bottom left
              </option>
              <option value="bottom-right" className="bg-[#1e2026] text-[#f5f6f8]">
                Bottom right
              </option>
            </select>
          </div>

          <ToggleRow
            label="Show idle sessions"
            checked={settings.showIdleSessions}
            onChange={(v) => patch({ showIdleSessions: v })}
          />
        </div>
      </section>

      {/* Window & Overlay */}
      <section className="space-y-1.5">
        <h2 className="px-1 text-[11px] font-medium text-[var(--text-secondary)]">
          Overlay window
        </h2>
        <div className="miro-glass-section divide-y divide-[var(--border-hairline)]">
          <ToggleRow
            label="Always on top"
            checked={settings.alwaysOnTop}
            onChange={(v) => patch({ alwaysOnTop: v })}
          />
          <ToggleRow
            label="Start minimized on boot"
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
                  className="miro-glass-pill-btn px-2.5 py-1.5 text-[11px] text-[var(--text-secondary)] hover:text-[var(--text-primary)] cursor-pointer"
                >
                  {corner.label}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      {/* Keyboard Shortcuts */}
      <section className="space-y-1.5">
        <h2 className="px-1 text-[11px] font-medium text-[var(--text-secondary)]">
          Keyboard shortcuts
        </h2>
        <div className="miro-glass-section p-3 space-y-2">
          <ShortcutField
            label="Show / Hide overlay"
            ariaLabel="Global toggle shortcut"
            value={hotkeyDraft}
            onChange={setHotkeyDraft}
          />
          <ShortcutField
            label="Toggle Focus Mode"
            ariaLabel="Toggle Focus Mode shortcut"
            value={focusHotkeyDraft}
            onChange={setFocusHotkeyDraft}
          />
          <ShortcutField
            label="Expand / Collapse"
            ariaLabel="Expand or collapse panel shortcut"
            value={expandHotkeyDraft}
            onChange={setExpandHotkeyDraft}
          />
          <ShortcutField
            label="Temporary reveal"
            ariaLabel="Temporarily reveal panel shortcut"
            value={peekHotkeyDraft}
            onChange={setPeekHotkeyDraft}
          />
          <div className="pt-1 flex justify-end">
            <button
              type="button"
              onClick={() =>
                patch({
                  hotkey: hotkeyDraft,
                  focusModeHotkey: focusHotkeyDraft,
                  expandCollapseHotkey: expandHotkeyDraft,
                  peekPanelHotkey: peekHotkeyDraft,
                })
              }
              className="miro-glass-pill-btn px-3 py-1 text-[11px] font-medium text-[var(--text-primary)] cursor-pointer"
            >
              Apply shortcuts
            </button>
          </div>
        </div>
      </section>

      {/* Monitoring & Hooks */}
      <section className="space-y-1.5">
        <h2 className="px-1 text-[11px] font-medium text-[var(--text-secondary)]">
          Monitoring
        </h2>
        <div className="miro-glass-section divide-y divide-[var(--border-hairline)]">
          <div className="px-3 py-2.5 flex items-center justify-between">
            <span className="text-[var(--text-primary)]">Polling fallback</span>
            <select
              aria-label="Polling fallback interval"
              value={settings.pollIntervalMs}
              onChange={(e) =>
                patch({ pollIntervalMs: Number(e.target.value) })
              }
              className="px-2.5 py-1 rounded-full bg-[var(--miro-control-bg)] border border-[var(--border-hairline)] font-mono-meta text-[11px] text-[var(--text-primary)] focus:outline-none focus:border-[var(--border-strong)] cursor-pointer"
            >
              <option value={1000} className="bg-[#1e2026] text-[#f5f6f8]">
                1.0s
              </option>
              <option value={2500} className="bg-[#1e2026] text-[#f5f6f8]">
                2.5s
              </option>
              <option value={5000} className="bg-[#1e2026] text-[#f5f6f8]">
                5.0s
              </option>
              <option value={10000} className="bg-[#1e2026] text-[#f5f6f8]">
                10.0s
              </option>
            </select>
          </div>

          <div className="p-3 space-y-2">
            <div className="flex items-center justify-between gap-2.5">
              <div>
                <div className="text-[var(--text-primary)]">
                  Official hook bridge (opt-in)
                </div>
                <div className="text-[10.5px] leading-[14px] text-[var(--text-muted)] mt-0.5">
                  Registers a lightweight observer in ~/.gemini/config/hooks.json
                  for confirmed turn and tool events
                </div>
              </div>
              <button
                type="button"
                onClick={hookStatus?.enabled ? onDisableHooks : onEnableHooks}
                className="miro-glass-pill-btn px-3 py-1 text-[11px] font-medium text-[var(--text-primary)] shrink-0 cursor-pointer"
              >
                {hookStatus?.enabled ? "Disable" : "Enable"}
              </button>
            </div>
          </div>

          <div className="px-3 py-2.5 flex items-center justify-between">
            <span className="text-[var(--text-primary)]">Source mode</span>
            <div className="miro-glass-track flex items-center p-0.5 gap-0.5">
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
                  className={`px-2.5 py-0.5 rounded-full text-[11px] transition-all duration-150 cursor-pointer ${
                    settings.providerMode === m.id
                      ? "bg-[var(--miro-control-active)] text-[var(--text-primary)] font-medium shadow-[inset_0_1px_0_rgb(255_255_255_/_0.38),0_1px_2px_rgb(0_0_0_/_0.16)]"
                      : "text-[var(--text-muted)] hover:text-[var(--text-primary)]"
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
          className="miro-glass-pill-btn px-3.5 py-1.5 text-[11.5px] font-medium text-[var(--status-error)] cursor-pointer"
        >
          Quit Agent Monitor
        </button>
      </div>
    </div>
  );
}

function ShortcutField({
  label,
  ariaLabel,
  value,
  onChange,
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange: (next: string) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-2">
      <span className="text-[var(--text-secondary)] truncate">{label}</span>
      <input
        type="text"
        aria-label={ariaLabel}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-24 px-2.5 py-1 rounded-full bg-[var(--miro-control-bg)] border border-[var(--border-hairline)] font-mono-meta text-[10px] text-[var(--text-primary)] text-center focus:outline-none focus:border-[var(--border-strong)]"
      />
    </div>
  );
}

function GlassSwitchControl({ checked }: { checked: boolean }) {
  return (
    <span
      aria-hidden="true"
      className="relative inline-flex items-center w-8 h-[18px] rounded-full transition-colors duration-200 shrink-0"
      style={{
        backgroundColor: checked ? "var(--miro-switch-on)" : "var(--miro-switch-off)",
        boxShadow:
          "inset 0 1px 2px rgba(0,0,0,0.22), inset 0 0 0 0.5px rgba(255,255,255,0.2)",
      }}
    >
      <span
        className="w-[14px] h-[14px] rounded-full bg-white shadow-[0_1px_3px_rgba(0,0,0,0.28)] transition-transform duration-200"
        style={{
          transitionTimingFunction: "var(--lg-spring)",
          transform: checked ? "translateX(16px)" : "translateX(2px)",
        }}
      />
    </span>
  );
}

function ToggleRowInline({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3 cursor-pointer">
      <span className="text-[var(--text-primary)]">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="sr-only"
      />
      <GlassSwitchControl checked={checked} />
    </label>
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
    <label className="px-3 py-2.5 flex items-center justify-between gap-3 cursor-pointer">
      <span className="text-[var(--text-primary)]">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="sr-only"
      />
      <GlassSwitchControl checked={checked} />
    </label>
  );
}

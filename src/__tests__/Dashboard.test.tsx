import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import App from "../App";
import { useMonitorStore } from "../store/monitorStore";
import { DEFAULT_SETTINGS, type AgentSession } from "../types/monitor";

const nowSecs = Math.floor(Date.now() / 1000);

const SAMPLE_SESSIONS: AgentSession[] = [
  {
    id: "sess-1",
    title: "MIRO",
    workspace: "d:\\MIRO",
    status: "working",
    activity: "Executing terminal command",
    lastActivity: nowSecs - 12,
    lastStatusChange: nowSecs - 45,
    source: "hooks",
    confidence: "confirmed",
    stale: false,
    disconnected: false,
  },
  {
    id: "sess-2",
    title: "FTune",
    workspace: "d:\\FTune",
    status: "waiting",
    activity: "Waiting for permission",
    lastActivity: nowSecs - 120,
    lastStatusChange: nowSecs - 120,
    source: "transcript",
    confidence: "inferred",
    stale: false,
    disconnected: false,
  },
  {
    id: "sess-3",
    title: "Database",
    workspace: "d:\\repos\\Database",
    status: "completed",
    activity: "Turn finished",
    lastActivity: nowSecs - 300,
    lastStatusChange: nowSecs - 300,
    source: "hooks",
    confidence: "confirmed",
    stale: false,
    disconnected: false,
  },
];

describe("MIRO Compact Overlay Dashboard", () => {
  beforeEach(() => {
    useMonitorStore.setState({
      snapshot: {
        sessions: [],
        connection: "antigravityNotRunning",
        antigravityRunning: false,
        statusMessage: null,
        activeProviders: [],
        hooksEnabled: false,
        updatedAt: nowSecs,
      },
      settings: { ...DEFAULT_SETTINGS },
      hookStatus: null,
      filter: "all",
      settingsOpen: false,
      displayMode: "normal",
      preFullscreenMode: null,
      isFullscreenActive: false,
      peeking: false,
      backdropSample: null,
      loading: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders compact header with logo, settings, and minimize buttons without notification history", () => {
    render(<App />);
    expect(screen.getByLabelText("Logo")).toBeInTheDocument();
    expect(screen.queryByText("MIRO")).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /notification history/i })
    ).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open settings" })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Minimize to system tray" })
    ).toBeInTheDocument();
    expect(
      screen.getByText("Antigravity IDE is not running")
    ).toBeInTheDocument();
  });

  it("renders connected empty state when Antigravity runs with no sessions", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: [],
        connection: "connected",
        antigravityRunning: true,
        statusMessage: "Listening for agent activity",
        activeProviders: ["process", "transcript"],
        hooksEnabled: false,
        updatedAt: nowSecs,
      },
    });
    render(<App />);
    expect(screen.getByText("No active sessions")).toBeInTheDocument();
  });

  it("renders monitoring error state when connection fails", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: [],
        connection: "error",
        antigravityRunning: false,
        statusMessage: "Access denied reading transcript directory",
        activeProviders: [],
        hooksEnabled: false,
        updatedAt: nowSecs,
      },
    });
    render(<App />);
    expect(screen.getByText("Monitoring unavailable")).toBeInTheDocument();
    expect(
      screen.getByText("Access denied reading transcript directory")
    ).toBeInTheDocument();
  });

  it("renders folder names, green glow for working, blue flash for done, and omits transcript inferred text", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: SAMPLE_SESSIONS,
        connection: "connected",
        antigravityRunning: true,
        statusMessage: null,
        activeProviders: ["process", "transcript", "hooks"],
        hooksEnabled: true,
        updatedAt: nowSecs,
      },
    });
    render(<App />);

    const miroCard = screen.getByTestId("session-card-sess-1");
    const dbCard = screen.getByTestId("session-card-sess-3");

    expect(within(miroCard).getByText("MIRO")).toBeInTheDocument();
    expect(screen.getByText("FTune")).toBeInTheDocument();
    expect(screen.getByText("Database")).toBeInTheDocument();

    expect(miroCard.className).toContain("card-working-glow");
    expect(dbCard.className).toContain("card-completed-flash");

    expect(screen.queryByText(/transcript/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/inferred/i)).not.toBeInTheDocument();
  });

  it("preserves last known status on disconnect without promoting to Done", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: [
          {
            ...SAMPLE_SESSIONS[0]!,
            status: "working",
            stale: true,
            disconnected: true,
            activity: "Antigravity disconnected",
          },
        ],
        connection: "antigravityNotRunning",
        antigravityRunning: false,
        statusMessage: null,
        activeProviders: ["process"],
        hooksEnabled: false,
        updatedAt: nowSecs,
      },
    });
    render(<App />);

    expect(screen.getByText("Disconnected")).toBeInTheDocument();
    expect(screen.queryByText("Done")).not.toBeInTheDocument();
  });

  it("opens settings view from header settings button", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Open settings" }));
    expect(screen.getByText("Focus Mode & Display")).toBeInTheDocument();
    expect(screen.getByText("Overlay window")).toBeInTheDocument();
    expect(
      screen.getByText("Official hook bridge (opt-in)")
    ).toBeInTheDocument();
  });

  it("supports Compact Mode and Focus Mode with click-to-expand and fullscreen auto-switching", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: SAMPLE_SESSIONS,
        connection: "connected",
        antigravityRunning: true,
        statusMessage: null,
        activeProviders: ["process", "transcript"],
        hooksEnabled: false,
        updatedAt: nowSecs,
      },
    });
    render(<App />);

    // Cycle Normal -> Compact -> Focus
    fireEvent.click(screen.getByRole("button", { name: "Switch display mode" }));
    expect(screen.getByTestId("compact-indicator")).toBeInTheDocument();
    expect(screen.queryByTestId("session-card-sess-1")).not.toBeInTheDocument();

    // Clicking compact indicator expands back to Normal Mode
    fireEvent.click(screen.getByRole("button", { name: "Expand monitor panel" }));
    expect(screen.getByTestId("session-card-sess-1")).toBeInTheDocument();

    // Auto fullscreen switches Normal -> Focus Mode (no agent names visible) and restores Normal on exit
    act(() => {
      useMonitorStore.getState().setFullscreenActive(true);
    });
    expect(useMonitorStore.getState().displayMode).toBe("focus");
    expect(screen.getByTestId("focus-indicator")).toBeInTheDocument();
    expect(screen.queryByText("MIRO")).not.toBeInTheDocument();

    act(() => {
      useMonitorStore.getState().setFullscreenActive(false);
    });
    expect(useMonitorStore.getState().displayMode).toBe("normal");
    expect(screen.getByTestId("session-card-sess-1")).toBeInTheDocument();
  });

  it("renders single-surface liquid glass capsules with press feedback and mode morph classes", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: SAMPLE_SESSIONS.slice(0, 2),
        connection: "connected",
        antigravityRunning: true,
        statusMessage: null,
        activeProviders: ["hooks"],
        hooksEnabled: true,
        updatedAt: nowSecs,
      },
      displayMode: "compact",
    });

    render(<App />);
    const compact = screen.getByTestId("compact-indicator");
    expect(compact.className).toContain("miro-glass-capsule");
    expect(compact.getAttribute("data-variant")).toBe("compact");

    // Cycle Compact -> Focus
    act(() => {
      useMonitorStore.getState().setDisplayMode("focus");
    });

    const focus = screen.getByTestId("focus-indicator");
    expect(focus.className).toContain("miro-glass-capsule");
    expect(focus.className).toContain("miro-morph-focus-compact");
    expect(focus.getAttribute("data-variant")).toBe("focus");

    fireEvent.pointerDown(focus, { clientX: 20, clientY: 12 });
    expect(focus.getAttribute("data-pressed")).toBe("true");
    fireEvent.pointerUp(focus);
    expect(focus.getAttribute("data-pressed")).toBe("false");

    // Cycle Focus -> Compact
    act(() => {
      useMonitorStore.getState().setDisplayMode("compact");
    });
    const compactReentered = screen.getByTestId("compact-indicator");
    expect(compactReentered.className).toContain("miro-morph-compact-focus");
  });

  it("applies Liquid Glass surfaces across Normal Mode and adapts to backdrop samples", () => {
    useMonitorStore.setState({
      snapshot: {
        sessions: SAMPLE_SESSIONS.slice(0, 2),
        connection: "connected",
        antigravityRunning: true,
        statusMessage: null,
        activeProviders: ["hooks"],
        hooksEnabled: true,
        updatedAt: nowSecs,
      },
      displayMode: "normal",
    });

    const { container } = render(<App />);

    const panel = container.querySelector('.miro-glass-capsule[data-variant="panel"]');
    expect(panel).not.toBeNull();
    expect(panel?.getAttribute("data-glass-backdrop")).toBe("true");

    const card = screen.getByTestId("session-card-sess-1");
    expect(card.className).toContain("miro-glass-capsule");
    expect(card.getAttribute("data-variant")).toBe("card");
    expect(card.getAttribute("data-glass-backdrop")).toBeNull();

    act(() => {
      useMonitorStore.getState().setBackdropSample({
        r: 238,
        g: 242,
        b: 250,
        luminance: 0.88,
        transparencyEffectsEnabled: true,
      });
    });

    expect(document.documentElement.getAttribute("data-backdrop-tone")).toBe("bright");
    expect(document.documentElement.style.getPropertyValue("--miro-ambient-r")).toBe("238");

    act(() => {
      useMonitorStore.getState().setBackdropSample({
        r: 18,
        g: 24,
        b: 38,
        luminance: 0.08,
        transparencyEffectsEnabled: false,
      });
    });

    expect(document.documentElement.getAttribute("data-backdrop-tone")).toBe("dark");
    expect(document.documentElement.getAttribute("data-reduce-transparency")).toBe("true");
  });
});

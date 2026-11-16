import {
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
      loading: false,
      error: null,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders compact header with logo, settings, and minimize buttons only", () => {
    render(<App />);
    expect(screen.getByLabelText("Logo")).toBeInTheDocument();
    expect(screen.queryByText("MIRO")).not.toBeInTheDocument();
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
    expect(screen.getByText("Overlay window")).toBeInTheDocument();
    expect(
      screen.getByText("Official hook bridge (opt-in)")
    ).toBeInTheDocument();
  });
});

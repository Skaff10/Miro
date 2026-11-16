export function formatRelativeTime(timestampSecs: number | null, nowSecs: number): string {
  if (timestampSecs === null || timestampSecs <= 0) {
    return "Unknown";
  }
  const delta = Math.max(0, Math.floor(nowSecs - timestampSecs));
  if (delta < 5) {
    return "Just now";
  }
  if (delta < 60) {
    return `${delta}s ago`;
  }
  const mins = Math.floor(delta / 60);
  if (mins < 60) {
    return `${mins}m ago`;
  }
  const hours = Math.floor(mins / 60);
  if (hours < 24) {
    return `${hours}h ago`;
  }
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function formatDuration(sinceSecs: number | null, nowSecs: number): string {
  if (sinceSecs === null || sinceSecs <= 0) {
    return "--";
  }
  const delta = Math.max(0, Math.floor(nowSecs - sinceSecs));
  if (delta < 60) {
    return `${delta}s`;
  }
  const mins = Math.floor(delta / 60);
  const secs = delta % 60;
  if (mins < 60) {
    return `${mins}m ${secs.toString().padStart(2, "0")}s`;
  }
  const hours = Math.floor(mins / 60);
  const remMins = mins % 60;
  return `${hours}h ${remMins.toString().padStart(2, "0")}m`;
}

export function basenameFromPath(path: string | null): string | null {
  if (!path) {
    return null;
  }
  const trimmed = path.replace(/[\\/]+$/, "");
  if (!trimmed) {
    return null;
  }
  const parts = trimmed.split(/[\\/]/);
  return parts[parts.length - 1] ?? trimmed;
}

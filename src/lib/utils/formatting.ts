/**
 * Formatting utility functions for the dashboard
 */

export function getUsageColor(utilization: number): string {
  if (utilization >= 90) return "red";
  if (utilization >= 80) return "orange";
  if (utilization >= 50) return "yellow";
  return "green";
}

export function formatResetTime(resets_at: string): string {
  try {
    const date = new Date(resets_at);
    if (Number.isNaN(date.getTime())) {
      return "";
    }
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();
    const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
    const diffMins = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));

    if (diffHours >= 24) {
      const days = Math.floor(diffHours / 24);
      return `${days}d ${diffHours % 24}h`;
    }
    if (diffHours > 0) {
      return `${diffHours}h ${diffMins}m`;
    }
    return `${diffMins}m`;
  } catch {
    return "";
  }
}

export function formatLastUpdate(date: Date | null): string {
  if (!date) return "Never";
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);
  const diffMins = Math.floor(diffSecs / 60);

  if (diffSecs < 60) return "Just now";
  if (diffMins === 1) return "1 min ago";
  return `${diffMins} min ago`;
}

export function formatSecondsAgo(seconds: number): string {
  if (seconds < 60) return "Just now";
  const mins = Math.floor(seconds / 60);
  if (mins === 1) return "1 min ago";
  return `${mins} min ago`;
}

export function formatCountdown(seconds: number): string {
  if (seconds <= 0) return "now";
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  if (mins > 0) {
    return `${mins}m ${secs}s`;
  }
  return `${secs}s`;
}

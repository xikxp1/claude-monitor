import { commands } from "$lib/bindings.generated";

// Re-export generated types
export type { UsageHistoryRecord, MetricStats, UsageStats } from "$lib/bindings.generated";

// Time range presets (frontend-only)
export type TimeRange = "1h" | "6h" | "24h" | "7d" | "30d";

// Initialize history storage (no-op since backend handles it)
export function initHistoryStorage(): void {
  // Database is initialized by Rust backend
}

// Get usage history for a specific time range preset
export async function getUsageHistoryByRange(range: TimeRange) {
  const result = await commands.getUsageHistoryByRange(range);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

// Get usage statistics for a time range
export async function getUsageStats(range: TimeRange) {
  const result = await commands.getUsageStats(range);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

// Clean up old data beyond retention period
export async function cleanupOldData(retentionDays: number) {
  const result = await commands.cleanupHistory(retentionDays);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

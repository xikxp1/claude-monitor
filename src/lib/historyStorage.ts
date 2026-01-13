import { invoke } from "@tauri-apps/api/core";

// Usage history record type (matches Rust struct)
export interface UsageHistoryRecord {
  id: number;
  timestamp: string;
  five_hour_utilization: number | null;
  five_hour_resets_at: string | null;
  seven_day_utilization: number | null;
  seven_day_resets_at: string | null;
  sonnet_utilization: number | null;
  sonnet_resets_at: string | null;
  opus_utilization: number | null;
  opus_resets_at: string | null;
}

// Time range presets
export type TimeRange = "1h" | "6h" | "24h" | "7d" | "30d";

// Usage statistics for a single metric
export interface MetricStats {
  current: number | null;
  change: number | null;
  velocity: number | null;
}

// Get statistics for a time range
export interface UsageStats {
  fiveHour: MetricStats;
  sevenDay: MetricStats;
  sonnet: MetricStats;
  opus: MetricStats;
  recordCount: number;
  periodHours: number;
}

// Initialize history storage (no-op since backend handles it)
export function initHistoryStorage(): void {
  // Database is initialized by Rust backend
}

// Get usage history for a specific time range preset
export async function getUsageHistoryByRange(range: TimeRange): Promise<UsageHistoryRecord[]> {
  return invoke<UsageHistoryRecord[]>("get_usage_history_by_range", { range });
}

// Get usage statistics for a time range
export async function getUsageStats(range: TimeRange): Promise<UsageStats> {
  return invoke<UsageStats>("get_usage_stats", { range });
}

// Clean up old data beyond retention period
export async function cleanupOldData(retentionDays: number): Promise<number> {
  return invoke<number>("cleanup_history", { retentionDays });
}

import { commands } from "$lib/electrobunClient";
import type { ProviderKind } from "$lib/types";

export type { UsageHistoryPoint, UsageStats, WindowStats } from "$lib/types";

export type TimeRange = "1h" | "6h" | "24h" | "7d" | "30d";

export function initHistoryStorage(): void {
  // Database is initialized by Rust backend.
}

export async function getUsageHistoryByRange(provider: ProviderKind, range: TimeRange) {
  const result = await commands.getUsageHistoryByRange(provider, range);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

export async function getUsageStats(provider: ProviderKind, range: TimeRange) {
  const result = await commands.getUsageStats(provider, range);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

export async function cleanupOldData(retentionDays: number) {
  const result = await commands.cleanupHistory(retentionDays);
  if (result.status === "error") {
    throw new Error(result.error);
  }
  return result.data;
}

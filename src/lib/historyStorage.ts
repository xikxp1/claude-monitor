import Database from "@tauri-apps/plugin-sql";
import type { UsageData } from "./types";

const DB_NAME = "sqlite:usage_history.db";

let db: Database | null = null;
let initPromise: Promise<Database> | null = null;

// Database schema
const SCHEMA = `
  CREATE TABLE IF NOT EXISTS usage_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT NOT NULL,
    five_hour_utilization REAL,
    five_hour_resets_at TEXT,
    seven_day_utilization REAL,
    seven_day_resets_at TEXT,
    sonnet_utilization REAL,
    sonnet_resets_at TEXT,
    opus_utilization REAL,
    opus_resets_at TEXT
  );

  CREATE INDEX IF NOT EXISTS idx_timestamp ON usage_history(timestamp);
`;

async function initDatabase(): Promise<Database> {
  const database = await Database.load(DB_NAME);

  // Create tables if they don't exist
  await database.execute(SCHEMA);

  return database;
}

function getDatabase(): Promise<Database> {
  if (!initPromise) {
    initPromise = initDatabase();
    initPromise
      .then((database) => {
        db = database;
      })
      .catch((e) => {
        console.error("Failed to initialize database:", e);
        initPromise = null;
      });
  }
  return initPromise;
}

// Initialize database early
export function initHistoryStorage(): void {
  getDatabase().catch((e) => {
    console.error("Failed to initialize history storage:", e);
  });
}

// Save a usage snapshot
export async function saveUsageSnapshot(usage: UsageData): Promise<void> {
  const database = await getDatabase();
  const timestamp = new Date().toISOString();

  await database.execute(
    `INSERT INTO usage_history (
      timestamp,
      five_hour_utilization, five_hour_resets_at,
      seven_day_utilization, seven_day_resets_at,
      sonnet_utilization, sonnet_resets_at,
      opus_utilization, opus_resets_at
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    [
      timestamp,
      usage.five_hour?.utilization ?? null,
      usage.five_hour?.resets_at ?? null,
      usage.seven_day?.utilization ?? null,
      usage.seven_day?.resets_at ?? null,
      usage.seven_day_sonnet?.utilization ?? null,
      usage.seven_day_sonnet?.resets_at ?? null,
      usage.seven_day_opus?.utilization ?? null,
      usage.seven_day_opus?.resets_at ?? null,
    ],
  );
}

// Usage history record type
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

// Get usage history within a time range
export async function getUsageHistory(from: Date, to: Date): Promise<UsageHistoryRecord[]> {
  const database = await getDatabase();

  const records = await database.select<UsageHistoryRecord[]>(
    `SELECT * FROM usage_history
     WHERE timestamp >= ? AND timestamp <= ?
     ORDER BY timestamp ASC`,
    [from.toISOString(), to.toISOString()],
  );

  return records;
}

// Get the latest N snapshots
export async function getLatestSnapshots(count: number): Promise<UsageHistoryRecord[]> {
  const database = await getDatabase();

  const records = await database.select<UsageHistoryRecord[]>(
    `SELECT * FROM usage_history
     ORDER BY timestamp DESC
     LIMIT ?`,
    [count],
  );

  // Return in chronological order
  return records.reverse();
}

// Get usage history for a specific time range preset
export type TimeRange = "1h" | "6h" | "24h" | "7d" | "30d";

export async function getUsageHistoryByRange(range: TimeRange): Promise<UsageHistoryRecord[]> {
  const now = new Date();
  let from: Date;

  switch (range) {
    case "1h":
      from = new Date(now.getTime() - 60 * 60 * 1000);
      break;
    case "6h":
      from = new Date(now.getTime() - 6 * 60 * 60 * 1000);
      break;
    case "24h":
      from = new Date(now.getTime() - 24 * 60 * 60 * 1000);
      break;
    case "7d":
      from = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      break;
    case "30d":
      from = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
      break;
  }

  return getUsageHistory(from, now);
}

// Usage statistics for a single metric
export interface MetricStats {
  current: number | null;
  change: number | null; // Change over the period (can be negative if reset occurred)
  velocity: number | null; // Change per hour
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

function getRangeHours(range: TimeRange): number {
  switch (range) {
    case "1h":
      return 1;
    case "6h":
      return 6;
    case "24h":
      return 24;
    case "7d":
      return 7 * 24;
    case "30d":
      return 30 * 24;
  }
}

export async function getUsageStats(range: TimeRange): Promise<UsageStats> {
  const database = await getDatabase();
  const now = new Date();
  const periodHours = getRangeHours(range);
  const from = new Date(now.getTime() - periodHours * 60 * 60 * 1000);

  // Get first and last records in the time range
  const firstRecord = await database.select<UsageHistoryRecord[]>(
    `SELECT * FROM usage_history
     WHERE timestamp >= ? AND timestamp <= ?
     ORDER BY timestamp ASC
     LIMIT 1`,
    [from.toISOString(), now.toISOString()],
  );

  const lastRecord = await database.select<UsageHistoryRecord[]>(
    `SELECT * FROM usage_history
     WHERE timestamp >= ? AND timestamp <= ?
     ORDER BY timestamp DESC
     LIMIT 1`,
    [from.toISOString(), now.toISOString()],
  );

  const countResult = await database.select<{ count: number }[]>(
    `SELECT COUNT(*) as count FROM usage_history
     WHERE timestamp >= ? AND timestamp <= ?`,
    [from.toISOString(), now.toISOString()],
  );

  const first = firstRecord[0];
  const last = lastRecord[0];
  const recordCount = countResult[0]?.count ?? 0;

  function calcMetricStats(firstVal: number | null, lastVal: number | null): MetricStats {
    const current = lastVal;
    let change: number | null = null;
    let velocity: number | null = null;

    if (firstVal !== null && lastVal !== null) {
      change = lastVal - firstVal;
      // Only calculate velocity if change is positive (no reset occurred)
      if (change >= 0 && periodHours > 0) {
        velocity = change / periodHours;
      }
    }

    return { current, change, velocity };
  }

  return {
    fiveHour: calcMetricStats(first?.five_hour_utilization ?? null, last?.five_hour_utilization ?? null),
    sevenDay: calcMetricStats(first?.seven_day_utilization ?? null, last?.seven_day_utilization ?? null),
    sonnet: calcMetricStats(first?.sonnet_utilization ?? null, last?.sonnet_utilization ?? null),
    opus: calcMetricStats(first?.opus_utilization ?? null, last?.opus_utilization ?? null),
    recordCount,
    periodHours,
  };
}

// Clean up old data beyond retention period
export async function cleanupOldData(retentionDays: number): Promise<number> {
  const database = await getDatabase();
  const cutoffDate = new Date(Date.now() - retentionDays * 24 * 60 * 60 * 1000);

  const result = await database.execute(`DELETE FROM usage_history WHERE timestamp < ?`, [cutoffDate.toISOString()]);

  return result.rowsAffected;
}

// Get total record count
export async function getRecordCount(): Promise<number> {
  const database = await getDatabase();

  const result = await database.select<{ count: number }[]>(`SELECT COUNT(*) as count FROM usage_history`);

  return result[0]?.count ?? 0;
}

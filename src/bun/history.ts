import { mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { Database } from "bun:sqlite";
import { Utils } from "electrobun/bun";
import type {
  ProviderKind,
  UsageHistoryPoint,
  UsageSnapshot,
  UsageStats,
  UsageWindow,
  WindowStats,
} from "../shared/types";

const LEGACY_SCHEMA = `
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

const V2_SCHEMA = `
CREATE TABLE IF NOT EXISTS usage_history_v2 (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  provider TEXT NOT NULL,
  timestamp TEXT NOT NULL,
  window_key TEXT NOT NULL,
  label TEXT NOT NULL,
  utilization REAL NOT NULL,
  resets_at TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_usage_history_v2_unique
ON usage_history_v2(provider, timestamp, window_key);
CREATE INDEX IF NOT EXISTS idx_usage_history_v2_lookup
ON usage_history_v2(provider, timestamp, window_key);
`;

let db: Database | null = null;

export function initDatabase(): void {
  getDb();
}

export function saveUsageSnapshot(snapshot: UsageSnapshot): void {
  const conn = getDb();
  insertSnapshot(conn, snapshot.provider, new Date().toISOString(), snapshot.windows);
}

export function getUsageHistoryByRange(
  provider: ProviderKind,
  range: string,
): UsageHistoryPoint[] {
  const now = new Date();
  const hours = getRangeHours(range);
  const from = new Date(now.getTime() - hours * 60 * 60 * 1000).toISOString();
  const to = now.toISOString();
  const bucket = getDownsampleBucketMinutes(range);
  return bucket
    ? getUsageHistoryDownsampled(provider, from, to, bucket)
    : getUsageHistory(provider, from, to);
}

export function getUsageStats(provider: ProviderKind, range: string): UsageStats {
  const conn = getDb();
  const now = new Date();
  const periodHours = getRangeHours(range);
  const from = new Date(now.getTime() - periodHours * 60 * 60 * 1000).toISOString();
  const to = now.toISOString();

  const rows = conn
    .query<
      {
        window_key: string;
        label: string;
        current: number | null;
        first_value: number | null;
        last_value: number | null;
      },
      [string, string, string]
    >(
      `
      WITH ranked AS (
        SELECT
          id,
          provider,
          timestamp,
          window_key,
          label,
          utilization,
          resets_at,
          ROW_NUMBER() OVER (PARTITION BY window_key ORDER BY timestamp ASC, id ASC) AS asc_rank,
          ROW_NUMBER() OVER (PARTITION BY window_key ORDER BY timestamp DESC, id DESC) AS desc_rank
        FROM usage_history_v2
        WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3
      )
      SELECT
        window_key,
        label,
        MAX(CASE WHEN desc_rank = 1 THEN utilization END) AS current,
        MAX(CASE WHEN asc_rank = 1 THEN utilization END) AS first_value,
        MAX(CASE WHEN desc_rank = 1 THEN utilization END) AS last_value
      FROM ranked
      GROUP BY window_key, label
      ORDER BY label ASC
    `,
    )
    .all(provider, from, to);

  const windows: WindowStats[] = rows.map((row) => {
    const change =
      row.first_value !== null && row.last_value !== null
        ? row.last_value - row.first_value
        : null;
    return {
      key: row.window_key,
      label: row.label,
      current: row.current,
      change,
      velocity: change !== null && change >= 0 && periodHours > 0 ? change / periodHours : null,
    };
  });

  const recordCount = conn
    .query<{ count: number }, [string, string, string]>(
      `SELECT COUNT(*) AS count FROM usage_history_v2 WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3`,
    )
    .get(provider, from, to)?.count ?? 0;

  return { windows, recordCount, periodHours };
}

export function cleanupOldData(retentionDays: number): number {
  const cutoff = new Date(Date.now() - retentionDays * 24 * 60 * 60 * 1000).toISOString();
  return getDb().query("DELETE FROM usage_history_v2 WHERE timestamp < ?1").run(cutoff).changes;
}

export function getDownsampleBucketMinutes(range: string): number | null {
  switch (range) {
    case "7d":
      return 60;
    case "30d":
      return 240;
    default:
      return null;
  }
}

export function getRangeHours(range: string): number {
  switch (range) {
    case "1h":
      return 1;
    case "6h":
      return 6;
    case "24h":
      return 24;
    case "7d":
      return 168;
    case "30d":
      return 720;
    default:
      return 24;
  }
}

function getDb(): Database {
  if (db) {
    return db;
  }

  const dbPath = join(Utils.paths.appData, "usage_history.db");
  mkdirSync(dirname(dbPath), { recursive: true });
  db = new Database(dbPath);
  db.exec(LEGACY_SCHEMA);
  db.exec(V2_SCHEMA);
  backfillLegacyClaudeData(db);
  return db;
}

function getUsageHistory(provider: ProviderKind, from: string, to: string): UsageHistoryPoint[] {
  return getDb()
    .query<HistoryRow, [string, string, string]>(
      `SELECT id, provider, timestamp, window_key, label, utilization, resets_at
       FROM usage_history_v2
       WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3
       ORDER BY timestamp ASC, window_key ASC`,
    )
    .all(provider, from, to)
    .map(mapHistoryPoint);
}

function getUsageHistoryDownsampled(
  provider: ProviderKind,
  from: string,
  to: string,
  bucketMinutes: number,
): UsageHistoryPoint[] {
  return getDb()
    .query<HistoryRow, [string, string, string]>(
      `SELECT
        MIN(id) AS id,
        provider,
        datetime((strftime('%s', timestamp) / (${bucketMinutes} * 60)) * (${bucketMinutes} * 60), 'unixepoch') AS timestamp,
        window_key,
        label,
        AVG(utilization) AS utilization,
        MAX(resets_at) AS resets_at
       FROM usage_history_v2
       WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3
       GROUP BY provider, window_key, label, (strftime('%s', timestamp) / (${bucketMinutes} * 60))
       ORDER BY timestamp ASC, window_key ASC`,
    )
    .all(provider, from, to)
    .map(mapHistoryPoint);
}

type HistoryRow = {
  id: number;
  provider: string;
  timestamp: string;
  window_key: string;
  label: string;
  utilization: number;
  resets_at: string | null;
};

function mapHistoryPoint(row: HistoryRow): UsageHistoryPoint {
  return {
    id: row.id,
    provider: parseProvider(row.provider),
    timestamp: row.timestamp,
    windowKey: row.window_key,
    label: row.label,
    utilization: row.utilization,
    resetsAt: row.resets_at,
  };
}

function parseProvider(raw: string): ProviderKind {
  if (raw === "codex" || raw === "ollama") {
    return raw;
  }
  return "claude";
}

function insertSnapshot(
  conn: Database,
  provider: ProviderKind,
  timestamp: string,
  windows: UsageWindow[],
): void {
  const stmt = conn.query(
    `INSERT OR IGNORE INTO usage_history_v2
     (provider, timestamp, window_key, label, utilization, resets_at)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6)`,
  );
  for (const window of windows) {
    stmt.run(provider, timestamp, window.key, window.label, window.utilization, window.resetsAt);
  }
}

function backfillLegacyClaudeData(conn: Database): void {
  const count = conn
    .query<{ count: number }, []>("SELECT COUNT(*) AS count FROM usage_history")
    .get()?.count ?? 0;
  if (count === 0) {
    return;
  }

  const rows = conn
    .query<
      {
        timestamp: string;
        five_hour_utilization: number | null;
        five_hour_resets_at: string | null;
        seven_day_utilization: number | null;
        seven_day_resets_at: string | null;
        sonnet_utilization: number | null;
        sonnet_resets_at: string | null;
        opus_utilization: number | null;
        opus_resets_at: string | null;
      },
      []
    >(
      `SELECT timestamp, five_hour_utilization, five_hour_resets_at,
        seven_day_utilization, seven_day_resets_at,
        sonnet_utilization, sonnet_resets_at,
        opus_utilization, opus_resets_at
       FROM usage_history ORDER BY timestamp ASC`,
    )
    .all();

  for (const row of rows) {
    insertSnapshot(
      conn,
      "claude",
      row.timestamp,
      [
        legacyWindow("five_hour", "5 Hour", row.five_hour_utilization, row.five_hour_resets_at),
        legacyWindow("seven_day", "7 Day", row.seven_day_utilization, row.seven_day_resets_at),
        legacyWindow(
          "seven_day_sonnet",
          "Sonnet (7 Day)",
          row.sonnet_utilization,
          row.sonnet_resets_at,
        ),
        legacyWindow("seven_day_opus", "Opus (7 Day)", row.opus_utilization, row.opus_resets_at),
      ].filter((window): window is UsageWindow => Boolean(window)),
    );
  }
}

function legacyWindow(
  key: string,
  label: string,
  utilization: number | null,
  resetsAt: string | null,
): UsageWindow | null {
  if (utilization === null) {
    return null;
  }
  return { key, label, utilization, resetsAt, windowDurationSeconds: null };
}

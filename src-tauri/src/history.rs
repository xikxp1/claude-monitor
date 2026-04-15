use crate::types::{ProviderKind, UsageSnapshot};
use rusqlite::{Connection, OptionalExtension, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

const LEGACY_SCHEMA: &str = r#"
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
"#;

const V2_SCHEMA: &str = r#"
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
"#;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageHistoryPoint {
    pub id: i64,
    pub provider: ProviderKind,
    pub timestamp: String,
    pub window_key: String,
    pub label: String,
    pub utilization: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WindowStats {
    pub key: String,
    pub label: String,
    pub current: Option<f64>,
    pub change: Option<f64>,
    pub velocity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub windows: Vec<WindowStats>,
    pub record_count: i64,
    pub period_hours: f64,
}

pub fn init_database<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> SqliteResult<()> {
    let db_path = get_db_path(app).ok_or_else(|| {
        rusqlite::Error::InvalidPath("Could not determine app data directory".into())
    })?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(&db_path)?;
    conn.execute_batch(LEGACY_SCHEMA)?;
    conn.execute_batch(V2_SCHEMA)?;
    backfill_legacy_claude_data(&conn)?;
    let _ = DB.set(Mutex::new(conn));
    Ok(())
}

pub fn save_usage_snapshot(snapshot: &UsageSnapshot) -> SqliteResult<()> {
    let conn = get_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();
    insert_snapshot(&conn, snapshot.provider, &timestamp, &snapshot.windows)
}

pub fn get_usage_history_by_range(
    provider: ProviderKind,
    range: &str,
) -> SqliteResult<Vec<UsageHistoryPoint>> {
    let now = chrono::Utc::now();
    let hours = get_range_hours(range) as i64;
    let from = now - chrono::Duration::hours(hours);
    let from_str = from.to_rfc3339();
    let now_str = now.to_rfc3339();

    if let Some(bucket_minutes) = get_downsample_bucket_minutes(range) {
        get_usage_history_downsampled(provider, &from_str, &now_str, bucket_minutes)
    } else {
        get_usage_history(provider, &from_str, &now_str)
    }
}

pub fn get_usage_stats(provider: ProviderKind, range: &str) -> SqliteResult<UsageStats> {
    let conn = get_db()?;
    let now = chrono::Utc::now();
    let period_hours = get_range_hours(range);
    let from = now - chrono::Duration::hours(period_hours as i64);
    let from_str = from.to_rfc3339();
    let now_str = now.to_rfc3339();
    let provider_str = provider.as_str();

    let mut stmt = conn.prepare(
        r#"
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
        "#,
    )?;

    let windows = stmt
        .query_map(
            rusqlite::params![provider_str, &from_str, &now_str],
            |row| {
                let current: Option<f64> = row.get(2)?;
                let first_value: Option<f64> = row.get(3)?;
                let last_value: Option<f64> = row.get(4)?;
                let change = match (first_value, last_value) {
                    (Some(first), Some(last)) => Some(last - first),
                    _ => None,
                };
                let velocity = change.and_then(|delta| {
                    if delta >= 0.0 && period_hours > 0.0 {
                        Some(delta / period_hours)
                    } else {
                        None
                    }
                });

                Ok(WindowStats {
                    key: row.get(0)?,
                    label: row.get(1)?,
                    current,
                    change,
                    velocity,
                })
            },
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let record_count: i64 = conn.query_row(
        r#"SELECT COUNT(*) FROM usage_history_v2 WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3"#,
        rusqlite::params![provider_str, &from_str, &now_str],
        |row| row.get(0),
    )?;

    Ok(UsageStats {
        windows,
        record_count,
        period_hours,
    })
}

pub fn cleanup_old_data(retention_days: u32) -> SqliteResult<usize> {
    let conn = get_db()?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    conn.execute(
        "DELETE FROM usage_history_v2 WHERE timestamp < ?1",
        rusqlite::params![cutoff_str],
    )
}

pub fn get_downsample_bucket_minutes(range: &str) -> Option<u32> {
    match range {
        "7d" => Some(60),
        "30d" => Some(240),
        _ => None,
    }
}

fn get_usage_history(
    provider: ProviderKind,
    from: &str,
    to: &str,
) -> SqliteResult<Vec<UsageHistoryPoint>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(
        r#"SELECT id, provider, timestamp, window_key, label, utilization, resets_at
        FROM usage_history_v2
        WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3
        ORDER BY timestamp ASC, window_key ASC"#,
    )?;

    stmt.query_map(
        rusqlite::params![provider.as_str(), from, to],
        map_history_point,
    )?
    .collect::<Result<Vec<_>, _>>()
}

fn get_usage_history_downsampled(
    provider: ProviderKind,
    from: &str,
    to: &str,
    bucket_minutes: u32,
) -> SqliteResult<Vec<UsageHistoryPoint>> {
    let conn = get_db()?;
    let query = format!(
        r#"SELECT
            MIN(id) AS id,
            provider,
            datetime((strftime('%s', timestamp) / ({bucket_minutes} * 60)) * ({bucket_minutes} * 60), 'unixepoch') AS timestamp,
            window_key,
            label,
            AVG(utilization) AS utilization,
            MAX(resets_at) AS resets_at
        FROM usage_history_v2
        WHERE provider = ?1 AND timestamp >= ?2 AND timestamp <= ?3
        GROUP BY provider, window_key, label, (strftime('%s', timestamp) / ({bucket_minutes} * 60))
        ORDER BY timestamp ASC, window_key ASC"#
    );

    let mut stmt = conn.prepare(&query)?;
    stmt.query_map(
        rusqlite::params![provider.as_str(), from, to],
        map_history_point,
    )?
    .collect::<Result<Vec<_>, _>>()
}

fn map_history_point(row: &rusqlite::Row<'_>) -> SqliteResult<UsageHistoryPoint> {
    let provider_raw: String = row.get(1)?;
    Ok(UsageHistoryPoint {
        id: row.get(0)?,
        provider: parse_provider(&provider_raw),
        timestamp: row.get(2)?,
        window_key: row.get(3)?,
        label: row.get(4)?,
        utilization: row.get(5)?,
        resets_at: row.get(6)?,
    })
}

fn parse_provider(raw: &str) -> ProviderKind {
    match raw {
        "codex" => ProviderKind::Codex,
        "ollama" => ProviderKind::Ollama,
        _ => ProviderKind::Claude,
    }
}

fn insert_snapshot(
    conn: &Connection,
    provider: ProviderKind,
    timestamp: &str,
    windows: &[crate::types::UsageWindow],
) -> SqliteResult<()> {
    let mut stmt = conn.prepare(
        r#"INSERT OR IGNORE INTO usage_history_v2
        (provider, timestamp, window_key, label, utilization, resets_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
    )?;

    for window in windows {
        stmt.execute(rusqlite::params![
            provider.as_str(),
            timestamp,
            &window.key,
            &window.label,
            window.utilization,
            &window.resets_at,
        ])?;
    }

    Ok(())
}

fn backfill_legacy_claude_data(conn: &Connection) -> SqliteResult<()> {
    let has_legacy_rows: Option<i64> = conn
        .query_row("SELECT COUNT(*) FROM usage_history", [], |row| row.get(0))
        .optional()?;

    if has_legacy_rows.unwrap_or(0) == 0 {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        r#"SELECT timestamp, five_hour_utilization, five_hour_resets_at,
            seven_day_utilization, seven_day_resets_at,
            sonnet_utilization, sonnet_resets_at,
            opus_utilization, opus_resets_at
        FROM usage_history
        ORDER BY timestamp ASC"#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<f64>>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<f64>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<f64>>(7)?,
            row.get::<_, Option<String>>(8)?,
        ))
    })?;

    for row in rows {
        let (
            timestamp,
            five_hour_utilization,
            five_hour_resets_at,
            seven_day_utilization,
            seven_day_resets_at,
            sonnet_utilization,
            sonnet_resets_at,
            opus_utilization,
            opus_resets_at,
        ) = row?;

        let legacy_windows = [
            legacy_window(
                "five_hour",
                "5 Hour",
                five_hour_utilization,
                five_hour_resets_at,
            ),
            legacy_window(
                "seven_day",
                "7 Day",
                seven_day_utilization,
                seven_day_resets_at,
            ),
            legacy_window(
                "seven_day_sonnet",
                "Sonnet (7 Day)",
                sonnet_utilization,
                sonnet_resets_at,
            ),
            legacy_window(
                "seven_day_opus",
                "Opus (7 Day)",
                opus_utilization,
                opus_resets_at,
            ),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        insert_snapshot(conn, ProviderKind::Claude, &timestamp, &legacy_windows)?;
    }

    Ok(())
}

fn legacy_window(
    key: &str,
    label: &str,
    utilization: Option<f64>,
    resets_at: Option<String>,
) -> Option<crate::types::UsageWindow> {
    Some(crate::types::UsageWindow {
        key: key.to_string(),
        label: label.to_string(),
        utilization: utilization?,
        resets_at,
        window_duration_seconds: None,
    })
}

fn get_range_hours(range: &str) -> f64 {
    match range {
        "1h" => 1.0,
        "6h" => 6.0,
        "24h" => 24.0,
        "7d" => 168.0,
        "30d" => 720.0,
        _ => 24.0,
    }
}

fn get_db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("usage_history.db"))
}

fn get_db() -> SqliteResult<std::sync::MutexGuard<'static, Connection>> {
    let db = DB.get().ok_or(rusqlite::Error::InvalidQuery)?;
    db.lock().map_err(|_| rusqlite::Error::InvalidQuery)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_expected_range_hours() {
        assert_eq!(get_range_hours("1h"), 1.0);
        assert_eq!(get_range_hours("30d"), 720.0);
        assert_eq!(get_range_hours("nope"), 24.0);
    }

    #[test]
    fn returns_expected_downsample_buckets() {
        assert_eq!(get_downsample_bucket_minutes("24h"), None);
        assert_eq!(get_downsample_bucket_minutes("7d"), Some(60));
        assert_eq!(get_downsample_bucket_minutes("30d"), Some(240));
    }

    #[test]
    fn parses_provider_names() {
        assert_eq!(parse_provider("claude"), ProviderKind::Claude);
        assert_eq!(parse_provider("codex"), ProviderKind::Codex);
    }
}

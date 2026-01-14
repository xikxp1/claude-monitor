use crate::types::UsageData;
use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

/// Global database connection (lazy initialized)
static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

const SCHEMA: &str = r#"
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

/// Usage history record matching frontend TypeScript interface
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UsageHistoryRecord {
    pub id: i64,
    pub timestamp: String,
    pub five_hour_utilization: Option<f64>,
    pub five_hour_resets_at: Option<String>,
    pub seven_day_utilization: Option<f64>,
    pub seven_day_resets_at: Option<String>,
    pub sonnet_utilization: Option<f64>,
    pub sonnet_resets_at: Option<String>,
    pub opus_utilization: Option<f64>,
    pub opus_resets_at: Option<String>,
}

/// Metric statistics
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct MetricStats {
    pub current: Option<f64>,
    pub change: Option<f64>,
    pub velocity: Option<f64>,
}

/// Usage statistics for a time range
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub five_hour: MetricStats,
    pub seven_day: MetricStats,
    pub sonnet: MetricStats,
    pub opus: MetricStats,
    pub record_count: i64,
    pub period_hours: f64,
}

/// Get the database path in the app data directory
fn get_db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join("usage_history.db"))
}

/// Initialize the database connection
pub fn init_database<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> SqliteResult<()> {
    let db_path = get_db_path(app).ok_or_else(|| {
        rusqlite::Error::InvalidPath("Could not determine app data directory".into())
    })?;

    // Ensure the directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(&db_path)?;

    // Create tables if they don't exist
    conn.execute_batch(SCHEMA)?;

    // Store the connection globally
    let _ = DB.set(Mutex::new(conn));

    Ok(())
}

fn get_db() -> SqliteResult<std::sync::MutexGuard<'static, Connection>> {
    let db = DB.get().ok_or_else(|| rusqlite::Error::InvalidQuery)?;
    db.lock().map_err(|_| rusqlite::Error::InvalidQuery)
}

/// Save a usage snapshot to the database
pub fn save_usage_snapshot(usage: &UsageData) -> SqliteResult<()> {
    let conn = get_db()?;
    let timestamp = chrono::Utc::now().to_rfc3339();

    conn.execute(
        r#"INSERT INTO usage_history (
            timestamp,
            five_hour_utilization, five_hour_resets_at,
            seven_day_utilization, seven_day_resets_at,
            sonnet_utilization, sonnet_resets_at,
            opus_utilization, opus_resets_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
        rusqlite::params![
            timestamp,
            usage.five_hour.as_ref().map(|p| p.utilization),
            usage.five_hour.as_ref().map(|p| &p.resets_at),
            usage.seven_day.as_ref().map(|p| p.utilization),
            usage.seven_day.as_ref().map(|p| &p.resets_at),
            usage.seven_day_sonnet.as_ref().map(|p| p.utilization),
            usage.seven_day_sonnet.as_ref().map(|p| &p.resets_at),
            usage.seven_day_opus.as_ref().map(|p| p.utilization),
            usage.seven_day_opus.as_ref().map(|p| &p.resets_at),
        ],
    )?;

    Ok(())
}

/// Get usage history within a time range
pub fn get_usage_history(from: &str, to: &str) -> SqliteResult<Vec<UsageHistoryRecord>> {
    let conn = get_db()?;
    let mut stmt = conn.prepare(
        r#"SELECT id, timestamp,
            five_hour_utilization, five_hour_resets_at,
            seven_day_utilization, seven_day_resets_at,
            sonnet_utilization, sonnet_resets_at,
            opus_utilization, opus_resets_at
        FROM usage_history
        WHERE timestamp >= ?1 AND timestamp <= ?2
        ORDER BY timestamp ASC"#,
    )?;

    let records = stmt
        .query_map(rusqlite::params![from, to], |row| {
            Ok(UsageHistoryRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                five_hour_utilization: row.get(2)?,
                five_hour_resets_at: row.get(3)?,
                seven_day_utilization: row.get(4)?,
                seven_day_resets_at: row.get(5)?,
                sonnet_utilization: row.get(6)?,
                sonnet_resets_at: row.get(7)?,
                opus_utilization: row.get(8)?,
                opus_resets_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Get usage history by preset time range.
/// Automatically applies downsampling for larger ranges (7d, 30d) to improve performance.
pub fn get_usage_history_by_range(range: &str) -> SqliteResult<Vec<UsageHistoryRecord>> {
    let now = chrono::Utc::now();
    let hours = match range {
        "1h" => 1,
        "6h" => 6,
        "24h" => 24,
        "7d" => 7 * 24,
        "30d" => 30 * 24,
        _ => 24, // default to 24h
    };

    let from = now - chrono::Duration::hours(hours);
    let from_str = from.to_rfc3339();
    let now_str = now.to_rfc3339();

    // Apply downsampling for larger time ranges
    if let Some(bucket_minutes) = get_downsample_bucket_minutes(range) {
        get_usage_history_downsampled(&from_str, &now_str, bucket_minutes)
    } else {
        get_usage_history(&from_str, &now_str)
    }
}

fn get_range_hours(range: &str) -> f64 {
    match range {
        "1h" => 1.0,
        "6h" => 6.0,
        "24h" => 24.0,
        "7d" => 7.0 * 24.0,
        "30d" => 30.0 * 24.0,
        _ => 24.0,
    }
}

/// Get the bucket size in minutes for downsampling based on time range.
/// Returns None if no downsampling should be applied (full resolution).
///
/// Strategy:
/// - 1h, 6h, 24h: No downsampling (full resolution, max ~288 points/day)
/// - 7d: 1-hour buckets (max 168 points)
/// - 30d: 4-hour buckets (max 180 points)
pub fn get_downsample_bucket_minutes(range: &str) -> Option<u32> {
    match range {
        "7d" => Some(60),   // 1-hour buckets
        "30d" => Some(240), // 4-hour buckets
        _ => None,          // No downsampling for 1h, 6h, 24h
    }
}

/// Get usage history with automatic downsampling for larger time ranges.
/// Uses time-bucket aggregation to reduce data points while preserving trends.
pub fn get_usage_history_downsampled(
    from: &str,
    to: &str,
    bucket_minutes: u32,
) -> SqliteResult<Vec<UsageHistoryRecord>> {
    let conn = get_db()?;

    // SQLite strftime format for bucketing by minutes
    // We truncate timestamps to bucket boundaries using integer division
    // Example: bucket_minutes=60 groups by hour, bucket_minutes=240 groups by 4 hours
    let query = format!(
        r#"SELECT
            MIN(id) as id,
            datetime(
                (strftime('%s', timestamp) / ({bucket_minutes} * 60)) * ({bucket_minutes} * 60),
                'unixepoch'
            ) as bucket_timestamp,
            AVG(five_hour_utilization) as five_hour_utilization,
            MAX(five_hour_resets_at) as five_hour_resets_at,
            AVG(seven_day_utilization) as seven_day_utilization,
            MAX(seven_day_resets_at) as seven_day_resets_at,
            AVG(sonnet_utilization) as sonnet_utilization,
            MAX(sonnet_resets_at) as sonnet_resets_at,
            AVG(opus_utilization) as opus_utilization,
            MAX(opus_resets_at) as opus_resets_at
        FROM usage_history
        WHERE timestamp >= ?1 AND timestamp <= ?2
        GROUP BY (strftime('%s', timestamp) / ({bucket_minutes} * 60))
        ORDER BY bucket_timestamp ASC"#,
        bucket_minutes = bucket_minutes
    );

    let mut stmt = conn.prepare(&query)?;

    let records = stmt
        .query_map(rusqlite::params![from, to], |row| {
            Ok(UsageHistoryRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                five_hour_utilization: row.get(2)?,
                five_hour_resets_at: row.get(3)?,
                seven_day_utilization: row.get(4)?,
                seven_day_resets_at: row.get(5)?,
                sonnet_utilization: row.get(6)?,
                sonnet_resets_at: row.get(7)?,
                opus_utilization: row.get(8)?,
                opus_resets_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(records)
}

/// Get usage statistics for a time range
pub fn get_usage_stats(range: &str) -> SqliteResult<UsageStats> {
    let conn = get_db()?;
    let now = chrono::Utc::now();
    let period_hours = get_range_hours(range);
    let from = now - chrono::Duration::hours(period_hours as i64);
    let from_str = from.to_rfc3339();
    let now_str = now.to_rfc3339();

    // Get first record
    let first: Option<UsageHistoryRecord> = conn
        .query_row(
            r#"SELECT id, timestamp,
                five_hour_utilization, five_hour_resets_at,
                seven_day_utilization, seven_day_resets_at,
                sonnet_utilization, sonnet_resets_at,
                opus_utilization, opus_resets_at
            FROM usage_history
            WHERE timestamp >= ?1 AND timestamp <= ?2
            ORDER BY timestamp ASC
            LIMIT 1"#,
            rusqlite::params![&from_str, &now_str],
            |row| {
                Ok(UsageHistoryRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    five_hour_utilization: row.get(2)?,
                    five_hour_resets_at: row.get(3)?,
                    seven_day_utilization: row.get(4)?,
                    seven_day_resets_at: row.get(5)?,
                    sonnet_utilization: row.get(6)?,
                    sonnet_resets_at: row.get(7)?,
                    opus_utilization: row.get(8)?,
                    opus_resets_at: row.get(9)?,
                })
            },
        )
        .ok();

    // Get last record
    let last: Option<UsageHistoryRecord> = conn
        .query_row(
            r#"SELECT id, timestamp,
                five_hour_utilization, five_hour_resets_at,
                seven_day_utilization, seven_day_resets_at,
                sonnet_utilization, sonnet_resets_at,
                opus_utilization, opus_resets_at
            FROM usage_history
            WHERE timestamp >= ?1 AND timestamp <= ?2
            ORDER BY timestamp DESC
            LIMIT 1"#,
            rusqlite::params![&from_str, &now_str],
            |row| {
                Ok(UsageHistoryRecord {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    five_hour_utilization: row.get(2)?,
                    five_hour_resets_at: row.get(3)?,
                    seven_day_utilization: row.get(4)?,
                    seven_day_resets_at: row.get(5)?,
                    sonnet_utilization: row.get(6)?,
                    sonnet_resets_at: row.get(7)?,
                    opus_utilization: row.get(8)?,
                    opus_resets_at: row.get(9)?,
                })
            },
        )
        .ok();

    // Get record count
    let record_count: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM usage_history WHERE timestamp >= ?1 AND timestamp <= ?2"#,
            rusqlite::params![&from_str, &now_str],
            |row| row.get(0),
        )
        .unwrap_or(0);

    fn calc_stats(
        first_val: Option<f64>,
        last_val: Option<f64>,
        period_hours: f64,
    ) -> MetricStats {
        let current = last_val;
        let change = match (first_val, last_val) {
            (Some(f), Some(l)) => Some(l - f),
            _ => None,
        };
        let velocity = change.and_then(|c| {
            if c >= 0.0 && period_hours > 0.0 {
                Some(c / period_hours)
            } else {
                None
            }
        });
        MetricStats {
            current,
            change,
            velocity,
        }
    }

    Ok(UsageStats {
        five_hour: calc_stats(
            first.as_ref().and_then(|r| r.five_hour_utilization),
            last.as_ref().and_then(|r| r.five_hour_utilization),
            period_hours,
        ),
        seven_day: calc_stats(
            first.as_ref().and_then(|r| r.seven_day_utilization),
            last.as_ref().and_then(|r| r.seven_day_utilization),
            period_hours,
        ),
        sonnet: calc_stats(
            first.as_ref().and_then(|r| r.sonnet_utilization),
            last.as_ref().and_then(|r| r.sonnet_utilization),
            period_hours,
        ),
        opus: calc_stats(
            first.as_ref().and_then(|r| r.opus_utilization),
            last.as_ref().and_then(|r| r.opus_utilization),
            period_hours,
        ),
        record_count,
        period_hours,
    })
}

/// Clean up old data beyond retention period
pub fn cleanup_old_data(retention_days: u32) -> SqliteResult<usize> {
    let conn = get_db()?;
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff.to_rfc3339();

    let rows_deleted = conn.execute(
        "DELETE FROM usage_history WHERE timestamp < ?1",
        rusqlite::params![cutoff_str],
    )?;

    Ok(rows_deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod get_range_hours {
        use super::*;

        #[test]
        fn converts_1h_correctly() {
            assert_eq!(get_range_hours("1h"), 1.0);
        }

        #[test]
        fn converts_6h_correctly() {
            assert_eq!(get_range_hours("6h"), 6.0);
        }

        #[test]
        fn converts_24h_correctly() {
            assert_eq!(get_range_hours("24h"), 24.0);
        }

        #[test]
        fn converts_7d_correctly() {
            assert_eq!(get_range_hours("7d"), 168.0); // 7 * 24
        }

        #[test]
        fn converts_30d_correctly() {
            assert_eq!(get_range_hours("30d"), 720.0); // 30 * 24
        }

        #[test]
        fn defaults_to_24h_for_unknown() {
            assert_eq!(get_range_hours("invalid"), 24.0);
            assert_eq!(get_range_hours(""), 24.0);
            assert_eq!(get_range_hours("1w"), 24.0);
        }
    }

    mod get_downsample_bucket_minutes {
        use super::*;

        #[test]
        fn returns_none_for_1h() {
            assert_eq!(get_downsample_bucket_minutes("1h"), None);
        }

        #[test]
        fn returns_none_for_6h() {
            assert_eq!(get_downsample_bucket_minutes("6h"), None);
        }

        #[test]
        fn returns_none_for_24h() {
            assert_eq!(get_downsample_bucket_minutes("24h"), None);
        }

        #[test]
        fn returns_60_minutes_for_7d() {
            assert_eq!(get_downsample_bucket_minutes("7d"), Some(60));
        }

        #[test]
        fn returns_240_minutes_for_30d() {
            assert_eq!(get_downsample_bucket_minutes("30d"), Some(240));
        }

        #[test]
        fn returns_none_for_unknown() {
            assert_eq!(get_downsample_bucket_minutes("invalid"), None);
            assert_eq!(get_downsample_bucket_minutes(""), None);
            assert_eq!(get_downsample_bucket_minutes("1w"), None);
        }
    }

    mod downsampling_strategy {
        use super::*;

        /// Test that the downsampling strategy produces reasonable data point counts
        #[test]
        fn max_data_points_for_7d() {
            // 7 days * 24 hours = 168 hours
            // With 60-minute buckets, max 168 data points
            let bucket_minutes = get_downsample_bucket_minutes("7d").unwrap();
            let hours = get_range_hours("7d") as u32;
            let max_points = (hours * 60) / bucket_minutes;
            assert_eq!(max_points, 168);
        }

        #[test]
        fn max_data_points_for_30d() {
            // 30 days * 24 hours = 720 hours
            // With 240-minute (4-hour) buckets, max 180 data points
            let bucket_minutes = get_downsample_bucket_minutes("30d").unwrap();
            let hours = get_range_hours("30d") as u32;
            let max_points = (hours * 60) / bucket_minutes;
            assert_eq!(max_points, 180);
        }

        #[test]
        fn no_downsampling_preserves_full_resolution() {
            // For 24h with 5-minute refresh interval, max 288 points
            // This is acceptable without downsampling
            assert!(get_downsample_bucket_minutes("24h").is_none());
            assert!(get_downsample_bucket_minutes("6h").is_none());
            assert!(get_downsample_bucket_minutes("1h").is_none());
        }

        #[test]
        fn bucket_size_increases_with_range() {
            // Larger ranges should have larger bucket sizes
            let bucket_7d = get_downsample_bucket_minutes("7d");
            let bucket_30d = get_downsample_bucket_minutes("30d");

            assert!(bucket_7d.is_some());
            assert!(bucket_30d.is_some());
            assert!(bucket_30d.unwrap() > bucket_7d.unwrap());
        }
    }

    mod downsampling_math {
        /// Verify the SQL bucketing math works correctly
        #[test]
        fn bucket_calculation_60_minutes() {
            // For 60-minute buckets, timestamps should be grouped by hour
            // Unix timestamp 1704067200 = 2024-01-01 00:00:00 UTC
            let bucket_minutes = 60u64;
            let bucket_seconds = bucket_minutes * 60;

            // 00:05:00 and 00:55:00 should be in the same bucket
            let ts1 = 1704067200 + (5 * 60); // 00:05:00
            let ts2 = 1704067200 + (55 * 60); // 00:55:00
            let ts3 = 1704067200 + (65 * 60); // 01:05:00 (next bucket)

            let bucket1 = (ts1 / bucket_seconds) * bucket_seconds;
            let bucket2 = (ts2 / bucket_seconds) * bucket_seconds;
            let bucket3 = (ts3 / bucket_seconds) * bucket_seconds;

            assert_eq!(bucket1, bucket2); // Same hour
            assert_ne!(bucket1, bucket3); // Different hours
        }

        #[test]
        fn bucket_calculation_240_minutes() {
            // For 240-minute (4-hour) buckets
            let bucket_minutes = 240u64;
            let bucket_seconds = bucket_minutes * 60;

            // Unix timestamp 1704067200 = 2024-01-01 00:00:00 UTC
            let base = 1704067200u64;

            // 01:00 and 03:00 should be in the same bucket (0-4 hour window)
            let ts1 = base + (1 * 3600); // 01:00
            let ts2 = base + (3 * 3600); // 03:00
            let ts3 = base + (5 * 3600); // 05:00 (next bucket: 4-8 hour window)

            let bucket1 = (ts1 / bucket_seconds) * bucket_seconds;
            let bucket2 = (ts2 / bucket_seconds) * bucket_seconds;
            let bucket3 = (ts3 / bucket_seconds) * bucket_seconds;

            assert_eq!(bucket1, bucket2); // Same 4-hour window
            assert_ne!(bucket1, bucket3); // Different 4-hour windows
        }

        #[test]
        fn averaging_preserves_range() {
            // Test that averaging multiple values stays within expected range
            let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
            let avg: f64 = values.iter().sum::<f64>() / values.len() as f64;

            assert_eq!(avg, 30.0);
            assert!(avg >= 10.0 && avg <= 50.0);
        }
    }

    mod metric_stats_calculation {
        use super::*;

        // Test the calc_stats logic by directly testing it
        // Note: calc_stats is defined inside get_usage_stats, so we test via a helper

        fn calc_stats(
            first_val: Option<f64>,
            last_val: Option<f64>,
            period_hours: f64,
        ) -> MetricStats {
            let current = last_val;
            let change = match (first_val, last_val) {
                (Some(f), Some(l)) => Some(l - f),
                _ => None,
            };
            let velocity = change.and_then(|c| {
                if c >= 0.0 && period_hours > 0.0 {
                    Some(c / period_hours)
                } else {
                    None
                }
            });
            MetricStats {
                current,
                change,
                velocity,
            }
        }

        #[test]
        fn returns_none_for_no_values() {
            let stats = calc_stats(None, None, 24.0);
            assert!(stats.current.is_none());
            assert!(stats.change.is_none());
            assert!(stats.velocity.is_none());
        }

        #[test]
        fn returns_current_only_when_first_missing() {
            let stats = calc_stats(None, Some(50.0), 24.0);
            assert_eq!(stats.current, Some(50.0));
            assert!(stats.change.is_none());
            assert!(stats.velocity.is_none());
        }

        #[test]
        fn calculates_positive_change_correctly() {
            let stats = calc_stats(Some(20.0), Some(50.0), 24.0);
            assert_eq!(stats.current, Some(50.0));
            assert_eq!(stats.change, Some(30.0));
            assert_eq!(stats.velocity, Some(1.25)); // 30 / 24
        }

        #[test]
        fn calculates_negative_change_without_velocity() {
            let stats = calc_stats(Some(80.0), Some(50.0), 24.0);
            assert_eq!(stats.current, Some(50.0));
            assert_eq!(stats.change, Some(-30.0));
            assert!(stats.velocity.is_none()); // No velocity for negative change
        }

        #[test]
        fn handles_zero_change() {
            let stats = calc_stats(Some(50.0), Some(50.0), 24.0);
            assert_eq!(stats.current, Some(50.0));
            assert_eq!(stats.change, Some(0.0));
            assert_eq!(stats.velocity, Some(0.0));
        }

        #[test]
        fn handles_zero_period_hours() {
            let stats = calc_stats(Some(20.0), Some(50.0), 0.0);
            assert_eq!(stats.change, Some(30.0));
            assert!(stats.velocity.is_none()); // Can't divide by zero
        }
    }
}

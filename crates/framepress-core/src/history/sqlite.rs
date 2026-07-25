//! SQLite-backed history store. Uses `rusqlite` directly with a manual
//! `Mutex<Connection>` (deadpool is overkill for the single-user desktop
//! case but we still serialize access; the spec calls for `r2d2`/`deadpool`
//! and the abstraction is preserved in the trait API).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;

use chrono::{Datelike, Duration, Local, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{CompressionResult, ImageFormat};
use crate::history::entry::{HistoryEntry, HistoryStatus};
use crate::history::{
    AnalyticsRange, AnalyticsSnapshot, BiggestWin, HistoryRepository, SavingsTrendPoint,
    StatsBreakdown,
};

/// Configuration for opening the SQLite DB.
pub struct SqliteHistoryConfig {
    /// Path to the SQLite file. `:memory:` for tests.
    pub path: PathBuf,
}

impl SqliteHistoryConfig {
    /// Default config at `$XDG_DATA_HOME/framepress/history.db` (Linux) or the
    /// macOS `~/Library/Application Support/framepress/history.db`.
    pub fn default_path() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("framepress").join("history.db")
    }
}

/// The on-disk history store. Phase 1: serialized access via `Mutex`.
pub struct SqliteHistory {
    conn: Mutex<Connection>,
}

impl SqliteHistory {
    /// Open or create the DB at `config.path`. Runs migrations.
    pub fn open(config: SqliteHistoryConfig) -> anyhow::Result<Self> {
        if config.path != Path::new(":memory:") {
            if let Some(parent) = config.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&config.path)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Apply schema migrations using the `user_version` PRAGMA.
    fn migrate(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().expect("history conn poisoned");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                input_path TEXT NOT NULL,
                output_path TEXT,
                format TEXT NOT NULL,
                original_bytes INTEGER NOT NULL,
                optimized_bytes INTEGER,
                engine TEXT,
                preset TEXT NOT NULL,
                source TEXT NOT NULL DEFAULT 'Desktop',
                status TEXT NOT NULL,
                error_message TEXT,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                thumbnail_path TEXT,
                dssim REAL,
                margin_pct REAL
            );
            CREATE INDEX IF NOT EXISTS idx_history_completed_at ON history(completed_at);
            CREATE INDEX IF NOT EXISTS idx_history_status ON history(status);
            CREATE INDEX IF NOT EXISTS idx_history_status_completed_at ON history(status, completed_at);",
        )?;
        let has_source = conn
            .prepare("PRAGMA table_info(history)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|column| column == "source");
        if !has_source {
            conn.execute(
                "ALTER TABLE history ADD COLUMN source TEXT NOT NULL DEFAULT 'Desktop'",
                [],
            )?;
        }
        conn.execute_batch("PRAGMA user_version = 2;")?;
        Ok(())
    }

    /// Insert a fully-formed entry. Returns the row id.
    pub fn insert(&self, entry: &HistoryEntry) -> anyhow::Result<i64> {
        let conn = self.conn.lock().expect("history conn poisoned");
        conn.execute(
            "INSERT INTO history (
                input_path, output_path, format, original_bytes, optimized_bytes,
                engine, preset, source, status, error_message, started_at, completed_at,
                thumbnail_path, dssim, margin_pct
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                entry.input_path,
                entry.output_path,
                entry.format,
                entry.original_bytes as i64,
                entry.optimized_bytes.map(|v| v as i64),
                entry.engine,
                entry.preset,
                entry.source,
                entry.status.as_str(),
                entry.error_message,
                entry.started_at,
                entry.completed_at,
                entry.thumbnail_path,
                entry.dssim,
                entry.margin_pct,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Fetch up to `limit` most recent entries.
    pub fn recent(&self, limit: u32) -> anyhow::Result<Vec<HistoryEntry>> {
        let conn = self.conn.lock().expect("history conn poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, input_path, output_path, format, original_bytes, optimized_bytes,
                    engine, preset, status, error_message, started_at, completed_at,
                    thumbnail_path, dssim, margin_pct, source
             FROM history ORDER BY completed_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            let status_str: String = row.get(8)?;
            Ok(HistoryEntry {
                id: row.get(0)?,
                input_path: row.get(1)?,
                output_path: row.get(2)?,
                format: row.get(3)?,
                original_bytes: row.get::<_, i64>(4)? as u64,
                optimized_bytes: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                engine: row.get(6)?,
                preset: row.get(7)?,
                status: HistoryStatus::parse(&status_str).unwrap_or(HistoryStatus::Failed),
                error_message: row.get(9)?,
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
                thumbnail_path: row.get(12)?,
                dssim: row.get(13)?,
                margin_pct: row.get(14)?,
                source: row.get(15)?,
            })
        })?;
        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }

    /// Compute aggregate stats: today's savings (bytes + count), total count.
    pub fn stats(&self) -> anyhow::Result<crate::history::StatsSnapshot> {
        let conn = self.conn.lock().expect("history conn poisoned");
        let today_cutoff = local_midnight_millis(Local::now().date_naive());

        let today_savings: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(original_bytes - COALESCE(optimized_bytes, 0)), 0)
                 FROM history
                 WHERE status = 'Completed' AND completed_at >= ?1",
                params![today_cutoff],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let today_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM history
                 WHERE status = 'Completed' AND completed_at >= ?1",
                params![today_cutoff],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let total_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM history WHERE status = 'Completed'",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let avg_savings_pct: f64 = conn
            .query_row(
                "SELECT COALESCE(AVG(
                    CASE WHEN optimized_bytes > 0 AND original_bytes > 0
                         THEN (1.0 - (CAST(optimized_bytes AS REAL) / CAST(original_bytes AS REAL))) * 100.0
                         ELSE NULL END
                 ), 0.0)
                 FROM history WHERE status = 'Completed'",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0.0);

        Ok(crate::history::StatsSnapshot {
            today_savings_bytes: today_savings.max(0) as u64,
            today_optimized_count: today_count.max(0) as u64,
            total_optimized_count: total_count.max(0) as u64,
            average_savings_pct: avg_savings_pct,
        })
    }

    /// Build the complete local analytics payload used by the Statistics page.
    pub fn analytics(&self, range: AnalyticsRange) -> anyhow::Result<AnalyticsSnapshot> {
        let today = Local::now().date_naive();
        let start_date = range.days().map(|days| today - Duration::days(days - 1));
        let start = start_date.map(local_midnight_millis);
        let conn = self.conn.lock().expect("history conn poisoned");
        let entries = completed_entries_between(&conn, start, None)?;

        let (saved_bytes, optimized_count, input_bytes, average_savings_pct) =
            aggregate_entries(&entries);
        let savings_change_pct = match range.days() {
            Some(days) => {
                let current_start = start.expect("dated range has a start");
                let previous_start = local_midnight_millis(today - Duration::days(days * 2 - 1));
                let previous =
                    completed_entries_between(&conn, Some(previous_start), Some(current_start))?;
                let (previous_saved, _, _, _) = aggregate_entries(&previous);
                Some(percent_change(saved_bytes, previous_saved))
            }
            None => None,
        };

        let trend = build_trend(&entries, range, today, start_date);
        // The dashboard is about the file the user supplied, not the output
        // representation. This keeps optional WebP exports attributed to the
        // original PNG/JPEG source.
        let formats = build_breakdown(&entries, source_format);
        let presets = build_breakdown(&entries, |entry| entry.preset.clone());
        let sources = build_breakdown(&entries, |entry| entry.source.clone());
        let biggest_wins = build_biggest_wins(entries);

        Ok(AnalyticsSnapshot {
            saved_bytes,
            optimized_count,
            input_bytes,
            average_savings_pct,
            savings_change_pct,
            trend,
            formats,
            presets,
            sources,
            biggest_wins,
        })
    }
}

fn local_midnight_millis(date: chrono::NaiveDate) -> i64 {
    Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("valid midnight"))
        .single()
        .or_else(|| {
            Local
                .from_local_datetime(&date.and_hms_opt(12, 0, 0).expect("valid noon"))
                .single()
        })
        .expect("local time must resolve")
        .timestamp_millis()
}

fn completed_entries_between(
    conn: &Connection,
    start: Option<i64>,
    end: Option<i64>,
) -> anyhow::Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, input_path, output_path, format, original_bytes, optimized_bytes,
                engine, preset, status, error_message, started_at, completed_at,
                thumbnail_path, dssim, margin_pct, source
         FROM history
         WHERE status = 'Completed'
           AND (?1 IS NULL OR completed_at >= ?1)
           AND (?2 IS NULL OR completed_at < ?2)",
    )?;
    let rows = stmt.query_map(params![start, end], |row| {
        Ok(HistoryEntry {
            id: row.get(0)?,
            input_path: row.get(1)?,
            output_path: row.get(2)?,
            format: row.get(3)?,
            original_bytes: row.get::<_, i64>(4)? as u64,
            optimized_bytes: row.get::<_, Option<i64>>(5)?.map(|value| value as u64),
            engine: row.get(6)?,
            preset: row.get(7)?,
            status: HistoryStatus::Completed,
            error_message: row.get(9)?,
            started_at: row.get(10)?,
            completed_at: row.get(11)?,
            thumbnail_path: row.get(12)?,
            dssim: row.get(13)?,
            margin_pct: row.get(14)?,
            source: row.get(15)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn aggregate_entries(entries: &[HistoryEntry]) -> (u64, u64, u64, f64) {
    let mut saved = 0_u64;
    let mut input = 0_u64;
    let mut percent_total = 0.0;
    let mut count = 0_u64;
    for entry in entries {
        let optimized = entry.optimized_bytes.unwrap_or(entry.original_bytes);
        saved = saved.saturating_add(entry.original_bytes.saturating_sub(optimized));
        input = input.saturating_add(entry.original_bytes);
        if entry.original_bytes > 0 {
            percent_total += (1.0 - optimized as f64 / entry.original_bytes as f64) * 100.0;
        }
        count += 1;
    }
    let average = if count > 0 {
        percent_total / count as f64
    } else {
        0.0
    };
    (saved, count, input, average)
}

fn percent_change(current: u64, previous: u64) -> f64 {
    if previous == 0 {
        if current == 0 {
            0.0
        } else {
            100.0
        }
    } else {
        ((current as f64 - previous as f64) / previous as f64) * 100.0
    }
}

fn build_trend(
    entries: &[HistoryEntry],
    range: AnalyticsRange,
    today: chrono::NaiveDate,
    start_date: Option<chrono::NaiveDate>,
) -> Vec<SavingsTrendPoint> {
    let monthly = range == AnalyticsRange::All;
    let mut points: BTreeMap<String, SavingsTrendPoint> = BTreeMap::new();
    for entry in entries {
        let Some(completed_at) = entry.completed_at else {
            continue;
        };
        let Some(date) = Local.timestamp_millis_opt(completed_at).single() else {
            continue;
        };
        let period = if monthly {
            format!("{:04}-{:02}", date.year(), date.month())
        } else {
            date.format("%Y-%m-%d").to_string()
        };
        let point = points
            .entry(period.clone())
            .or_insert_with(|| SavingsTrendPoint {
                period,
                ..Default::default()
            });
        point.saved_bytes = point.saved_bytes.saturating_add(
            entry
                .original_bytes
                .saturating_sub(entry.optimized_bytes.unwrap_or(entry.original_bytes)),
        );
        point.optimized_count += 1;
    }

    if let Some(start) = start_date {
        let days = range.days().expect("dated range has days");
        for offset in 0..days {
            let date = start + Duration::days(offset);
            let period = date.format("%Y-%m-%d").to_string();
            points.entry(period.clone()).or_insert(SavingsTrendPoint {
                period,
                ..Default::default()
            });
        }
    } else if let Some(first) = entries.iter().filter_map(|entry| entry.completed_at).min() {
        if let Some(first_date) = Local.timestamp_millis_opt(first).single() {
            let mut year = first_date.year();
            let mut month = first_date.month();
            while (year, month) <= (today.year(), today.month()) {
                let period = format!("{year:04}-{month:02}");
                points.entry(period.clone()).or_insert(SavingsTrendPoint {
                    period,
                    ..Default::default()
                });
                if month == 12 {
                    year += 1;
                    month = 1;
                } else {
                    month += 1;
                }
            }
        }
    }
    points.into_values().collect()
}

fn build_breakdown(
    entries: &[HistoryEntry],
    key: impl Fn(&HistoryEntry) -> String,
) -> Vec<StatsBreakdown> {
    let mut grouped: BTreeMap<String, StatsBreakdown> = BTreeMap::new();
    for entry in entries {
        let name = key(entry);
        let item = grouped
            .entry(name.clone())
            .or_insert_with(|| StatsBreakdown {
                key: name,
                ..Default::default()
            });
        item.saved_bytes = item.saved_bytes.saturating_add(
            entry
                .original_bytes
                .saturating_sub(entry.optimized_bytes.unwrap_or(entry.original_bytes)),
        );
        item.optimized_count += 1;
    }
    let mut breakdown: Vec<_> = grouped.into_values().collect();
    breakdown.sort_by_key(|item| std::cmp::Reverse(item.saved_bytes));
    breakdown
}

fn source_format(entry: &HistoryEntry) -> String {
    std::path::Path::new(&entry.input_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.is_empty())
        .map(|extension| extension.to_ascii_uppercase())
        .unwrap_or_else(|| entry.format.to_ascii_uppercase())
}

fn build_biggest_wins(mut entries: Vec<HistoryEntry>) -> Vec<BiggestWin> {
    entries.sort_by_key(|entry| {
        std::cmp::Reverse(
            entry
                .original_bytes
                .saturating_sub(entry.optimized_bytes.unwrap_or(entry.original_bytes)),
        )
    });
    entries
        .into_iter()
        .take(5)
        .map(|entry| {
            let optimized_bytes = entry.optimized_bytes.unwrap_or(entry.original_bytes);
            let saved_bytes = entry.original_bytes.saturating_sub(optimized_bytes);
            BiggestWin {
                input_path: entry.input_path,
                output_exists: entry
                    .output_path
                    .as_ref()
                    .is_some_and(|path| std::path::Path::new(path).is_file()),
                output_path: entry.output_path,
                thumbnail_path: entry.thumbnail_path,
                original_bytes: entry.original_bytes,
                optimized_bytes,
                saved_bytes,
                savings_pct: if entry.original_bytes == 0 {
                    0.0
                } else {
                    saved_bytes as f64 / entry.original_bytes as f64 * 100.0
                },
                format: entry.format,
                preset: entry.preset,
                engine: entry.engine,
                completed_at: entry.completed_at.unwrap_or(entry.started_at),
            }
        })
        .collect()
}

impl HistoryRepository for SqliteHistory {
    fn record(&self, result: &CompressionResult) -> anyhow::Result<i64> {
        let entry: HistoryEntry = result.into();
        self.insert(&entry)
    }

    fn recent(&self, limit: u32) -> anyhow::Result<Vec<CompressionResult>> {
        // Map HistoryEntry → CompressionResult for the trait return type.
        let entries = SqliteHistory::recent(self, limit)?;
        Ok(entries
            .into_iter()
            .filter_map(|e| {
                let output = e.output_path?;
                Some(CompressionResult {
                    engine: e.engine.unwrap_or_else(|| "unknown".to_string()),
                    output_path: PathBuf::from(output),
                    format: ImageFormat::from_str(&e.format).ok()?,
                    original_bytes: e.original_bytes,
                    optimized_bytes: e.optimized_bytes.unwrap_or(0),
                    dssim: e.dssim,
                    duration_ms: 0,
                })
            })
            .collect())
    }

    fn record_via_entry(&self, entry: &HistoryEntry) -> anyhow::Result<i64> {
        // Preserve queue metadata such as the original source path, preset,
        // MCP provenance, timing, and quality details. The trait default
        // rebuilds a CompressionResult and would lose those fields.
        self.insert(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history_entry(
        input: &str,
        format: &str,
        preset: &str,
        original_bytes: u64,
        optimized_bytes: u64,
        completed_at: i64,
    ) -> HistoryEntry {
        HistoryEntry {
            id: 0,
            input_path: input.to_string(),
            output_path: Some(format!("{input}.out")),
            format: format.to_string(),
            original_bytes,
            optimized_bytes: Some(optimized_bytes),
            engine: Some("test".to_string()),
            preset: preset.to_string(),
            source: "Desktop".to_string(),
            status: HistoryStatus::Completed,
            error_message: None,
            started_at: completed_at - 20,
            completed_at: Some(completed_at),
            dssim: None,
            margin_pct: None,
            thumbnail_path: None,
        }
    }

    #[test]
    fn open_in_memory_and_migrate() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        // Migrations should be idempotent.
        store.migrate().unwrap();
    }

    #[test]
    fn insert_and_recent_round_trip() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let entry = HistoryEntry {
            id: 0,
            input_path: "/tmp/a.png".to_string(),
            output_path: Some("/tmp/a-framepress.png".to_string()),
            format: "PNG".to_string(),
            original_bytes: 1000,
            optimized_bytes: Some(500),
            engine: Some("oxipng".to_string()),
            preset: "website".to_string(),
            source: "Desktop".to_string(),
            status: HistoryStatus::Completed,
            error_message: None,
            started_at: 1_000_000,
            completed_at: Some(1_001_000),
            thumbnail_path: None,
            dssim: Some(0.0),
            margin_pct: Some(34.0),
        };
        let id = store.insert(&entry).unwrap();
        assert!(id > 0);
        let recent = store.recent(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].input_path, "/tmp/a.png");
        assert_eq!(recent[0].optimized_bytes, Some(500));
    }

    #[test]
    fn record_via_entry_preserves_mcp_provenance() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let entry = history_entry(
            "/tmp/from-agent.png",
            "PNG",
            "maximum_compression",
            1_000,
            400,
            Local::now().timestamp_millis(),
        );
        let entry = HistoryEntry {
            source: "Agent (MCP): Codex".to_string(),
            ..entry
        };

        HistoryRepository::record_via_entry(&store, &entry).unwrap();
        let recent = store.recent(1).unwrap();
        assert_eq!(recent[0].input_path, "/tmp/from-agent.png");
        assert_eq!(recent[0].source, "Agent (MCP): Codex");
    }

    #[test]
    fn stats_reports_zeros_on_empty_db() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let snap = store.stats().unwrap();
        assert_eq!(snap.today_savings_bytes, 0);
        assert_eq!(snap.total_optimized_count, 0);
    }

    #[test]
    fn analytics_zero_fills_the_seven_day_trend_for_an_empty_database() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();

        let analytics = store.analytics(AnalyticsRange::Days7).unwrap();
        assert_eq!(analytics.optimized_count, 0);
        assert_eq!(analytics.trend.len(), 7);
        assert!(analytics.trend.iter().all(|point| point.saved_bytes == 0));
    }

    #[test]
    fn analytics_groups_completed_entries_and_keeps_webp_exports() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let now = Local::now().timestamp_millis();
        store
            .insert(&history_entry(
                "/tmp/a.png",
                "PNG",
                "website",
                1_000,
                400,
                now,
            ))
            .unwrap();
        // Explicit WebP exports are completed history entries and intentionally
        // participate in the user's selected savings accounting.
        store
            .insert(&history_entry(
                "/tmp/a.png",
                "WEBP",
                "maximum_compression",
                2_000,
                500,
                now,
            ))
            .unwrap();
        let mut cancelled = history_entry("/tmp/cancelled.png", "PNG", "email", 4_000, 100, now);
        cancelled.status = HistoryStatus::Cancelled;
        store.insert(&cancelled).unwrap();

        let analytics = store.analytics(AnalyticsRange::Days7).unwrap();
        assert_eq!(analytics.optimized_count, 2);
        assert_eq!(analytics.input_bytes, 3_000);
        assert_eq!(analytics.saved_bytes, 2_100);
        assert_eq!(analytics.formats.len(), 1);
        assert_eq!(analytics.formats[0].key, "PNG");
        assert_eq!(analytics.presets.len(), 2);
        assert_eq!(analytics.biggest_wins[0].format, "WEBP");
        assert_eq!(analytics.biggest_wins[0].saved_bytes, 1_500);
        assert_eq!(
            analytics
                .trend
                .iter()
                .map(|point| point.optimized_count)
                .sum::<u64>(),
            2
        );
    }

    #[test]
    fn stats_uses_the_current_local_calendar_day() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let today = Local::now().timestamp_millis();
        let yesterday = local_midnight_millis(Local::now().date_naive() - Duration::days(1)) + 1;
        store
            .insert(&history_entry(
                "/tmp/today.png",
                "PNG",
                "website",
                100,
                20,
                today,
            ))
            .unwrap();
        store
            .insert(&history_entry(
                "/tmp/yesterday.png",
                "PNG",
                "website",
                100,
                20,
                yesterday,
            ))
            .unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.today_optimized_count, 1);
        assert_eq!(stats.today_savings_bytes, 80);
    }
}

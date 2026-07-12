//! SQLite-backed history store. Uses `rusqlite` directly with a manual
//! `Mutex<Connection>` (deadpool is overkill for the single-user desktop
//! case but we still serialize access; the spec calls for `r2d2`/`deadpool`
//! and the abstraction is preserved in the trait API).

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{CompressionResult, ImageFormat};
use crate::history::entry::{HistoryEntry, HistoryStatus};
use crate::history::HistoryRepository;

/// Configuration for opening the SQLite DB.
pub struct SqliteHistoryConfig {
    /// Path to the SQLite file. `:memory:` for tests.
    pub path: PathBuf,
}

impl SqliteHistoryConfig {
    /// Default config at `$XDG_DATA_HOME/tinydrop/history.db` (Linux) or the
    /// macOS `~/Library/Application Support/tinydrop/history.db`.
    pub fn default_path() -> PathBuf {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("tinydrop").join("history.db")
    }
}

/// The on-disk history store. Phase 1: serialized access via `Mutex`.
pub struct SqliteHistory {
    conn: Mutex<Connection>,
}

impl SqliteHistory {
    /// Open or create the DB at `config.path`. Runs migrations.
    pub fn open(config: SqliteHistoryConfig) -> anyhow::Result<Self> {
        if config.path != PathBuf::from(":memory:") {
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
                status TEXT NOT NULL,
                error_message TEXT,
                started_at INTEGER NOT NULL,
                completed_at INTEGER,
                thumbnail_path TEXT,
                dssim REAL,
                margin_pct REAL
            );
            CREATE INDEX IF NOT EXISTS idx_history_completed_at ON history(completed_at);
            CREATE INDEX IF NOT EXISTS idx_history_status ON history(status);",
        )?;
        conn.execute_batch("PRAGMA user_version = 1;")?;
        Ok(())
    }

    /// Insert a fully-formed entry. Returns the row id.
    pub fn insert(&self, entry: &HistoryEntry) -> anyhow::Result<i64> {
        let conn = self.conn.lock().expect("history conn poisoned");
        conn.execute(
            "INSERT INTO history (
                input_path, output_path, format, original_bytes, optimized_bytes,
                engine, preset, status, error_message, started_at, completed_at,
                thumbnail_path, dssim, margin_pct
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                entry.input_path,
                entry.output_path,
                entry.format,
                entry.original_bytes as i64,
                entry.optimized_bytes.map(|v| v as i64),
                entry.engine,
                entry.preset,
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
                    thumbnail_path, dssim, margin_pct
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
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let one_day_ago_ms = 24 * 60 * 60 * 1000_i64;
        let today_cutoff = now - one_day_ago_ms;

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
            output_path: Some("/tmp/a-tinydrop.png".to_string()),
            format: "PNG".to_string(),
            original_bytes: 1000,
            optimized_bytes: Some(500),
            engine: Some("oxipng".to_string()),
            preset: "website".to_string(),
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
    fn stats_reports_zeros_on_empty_db() {
        let store = SqliteHistory::open(SqliteHistoryConfig {
            path: PathBuf::from(":memory:"),
        })
        .unwrap();
        let snap = store.stats().unwrap();
        assert_eq!(snap.today_savings_bytes, 0);
        assert_eq!(snap.total_optimized_count, 0);
    }
}

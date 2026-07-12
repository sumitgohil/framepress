//! SQLite-backed optimization history. Phase 1 keeps a single on-disk DB at
//! the OS app-data dir; the trait abstraction lets tests inject in-memory
//! fakes without touching SQLite.

mod entry;
mod sqlite;
mod stats;

pub use entry::{HistoryEntry, HistoryStatus};
pub use sqlite::{SqliteHistory, SqliteHistoryConfig};
pub use stats::{StatsAggregator, StatsSnapshot};

use std::str::FromStr;

use crate::domain::CompressionResult;

/// Repository abstraction so the optimizer and queue can record results
/// without binding to SQLite. Implementations may use any backing store.
pub trait HistoryRepository: Send + Sync {
    /// Persist a successful optimization result.
    fn record(&self, result: &CompressionResult) -> anyhow::Result<i64>;

    /// Most recent results, newest first.
    fn recent(&self, limit: u32) -> anyhow::Result<Vec<CompressionResult>>;

    /// Persist a fully-formed [`HistoryEntry`] (e.g. one already augmented
    /// with status, preset, margin from the queue processor).
    fn record_via_entry(&self, entry: &HistoryEntry) -> anyhow::Result<i64> {
        // Default impl: derive a CompressionResult from the entry.
        let result = crate::domain::CompressionResult {
            engine: entry
                .engine
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            output_path: std::path::PathBuf::from(
                entry
                    .output_path
                    .clone()
                    .unwrap_or_else(|| entry.input_path.clone()),
            ),
            format: crate::domain::ImageFormat::from_str(&entry.format)
                .unwrap_or(crate::domain::ImageFormat::Png),
            original_bytes: entry.original_bytes,
            optimized_bytes: entry.optimized_bytes.unwrap_or(0),
            dssim: entry.dssim,
            duration_ms: 0,
        };
        self.record(&result)
    }
}

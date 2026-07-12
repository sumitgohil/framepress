//! Repository traits. The concrete implementations live in `tinydrop-core`'s
//! `history` module (SQLite in Branch 6).

use crate::domain::CompressionResult;

/// Read/write access to the optimization history.
///
/// Implementations may use any backing store. The Phase 1 implementation is
/// SQLite via `deadpool-sqlite`; the trait exists so the optimizer and queue
/// layers can be tested against in-memory fakes.
pub trait HistoryRepository: Send + Sync {
    /// Persist a successful optimization result.
    fn record(&self, result: &CompressionResult) -> anyhow::Result<i64>;

    /// Most recent results, newest first. `limit` caps the returned row count.
    fn recent(&self, limit: u32) -> anyhow::Result<Vec<CompressionResult>>;
}

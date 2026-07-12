//! Aggregate stats. Phase 1 keeps the math in a thin struct; the actual
//! aggregation lives in the SQLite store.

use serde::{Deserialize, Serialize};

/// Aggregate stats snapshot, surfaced to the UI on demand.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsSnapshot {
    /// Total bytes saved today (original - optimized) for completed jobs.
    pub today_savings_bytes: u64,
    /// Number of images optimized today.
    pub today_optimized_count: u64,
    /// Cumulative count of completed optimizations.
    pub total_optimized_count: u64,
    /// Average savings percentage across all completed jobs.
    pub average_savings_pct: f64,
}

/// Aggregation helper. Phase 1 is a pass-through; future branches may compute
/// derived metrics (e.g. week-over-week delta).
pub struct StatsAggregator;

impl StatsAggregator {
    /// Construct a new aggregator. Cheap.
    pub fn new() -> Self {
        Self
    }

    /// Pass-through helper; lets the queue processor call a single API even
    /// when the implementation is trivial.
    pub fn snapshot(&self, snap: StatsSnapshot) -> StatsSnapshot {
        snap
    }
}

impl Default for StatsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

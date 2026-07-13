//! Aggregate stats. Phase 1 keeps the math in a thin struct; the actual
//! aggregation lives in the SQLite store.

use serde::{Deserialize, Serialize};

/// Date span used by the analytics dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum AnalyticsRange {
    #[serde(rename = "7d")]
    Days7,
    #[serde(rename = "30d")]
    Days30,
    #[serde(rename = "all")]
    All,
}

impl AnalyticsRange {
    /// Number of local calendar days represented by a bounded range.
    pub const fn days(self) -> Option<i64> {
        match self {
            Self::Days7 => Some(7),
            Self::Days30 => Some(30),
            Self::All => None,
        }
    }
}

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

/// A single zero-filled time bucket used by the savings chart.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct SavingsTrendPoint {
    /// ISO local calendar day (`YYYY-MM-DD`) or month (`YYYY-MM`).
    pub period: String,
    pub saved_bytes: u64,
    pub optimized_count: u64,
}

/// A ranked format or preset aggregate.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct StatsBreakdown {
    pub key: String,
    pub saved_bytes: u64,
    pub optimized_count: u64,
}

/// A completed file with one of the largest savings in the requested range.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BiggestWin {
    pub input_path: String,
    pub output_path: Option<String>,
    pub output_exists: bool,
    pub thumbnail_path: Option<String>,
    pub original_bytes: u64,
    pub optimized_bytes: u64,
    pub saved_bytes: u64,
    pub savings_pct: f64,
    pub format: String,
    pub preset: String,
    pub engine: Option<String>,
    pub completed_at: i64,
}

/// Complete analytics payload for the Statistics page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct AnalyticsSnapshot {
    pub saved_bytes: u64,
    pub optimized_count: u64,
    pub input_bytes: u64,
    pub average_savings_pct: f64,
    /// Percentage change in bytes saved versus the preceding equivalent period.
    /// `None` for the all-time range or when no comparison period exists.
    pub savings_change_pct: Option<f64>,
    pub trend: Vec<SavingsTrendPoint>,
    pub formats: Vec<StatsBreakdown>,
    pub presets: Vec<StatsBreakdown>,
    pub sources: Vec<StatsBreakdown>,
    pub biggest_wins: Vec<BiggestWin>,
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

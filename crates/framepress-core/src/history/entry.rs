//! History entry shape. Mirrors the SQLite schema so the queue/stats layers
//! can record and query without depending on the SQLite crate directly.

use serde::{Deserialize, Serialize};

use crate::domain::CompressionResult;

/// Terminal status of a history entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatus {
    /// Optimization finished and the quality gate passed.
    Completed,
    /// Optimization finished but the quality gate rejected all candidates.
    Failed,
    /// User cancelled before completion.
    Cancelled,
}

impl HistoryStatus {
    /// String form for SQLite.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    /// Parse a stored SQLite string back into the enum.
    #[allow(clippy::should_implement_trait)]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Completed" => Some(Self::Completed),
            "Failed" => Some(Self::Failed),
            "Cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// A row from the `history` table. Includes everything the UI needs to render
/// the History list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Row ID assigned by SQLite on insert.
    pub id: i64,
    /// Path of the input file.
    pub input_path: String,
    /// Path of the optimized output, if any.
    pub output_path: Option<String>,
    /// Detected image format.
    pub format: String,
    /// Original file size in bytes.
    pub original_bytes: u64,
    /// Optimized file size in bytes, if optimization succeeded.
    pub optimized_bytes: Option<u64>,
    /// Engine that won the adaptive scoring pass.
    pub engine: Option<String>,
    /// Active preset at the time of optimization.
    pub preset: String,
    /// Origin of the work, such as `Desktop` or `Agent (MCP): Codex`.
    pub source: String,
    /// Terminal status.
    pub status: HistoryStatus,
    /// Error message, if status != Completed.
    pub error_message: Option<String>,
    /// Started-at timestamp, unix millis.
    pub started_at: i64,
    /// Completed-at timestamp, unix millis.
    pub completed_at: Option<i64>,
    /// DSSIM measured against the original.
    pub dssim: Option<f64>,
    /// Margin over the runner-up engine (the hero-moment number).
    pub margin_pct: Option<f64>,
    /// Optional thumbnail path for the History row UI.
    pub thumbnail_path: Option<String>,
}

/// Convert a domain `CompressionResult` into a `HistoryEntry` ready for
/// insertion into SQLite.
impl From<&CompressionResult> for HistoryEntry {
    fn from(r: &CompressionResult) -> Self {
        Self {
            id: 0, // assigned by the DB on insert
            input_path: r.output_path.to_string_lossy().to_string(),
            output_path: Some(r.output_path.to_string_lossy().to_string()),
            format: r.format.to_string(),
            original_bytes: r.original_bytes,
            optimized_bytes: Some(r.optimized_bytes),
            engine: Some(r.engine.clone()),
            preset: "unknown".to_string(), // supplied by caller (preset is not on the result)
            source: "Desktop".to_string(),
            status: HistoryStatus::Completed,
            error_message: None,
            started_at: 0,
            completed_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0),
            ),
            dssim: r.dssim,
            margin_pct: None,
            thumbnail_path: None,
        }
    }
}

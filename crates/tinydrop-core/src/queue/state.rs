//! Queue state types: items, statuses, stats. These are the public shape
//! the desktop shell serializes over IPC.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::domain::{CompressionPreset, ImageFormat, ScoredCandidate};

/// Per-item job status. Mirrors the IPC `QueueItemStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Waiting for a worker.
    Pending,
    /// Currently being optimized.
    Running,
    /// Optimization finished and quality gate passed.
    Completed,
    /// Optimization finished but the quality gate rejected all candidates.
    Failed,
    /// User cancelled before completion.
    Cancelled,
}

/// A single queue item, as serialized to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// Stable job ID. Content-addressed in Branch 6.
    pub id: String,
    /// Path of the input file.
    pub input_path: PathBuf,
    /// Path of the optimized output file. `None` until completion.
    pub output_path: Option<PathBuf>,
    /// Detected input format.
    pub format: Option<ImageFormat>,
    /// Active preset.
    pub preset: CompressionPreset,
    /// Current status.
    pub status: JobStatus,
    /// Original file size in bytes. `None` until the worker picks it up.
    pub original_bytes: Option<u64>,
    /// Optimized file size in bytes. `None` until completion.
    pub optimized_bytes: Option<u64>,
    /// Engine that won the adaptive scoring pass. `None` until completion.
    pub engine: Option<String>,
    /// DSSIM measured against the original. `None` until scored.
    pub dssim: Option<f64>,
    /// Savings percentage, 0..100. `None` until completion.
    pub savings_pct: Option<f64>,
    /// Margin over the runner-up (for the "WebP beat PNG by 34%" toast).
    /// `None` until completion, or when there was no runner-up.
    pub margin_pct: Option<f64>,
    /// Error message if status == Failed or Cancelled.
    pub error_message: Option<String>,
    /// Per-candidate engine log: name, size, dssim, passed. `None` until scored.
    pub candidates_log: Option<Vec<CandidateLogEntry>>,
    /// Started at, unix millis. `None` until the worker picks it up.
    pub started_at: Option<i64>,
    /// Completed at, unix millis. `None` until completion.
    pub completed_at: Option<i64>,
}

/// Per-engine log entry exposed in the expandable log of a QueueCard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateLogEntry {
    pub engine: String,
    pub output_bytes: u64,
    pub dssim: Option<f64>,
    pub passed_gate: bool,
}

impl From<&ScoredCandidate> for CandidateLogEntry {
    fn from(s: &ScoredCandidate) -> Self {
        Self {
            engine: s.result.engine.clone(),
            output_bytes: s.result.optimized_bytes,
            dssim: s.result.dssim,
            passed_gate: s.passed_quality_gate,
        }
    }
}

/// Aggregate stats for the Queue UI's header.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// Number of items in any state.
    pub total: usize,
    /// Items currently running.
    pub running: usize,
    /// Items waiting for a worker.
    pub pending: usize,
    /// Items that completed successfully.
    pub completed: usize,
    /// Items that failed or were cancelled.
    pub failed: usize,
}

/// Internal counters shared between the processor and the snapshot that
/// goes to the UI.
#[derive(Debug, Default)]
pub(crate) struct AtomicStats {
    pub running: AtomicUsize,
    pub pending: AtomicUsize,
    pub completed: AtomicUsize,
    pub failed: AtomicUsize,
}

impl AtomicStats {
    pub(crate) fn snapshot(&self, total: usize) -> QueueStats {
        QueueStats {
            total,
            running: self.running.load(Ordering::SeqCst),
            pending: self.pending.load(Ordering::SeqCst),
            completed: self.completed.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
        }
    }
}

/// Public, mutex-protected queue state. The processor mutates it inside an
/// `Arc<Mutex<...>>`; the UI snapshots it for rendering.
#[derive(Debug, Default)]
pub struct QueueState {
    /// All known items, in enqueue order.
    pub(crate) items: Vec<QueueItem>,
}

impl QueueState {
    /// Snapshot of the current items, cloned for serialization.
    pub fn snapshot(&self) -> Vec<QueueItem> {
        self.items.clone()
    }

    /// Append a new pending item.
    pub(crate) fn push_pending(&mut self, item: QueueItem) {
        self.items.push(item);
    }
}

/// Construct a new QueueItem in `Pending` state.
pub fn new_pending_item(
    id: String,
    input_path: PathBuf,
    format: Option<ImageFormat>,
    preset: CompressionPreset,
) -> QueueItem {
    QueueItem {
        id,
        input_path,
        output_path: None,
        format,
        preset,
        status: JobStatus::Pending,
        original_bytes: None,
        optimized_bytes: None,
        engine: None,
        dssim: None,
        savings_pct: None,
        margin_pct: None,
        error_message: None,
        candidates_log: None,
        started_at: None,
        completed_at: None,
    }
}

/// Public constructor the processor uses to keep state out of this file's
/// public surface.
pub(crate) fn mark_running(item: &mut QueueItem) {
    item.status = JobStatus::Running;
    item.started_at = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0),
    );
}

// Re-export Arc so callers don't need a separate import.
pub(crate) type SharedQueueState = Arc<std::sync::Mutex<QueueState>>;

/// Helper to keep this module's compile-test surface tight.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pending_item_has_correct_defaults() {
        let item = new_pending_item(
            "job-1".to_string(),
            PathBuf::from("/tmp/in.png"),
            Some(ImageFormat::Png),
            CompressionPreset::Website,
        );
        assert_eq!(item.status, JobStatus::Pending);
        assert!(item.started_at.is_none());
        assert!(item.completed_at.is_none());
        assert!(item.output_path.is_none());
    }

    #[test]
    fn atomic_stats_snapshot_reflects_current_counts() {
        let s = AtomicStats::default();
        s.running.fetch_add(2, Ordering::SeqCst);
        s.pending.fetch_add(3, Ordering::SeqCst);
        s.completed.fetch_add(4, Ordering::SeqCst);
        let snap = s.snapshot(9);
        assert_eq!(snap.running, 2);
        assert_eq!(snap.pending, 3);
        assert_eq!(snap.completed, 4);
        assert_eq!(snap.total, 9);
    }
}

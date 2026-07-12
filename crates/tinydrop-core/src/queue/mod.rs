//! In-memory image optimization queue. Phase 1 keeps state in memory; the
//! SQLite-backed persistence lands in Branch 6.
//!
//! Concurrency model:
//! - One [`QueueProcessor`] per app, holding a shared `Arc<Mutex<QueueState>>`
//! - Each enqueued path becomes a [`QueueItem`] in `pending` state
//! - A worker tokio task pops pending items up to `max_concurrency`, runs the
//!   adaptive optimizer on each, and emits progress events
//! - Cancellation: per-item `CancellationToken`; cancel sets the token, the
//!   optimizer checks it between candidate scoring rounds
//! - Pause/resume: a global flag. When paused, the worker skips pending items
//!
//! See `ARCHITECTURE.md` for the async/rayon boundary and the rationale.

mod cancel;
mod processor;
mod state;

pub use cancel::CancellationToken;
pub use processor::QueueProcessor;
pub use state::{JobStatus, QueueItem, QueueState, QueueStats};

pub use crate::domain::CompressionPreset;

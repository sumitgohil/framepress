//! AppContext — the manual DI container for the desktop shell.
//!
//! Wraps the `tinydrop-core` [`AdaptiveOptimizer`] and (in later branches)
//! the queue processor, history repository, and settings store. Held in
//! `tauri::State<AppContext>` and fetched by every command.

use std::sync::Arc;

use anyhow::Context as _;
use tokio::sync::Mutex;
use tracing::info;

use tinydrop_core::{
    default_registry,
    history::{HistoryRepository, SqliteHistory, SqliteHistoryConfig},
    AdaptiveOptimizer, CompressionPreset, ImageFormat, QueueItem, QueueProcessor, QueueStats,
    ScoredCandidate,
};

/// The application context. Cheap to clone — all heavy state is behind
/// `Arc`/`Mutex` or trait objects that are themselves `Send + Sync`.
#[derive(Clone)]
pub struct AppContext {
    /// Adaptive optimizer — the product's headline component.
    optimizer: Arc<AdaptiveOptimizer>,
    /// Active preset (mutable, updated when the user changes the dropdown).
    active_preset: Arc<Mutex<CompressionPreset>>,
    /// Background queue processor.
    queue: Arc<QueueProcessor>,
    /// SQLite-backed history store.
    history: Arc<SqliteHistory>,
}

impl AppContext {
    /// Construct a new context. Wires the default engine registry.
    pub fn build() -> anyhow::Result<Self> {
        // Presets own their visual-quality budgets. Do not apply the old
        // global default gate here: it is stricter than Email and rejects the
        // lossy WebP candidate that the preset intentionally allows.
        let optimizer = Arc::new(AdaptiveOptimizer::new(default_registry()));
        let history = Arc::new(SqliteHistory::open(SqliteHistoryConfig {
            path: SqliteHistoryConfig::default_path(),
        })?);
        let history_repo: Arc<dyn HistoryRepository> = history.clone();
        let queue = Arc::new(QueueProcessor::new(optimizer.clone()).with_history(history_repo));
        info!(
            engines = optimizer.engines().len(),
            "TinyDrop context initialized",
        );
        Ok(Self {
            optimizer,
            active_preset: Arc::new(Mutex::new(CompressionPreset::Website)),
            queue,
            history,
        })
    }

    /// The configured optimizer (cheap to clone — it's already in an `Arc`).
    pub fn optimizer(&self) -> Arc<AdaptiveOptimizer> {
        self.optimizer.clone()
    }

    /// The background queue processor.
    pub fn queue(&self) -> Arc<QueueProcessor> {
        self.queue.clone()
    }

    /// The history store.
    pub fn history(&self) -> Arc<SqliteHistory> {
        self.history.clone()
    }

    /// The user's currently-selected preset.
    pub async fn active_preset(&self) -> CompressionPreset {
        *self.active_preset.lock().await
    }

    /// Update the active preset. Returns the new value.
    pub async fn set_active_preset(&self, preset: CompressionPreset) -> CompressionPreset {
        let mut guard = self.active_preset.lock().await;
        *guard = preset;
        *guard
    }

    /// Convenience: run the adaptive optimizer on a single input, blocking
    /// the current thread. Tauri commands wrap this in `spawn_blocking`.
    pub fn run_optimizer(
        &self,
        input: &std::path::Path,
        preset: CompressionPreset,
        output: &std::path::Path,
    ) -> anyhow::Result<ScoredCandidate> {
        self.optimizer
            .optimize(input, preset, output)
            .context("optimizer failed")
    }

    /// Detect an image's format from a file path.
    pub fn detect_format(&self, path: &std::path::Path) -> anyhow::Result<ImageFormat> {
        Ok(tinydrop_core::optimizer::detect_format(path)?)
    }

    /// Snapshot the current queue items.
    pub fn queue_snapshot(&self) -> Vec<QueueItem> {
        self.queue.snapshot()
    }

    /// Aggregate queue stats.
    pub fn queue_stats(&self) -> QueueStats {
        self.queue.stats()
    }
}

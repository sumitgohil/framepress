//! The QueueProcessor — runs QueueItems through the adaptive optimizer with
//! bounded concurrency, emits progress events, and respects cancel/pause.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::domain::{CompressionPreset, CompressionResult, ScoredCandidate};
use crate::errors::CoreError;
use crate::history::{HistoryEntry, HistoryRepository, HistoryStatus};
use crate::optimizer::AdaptiveOptimizer;
use crate::queue::cancel::CancellationToken;
use crate::queue::state::{
    new_pending_item, AtomicStats, CandidateLogEntry, JobStatus, QueueItem, QueueState,
    SharedQueueState,
};

/// Maximum number of jobs running concurrently.
const DEFAULT_MAX_CONCURRENCY: usize = 2;

/// Per-job handle held by the queue so cancel can find the token.
#[derive(Default)]
pub(crate) struct WorkerHandle {
    pub cancel: CancellationToken,
}

/// The queue processor. Cheap to clone (everything heavy is behind `Arc`).
#[derive(Clone)]
pub struct QueueProcessor {
    optimizer: Arc<AdaptiveOptimizer>,
    history: Option<Arc<dyn HistoryRepository>>,
    state: SharedQueueState,
    stats: Arc<AtomicStats>,
    paused: Arc<AtomicBool>,
    /// Wake up the worker when state changes (new item, resume, etc.).
    wake: Arc<Notify>,
    /// Tracks cancel tokens for in-flight jobs.
    workers: Arc<Mutex<std::collections::HashMap<String, WorkerHandle>>>,
    /// Ensures the long-lived worker is started only once.
    worker_started: Arc<AtomicBool>,
    max_concurrency: usize,
}

impl QueueProcessor {
    /// Construct a processor. Call [`Self::start`] from an active Tokio
    /// runtime to launch its worker task.
    pub fn new(optimizer: Arc<AdaptiveOptimizer>) -> Self {
        let state: SharedQueueState = Arc::new(Mutex::new(QueueState::default()));
        let stats = Arc::new(AtomicStats::default());
        let paused = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(Notify::new());
        let workers = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let worker_started = Arc::new(AtomicBool::new(false));

        let proc = Self {
            optimizer,
            history: None,
            state,
            stats,
            paused,
            wake,
            workers,
            worker_started,
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
        };

        proc
    }

    /// Start the queue worker. This must be called while a Tokio runtime is
    /// active; repeated calls are harmless.
    pub fn start(&self) {
        if self
            .worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.spawn_worker();
        }
    }

    /// Attach a history repository. Completed jobs are recorded here.
    pub fn with_history(mut self, history: Arc<dyn HistoryRepository>) -> Self {
        self.history = Some(history);
        self
    }

    /// Enqueue a single path. Returns the assigned job ID.
    pub fn enqueue(
        &self,
        input_path: PathBuf,
        preset: CompressionPreset,
    ) -> Result<String, CoreError> {
        if !input_path.is_file() {
            return Err(CoreError::InputNotFound(input_path));
        }
        if input_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with("-tinydrop"))
        {
            return Err(CoreError::AlreadyOptimized(input_path));
        }
        let id = format!("job-{}", next_id_seq(&input_path));
        let item = new_pending_item(id.clone(), input_path, None, preset);
        {
            let mut state = self.state.lock().expect("queue state poisoned");
            state.push_pending(item);
        }
        self.stats.pending.fetch_add(1, Ordering::SeqCst);
        self.wake.notify_one();
        info!(job_id = %id, "enqueued");
        Ok(id)
    }

    /// Record a completed, user-requested derivative without sending it back
    /// through the worker. Used for explicit exports such as a WebP copy.
    pub fn record_completed_export(
        &self,
        source_path: PathBuf,
        preset: CompressionPreset,
        result: CompressionResult,
    ) -> QueueItem {
        let completed_at = now_millis();
        let item = QueueItem {
            id: format!("export-{}", next_id_seq(&result.output_path)),
            // Display the created file in the queue, while history below
            // retains the source path that the user acted on.
            input_path: result.output_path.clone(),
            output_path: Some(result.output_path.clone()),
            format: Some(result.format),
            preset,
            status: JobStatus::Completed,
            original_bytes: Some(result.original_bytes),
            optimized_bytes: Some(result.optimized_bytes),
            engine: Some(result.engine.clone()),
            dssim: result.dssim,
            savings_pct: Some(result.savings_pct()),
            margin_pct: None,
            error_message: None,
            candidates_log: Some(vec![CandidateLogEntry {
                engine: result.engine.clone(),
                output_bytes: result.optimized_bytes,
                dssim: result.dssim,
                passed_gate: true,
            }]),
            started_at: Some(completed_at),
            completed_at: Some(completed_at),
        };
        {
            let mut state = self.state.lock().expect("queue state poisoned");
            state.push_pending(item.clone());
        }
        self.stats.completed.fetch_add(1, Ordering::SeqCst);

        if let Some(history) = &self.history {
            let entry = HistoryEntry {
                id: 0,
                input_path: source_path.to_string_lossy().to_string(),
                output_path: Some(result.output_path.to_string_lossy().to_string()),
                format: result.format.to_string(),
                original_bytes: result.original_bytes,
                optimized_bytes: Some(result.optimized_bytes),
                engine: Some(result.engine),
                preset: preset.label().to_string(),
                status: HistoryStatus::Completed,
                error_message: None,
                started_at: completed_at,
                completed_at: Some(completed_at),
                dssim: result.dssim,
                margin_pct: None,
                thumbnail_path: None,
            };
            if let Err(error) = history.record_via_entry(&entry) {
                warn!(error = %error, "failed to record WebP export in history");
            }
        }

        item
    }

    /// Cancel a queued or running job.
    pub fn cancel(&self, job_id: &str) {
        if let Some(handle) = self.workers.lock().expect("workers poisoned").get(job_id) {
            handle.cancel.cancel();
        }
        // For pending jobs without a handle, mark them cancelled immediately.
        {
            let mut state = self.state.lock().expect("queue state poisoned");
            if let Some(item) = state.items.iter_mut().find(|i| i.id == job_id) {
                if item.status == JobStatus::Pending {
                    item.status = JobStatus::Cancelled;
                    item.error_message = Some("cancelled by user".to_string());
                    item.completed_at = Some(now_millis());
                    self.stats.pending.fetch_sub(1, Ordering::SeqCst);
                    self.stats.failed.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        self.wake.notify_one();
    }

    /// Pause the worker. In-flight jobs run to completion; new jobs wait.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        info!("queue paused");
    }

    /// Resume processing.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        self.wake.notify_one();
        info!("queue resumed");
    }

    /// `true` if paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    /// Snapshot the current items for serialization to the UI.
    pub fn snapshot(&self) -> Vec<QueueItem> {
        self.state.lock().expect("queue state poisoned").snapshot()
    }

    /// Aggregate stats.
    pub fn stats(&self) -> crate::queue::QueueStats {
        let total = self.state.lock().expect("queue state poisoned").items.len();
        self.stats.snapshot(total)
    }

    /// Number of items currently in any state.
    pub fn len(&self) -> usize {
        self.state.lock().expect("queue state poisoned").items.len()
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Register a worker's cancel token. Called by the worker task on startup.
    #[allow(dead_code)]
    fn register_worker(&self, job_id: String, cancel: CancellationToken) {
        self.workers
            .lock()
            .expect("workers poisoned")
            .insert(job_id, WorkerHandle { cancel });
    }

    /// Unregister a worker's cancel token. Called by the worker task on exit.
    #[allow(dead_code)]
    fn unregister_worker(&self, job_id: &str) {
        self.workers
            .lock()
            .expect("workers poisoned")
            .remove(job_id);
    }

    /// Spawn the worker task. The worker is a long-lived async loop that
    /// pulls pending items and runs them.
    fn spawn_worker(&self) -> JoinHandle<()> {
        let state = self.state.clone();
        let stats = self.stats.clone();
        let paused = self.paused.clone();
        let wake = self.wake.clone();
        let optimizer = self.optimizer.clone();
        let workers = self.workers.clone();
        let history = self.history.clone();
        let max_concurrency = self.max_concurrency;

        tokio::spawn(async move {
            loop {
                let history = history.clone();
                let wake = wake.clone();
                // Pick the next pending job ID (if any).
                let next = {
                    let state_guard = state.lock().expect("queue state poisoned");
                    state_guard
                        .items
                        .iter()
                        .find(|i| i.status == JobStatus::Pending)
                        .map(|i| i.id.clone())
                };

                let Some(job_id) = next else {
                    // No work; wait for a wake signal.
                    wake.notified().await;
                    continue;
                };

                // Respect pause.
                if paused.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                    continue;
                }

                // Concurrency cap.
                let running = stats.running.load(Ordering::SeqCst);
                if running >= max_concurrency {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Promote pending -> running.
                {
                    let mut state_guard = state.lock().expect("queue state poisoned");
                    if let Some(item) = state_guard.items.iter_mut().find(|i| i.id == job_id) {
                        if item.status == JobStatus::Pending {
                            crate::queue::state::mark_running(item);
                            stats.pending.fetch_sub(1, Ordering::SeqCst);
                            stats.running.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                }

                // Spin up the per-job task.
                let state_for_job = state.clone();
                let stats_for_job = stats.clone();
                let workers_for_job = workers.clone();
                let opt_for_job = optimizer.clone();
                let wake_for_job = wake.clone();
                let job_id_for_task = job_id.clone();
                let cancel = CancellationToken::new();
                workers_for_job.lock().expect("workers poisoned").insert(
                    job_id.clone(),
                    WorkerHandle {
                        cancel: cancel.clone(),
                    },
                );

                tokio::spawn(async move {
                    let history_clone = history.clone();
                    let state_clone_for_history = state_for_job.clone();
                    let result = run_one_job(
                        opt_for_job.clone(),
                        state_for_job,
                        job_id_for_task.clone(),
                        cancel,
                    )
                    .await;
                    match result {
                        Ok(_) => {
                            stats_for_job.completed.fetch_add(1, Ordering::SeqCst);
                            // Persist to history on success.
                            if let Some(history) = history_clone {
                                let entry = state_clone_for_history
                                    .lock()
                                    .expect("queue state poisoned")
                                    .items
                                    .iter()
                                    .find(|i| i.id == job_id_for_task)
                                    .map(history_entry_from);
                                if let Some(entry) = entry {
                                    if let Err(e) = history.record_via_entry(&entry) {
                                        warn!(error = %e, "failed to record history entry");
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            stats_for_job.failed.fetch_add(1, Ordering::SeqCst);
                        }
                    }
                    stats_for_job.running.fetch_sub(1, Ordering::SeqCst);
                    workers_for_job
                        .lock()
                        .expect("workers poisoned")
                        .remove(&job_id_for_task);
                    wake_for_job.notify_one();
                });

                // Yield so the new task has a chance to acquire the lock.
                tokio::task::yield_now().await;
            }
        })
    }
}

async fn run_one_job(
    optimizer: Arc<AdaptiveOptimizer>,
    state: SharedQueueState,
    job_id: String,
    cancel: CancellationToken,
) -> Result<(), CoreError> {
    // Snapshot the input/preset under the lock.
    let (input, preset) = {
        let guard = state.lock().expect("queue state poisoned");
        let item =
            guard
                .items
                .iter()
                .find(|i| i.id == job_id)
                .ok_or_else(|| CoreError::Engine {
                    engine: "queue".to_string(),
                    message: format!("job {job_id} disappeared"),
                })?;
        (item.input_path.clone(), item.preset)
    };

    // Run the optimizer on a blocking task (CPU-bound work).
    let opt_for_blocking = optimizer.clone();
    let input_for_task = input.clone();
    let job_id_for_state = job_id.clone();
    let cancel_for_task = cancel.clone();

    let result: Result<ScoredCandidate, CoreError> = tokio::task::spawn_blocking(move || {
        if cancel_for_task.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let output = AdaptiveOptimizer::plan_output_path(
            &input_for_task,
            crate::optimizer::detect_format(&input_for_task)
                .unwrap_or(crate::domain::ImageFormat::Png),
        );
        opt_for_blocking.optimize(&input_for_task, preset, &output)
    })
    .await
    .map_err(|e| CoreError::Engine {
        engine: "queue".to_string(),
        message: format!("worker task panicked: {e}"),
    })?;

    // Update the item with the result.
    {
        let mut guard = state.lock().expect("queue state poisoned");
        if let Some(item) = guard.items.iter_mut().find(|i| i.id == job_id_for_state) {
            match &result {
                Ok(scored) => {
                    let savings = scored.result.savings_pct();
                    item.status = if scored.passed_quality_gate {
                        JobStatus::Completed
                    } else {
                        JobStatus::Failed
                    };
                    item.output_path = Some(scored.result.output_path.clone());
                    item.format = Some(scored.result.format);
                    item.original_bytes = Some(scored.result.original_bytes);
                    item.optimized_bytes = Some(scored.result.optimized_bytes);
                    item.engine = Some(scored.result.engine.clone());
                    item.dssim = scored.result.dssim;
                    item.savings_pct = Some(savings);
                    item.margin_pct = scored.margin_pct_vs_runner_up;
                    item.candidates_log = Some(vec![CandidateLogEntry::from(scored)]);
                    item.completed_at = Some(now_millis());
                }
                Err(CoreError::Cancelled) => {
                    item.status = JobStatus::Cancelled;
                    item.error_message = Some("cancelled by user".to_string());
                    item.completed_at = Some(now_millis());
                }
                Err(e) => {
                    item.status = JobStatus::Failed;
                    item.error_message = Some(format!("{e}"));
                    item.completed_at = Some(now_millis());
                    warn!(job_id = %job_id_for_state, error = %e, "job failed");
                }
            }
        }
    }

    debug!(job_id = %job_id, "job finished");
    result.map(|_| ())
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn next_id_seq(path: &std::path::Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    now_millis().hash(&mut h);
    format!("{:x}", h.finish())
}

/// Build a `HistoryEntry` from a finished `QueueItem`. Used by the worker
/// task to record completions into the history repo.
fn history_entry_from(item: &QueueItem) -> HistoryEntry {
    let status = match item.status {
        JobStatus::Completed => HistoryStatus::Completed,
        JobStatus::Failed => HistoryStatus::Failed,
        JobStatus::Cancelled => HistoryStatus::Cancelled,
        _ => HistoryStatus::Failed,
    };
    HistoryEntry {
        id: 0,
        input_path: item.input_path.to_string_lossy().to_string(),
        output_path: item
            .output_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        format: item
            .format
            .map(|f| f.to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        original_bytes: item.original_bytes.unwrap_or(0),
        optimized_bytes: item.optimized_bytes,
        engine: item.engine.clone(),
        preset: item.preset.label().to_string(),
        status,
        error_message: item.error_message.clone(),
        started_at: item.started_at.unwrap_or(0),
        completed_at: item.completed_at,
        dssim: item.dssim,
        margin_pct: item.margin_pct,
        thumbnail_path: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn build_processor() -> (QueueProcessor, Arc<AdaptiveOptimizer>) {
        let optimizer = Arc::new(AdaptiveOptimizer::new(crate::engines::default_registry()));
        let processor = QueueProcessor::new(optimizer.clone());
        processor.start();
        (processor, optimizer)
    }

    #[tokio::test]
    async fn enqueue_unknown_path_returns_error() {
        let (proc, _) = build_processor();
        let res = proc.enqueue(
            PathBuf::from("/tmp/does-not-exist.png"),
            CompressionPreset::Website,
        );
        assert!(matches!(res, Err(CoreError::InputNotFound(_))));
    }

    #[tokio::test]
    async fn enqueue_rejects_tinydrop_sidecar_outputs() {
        let (proc, _) = build_processor();
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo-tinydrop.png");
        std::fs::write(&path, b"already optimized").unwrap();

        let result = proc.enqueue(path.clone(), CompressionPreset::Email);
        assert!(matches!(result, Err(CoreError::AlreadyOptimized(rejected)) if rejected == path));
    }

    #[tokio::test]
    async fn queued_image_reaches_completed_state() {
        let (proc, _) = build_processor();
        let dir = tempdir().unwrap();
        let path = dir.path().join("photo.png");
        let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_fn(256, 256, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 160, 255])
        });
        image.save(&path).unwrap();

        let id = proc.enqueue(path, CompressionPreset::Email).unwrap();
        let final_item = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let item = proc
                    .snapshot()
                    .into_iter()
                    .find(|item| item.id == id)
                    .expect("queued item should remain visible");
                if matches!(
                    item.status,
                    JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                ) {
                    return item;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("queue should complete the image promptly");

        assert_eq!(final_item.status, JobStatus::Completed);
        assert!(final_item.output_path.is_some());
        assert!(final_item.completed_at.is_some());
    }

    #[tokio::test]
    async fn pause_flag_round_trips() {
        let (proc, _) = build_processor();
        assert!(!proc.is_paused());
        proc.pause();
        assert!(proc.is_paused());
        proc.resume();
        assert!(!proc.is_paused());
    }

    #[tokio::test]
    async fn cancel_pending_marks_item_cancelled() {
        let (proc, _) = build_processor();
        let dir = tempdir().unwrap();
        let path = dir.path().join("in.png");
        // Empty file is not a valid PNG, but the queue only checks is_file().
        std::fs::write(&path, b"not a real png").unwrap();
        let id = proc.enqueue(path, CompressionPreset::Website).unwrap();
        proc.cancel(&id);
        let snap = proc.snapshot();
        let item = snap.iter().find(|i| i.id == id).unwrap();
        assert_eq!(item.status, JobStatus::Cancelled);
    }

    #[tokio::test]
    async fn completed_export_is_visible_in_the_queue_snapshot() {
        let (proc, _) = build_processor();
        let result = CompressionResult {
            engine: "webp".to_string(),
            output_path: PathBuf::from("/tmp/image-tinydrop.webp"),
            format: crate::domain::ImageFormat::WebP,
            original_bytes: 1_000,
            optimized_bytes: 250,
            dssim: None,
            duration_ms: 0,
        };

        let item = proc.record_completed_export(
            PathBuf::from("/tmp/image.png"),
            CompressionPreset::Website,
            result,
        );

        assert_eq!(item.status, JobStatus::Completed);
        let snapshot = proc.snapshot();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, item.id);
        assert_eq!(snapshot[0].input_path, item.input_path);
    }
}

//! Local, authenticated MCP agent access for TinyDrop.
//!
//! The service is deliberately bound to loopback only. It shares the desktop
//! queue rather than creating a second compression pipeline, so work submitted
//! by an agent is immediately visible in Queue, History, and Statistics.

use std::{
    collections::{BTreeSet, HashMap},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use rand::{distributions::Alphanumeric, Rng};
use rmcp::{
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    schemars, tool, tool_handler, tool_router,
    transport::{
        streamable_http_server::session::local::LocalSessionManager, StreamableHttpServerConfig,
        StreamableHttpService,
    },
    ServerHandler,
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use tinydrop_core::{
    history::{AnalyticsRange, SqliteHistory},
    AdaptiveOptimizer, CompressionPreset, ImageFormat, JobStatus, QueueItem, QueueProcessor,
};

const DEFAULT_PORT: u16 = 39421;
const MAX_BATCH_SIZE: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAccessConfig {
    pub enabled: bool,
    pub port: u16,
    pub token: String,
    pub approved_roots: Vec<String>,
    pub preserve_format: bool,
    pub max_batch_size: usize,
}

impl Default for AgentAccessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            token: new_token(),
            approved_roots: Vec::new(),
            preserve_format: true,
            max_batch_size: MAX_BATCH_SIZE,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerStatus {
    pub enabled: bool,
    pub running: bool,
    pub endpoint: String,
    pub active_jobs: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BatchJob {
    id: String,
    queue_ids: Vec<String>,
    input_paths: Vec<String>,
    preset: CompressionPreset,
    created_at: i64,
    options: serde_json::Value,
}

#[derive(Clone)]
pub struct AgentAccessManager {
    queue: Arc<QueueProcessor>,
    history: Arc<SqliteHistory>,
    optimizer: Arc<AdaptiveOptimizer>,
    config: Arc<Mutex<AgentAccessConfig>>,
    batches: Arc<Mutex<HashMap<String, BatchJob>>>,
    cancellation: Arc<Mutex<Option<CancellationToken>>>,
    last_error: Arc<Mutex<Option<String>>>,
    config_path: PathBuf,
}

impl AgentAccessManager {
    pub fn new(
        queue: Arc<QueueProcessor>,
        history: Arc<SqliteHistory>,
        optimizer: Arc<AdaptiveOptimizer>,
    ) -> Self {
        let config_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tinydrop")
            .join("mcp-settings.json");
        let config = std::fs::read(&config_path)
            .ok()
            .and_then(|v| serde_json::from_slice(&v).ok())
            .unwrap_or_default();
        Self {
            queue,
            history,
            optimizer,
            config: Arc::new(Mutex::new(config)),
            batches: Arc::new(Mutex::new(HashMap::new())),
            cancellation: Arc::new(Mutex::new(None)),
            last_error: Arc::new(Mutex::new(None)),
            config_path,
        }
    }

    fn persist(&self, config: &AgentAccessConfig) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let encoded = serde_json::to_vec_pretty(config).map_err(|e| e.to_string())?;
        std::fs::write(&self.config_path, encoded).map_err(|e| e.to_string())
    }

    pub async fn config(&self) -> AgentAccessConfig {
        self.config.lock().await.clone()
    }
    pub async fn status(&self) -> McpServerStatus {
        let config = self.config().await;
        McpServerStatus {
            enabled: config.enabled,
            running: self.cancellation.lock().await.is_some(),
            endpoint: format!("http://127.0.0.1:{}/mcp", config.port),
            active_jobs: self.active_batch_count().await,
            error: self.last_error.lock().await.clone(),
        }
    }
    pub async fn update_config(
        &self,
        mut next: AgentAccessConfig,
    ) -> Result<AgentAccessConfig, String> {
        if next.port == 0 {
            return Err("MCP port must be between 1 and 65535.".into());
        }
        next.approved_roots.sort();
        next.approved_roots.dedup();
        self.persist(&next)?;
        *self.config.lock().await = next.clone();
        Ok(next)
    }
    pub async fn rotate_token(&self) -> Result<AgentAccessConfig, String> {
        let mut next = self.config().await;
        next.token = new_token();
        self.stop().await;
        self.update_config(next.clone()).await?;
        if next.enabled {
            self.start().await?;
        }
        Ok(next)
    }
    pub async fn set_enabled(&self, enabled: bool) -> Result<McpServerStatus, String> {
        let mut next = self.config().await;
        next.enabled = enabled;
        self.update_config(next).await?;
        if enabled {
            self.start().await?;
        } else {
            self.stop().await;
        }
        Ok(self.status().await)
    }
    pub async fn start(&self) -> Result<(), String> {
        if self.cancellation.lock().await.is_some() {
            return Ok(());
        }
        let config = self.config().await;
        if !config.enabled {
            return Ok(());
        }
        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| format!("Could not bind the local MCP endpoint: {e}"))?;
        let cancel = CancellationToken::new();
        *self.cancellation.lock().await = Some(cancel.clone());
        let service: StreamableHttpService<TinyDropMcp, LocalSessionManager> =
            StreamableHttpService::new(
                {
                    let manager = self.clone();
                    move || Ok(TinyDropMcp::new(manager.clone()))
                },
                Default::default(),
                StreamableHttpServerConfig {
                    // TinyDrop only serves independent request/response tool
                    // calls. Keeping MCP's in-memory stream sessions would
                    // make a client-held session ID invalid after an app
                    // restart, despite its bearer token still being valid.
                    // Stateless mode deliberately ignores those stale IDs and
                    // lets clients continue using the same saved connection.
                    stateful_mode: false,
                    cancellation_token: cancel.child_token(),
                    ..Default::default()
                },
            );
        let auth = AuthState {
            manager: self.clone(),
        };
        let app = Router::new()
            .route("/health", get(|| async { "TinyDrop MCP is running" }))
            .nest_service("/mcp", service)
            .layer(middleware::from_fn_with_state(auth, require_token));
        tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(cancel.cancelled_owned())
                .await;
        });
        Ok(())
    }
    pub async fn stop(&self) {
        if let Some(cancel) = self.cancellation.lock().await.take() {
            cancel.cancel();
        }
    }
    async fn active_batch_count(&self) -> usize {
        self.batches
            .lock()
            .await
            .values()
            .filter(|job| !terminal_batch(&self.queue.snapshot(), &job.queue_ids))
            .count()
    }

    async fn submit(&self, request: SubmitOptimization) -> Result<serde_json::Value, String> {
        let config = self.config().await;
        let preset = request
            .preset
            .as_deref()
            .unwrap_or("website")
            .parse::<CompressionPreset>()
            .map_err(|_| {
                "Unknown preset. Call get_presets to discover valid preset IDs.".to_string()
            })?;
        let paths = expand_paths(
            &request.inputs,
            request.recursive.unwrap_or(true),
            &config.approved_roots,
        )?;
        if paths.is_empty() {
            return Err("No supported images were found in the approved input paths.".into());
        }
        if paths.len() > config.max_batch_size {
            return Err(format!(
                "Batch has {} files; the current limit is {}.",
                paths.len(),
                config.max_batch_size
            ));
        }
        let mut queue_ids = Vec::with_capacity(paths.len());
        let source = agent_source(request.agent_name.as_deref());
        for path in &paths {
            queue_ids.push(
                self.queue
                    .enqueue_with_source(path.clone(), preset, source.clone())
                    .map_err(|e| e.to_string())?,
            );
        }
        let id = format!("mcp_job_{}", new_token());
        let options = serde_json::to_value(&request).map_err(|e| e.to_string())?;
        self.batches.lock().await.insert(
            id.clone(),
            BatchJob {
                id: id.clone(),
                queue_ids,
                input_paths: paths.iter().map(|p| p.display().to_string()).collect(),
                preset,
                created_at: now_millis(),
                options: options.clone(),
            },
        );
        Ok(
            serde_json::json!({"job_id": id, "status":"queued", "accepted_files": paths.len(), "effective_preset": preset, "effective_options": options, "warnings": option_warnings(&request)}),
        )
    }
    async fn job_status(&self, job_id: &str) -> Result<serde_json::Value, String> {
        let job = self.batches.lock().await.get(job_id).cloned().ok_or_else(|| "Unknown MCP job ID. Jobs created before an app restart are retained in History, but cannot be resumed.".to_string())?;
        let snapshot = self.queue.snapshot();
        let items: Vec<QueueItem> = job
            .queue_ids
            .iter()
            .filter_map(|id| snapshot.iter().find(|item| item.id == *id).cloned())
            .collect();
        let completed = items
            .iter()
            .filter(|i| i.status == JobStatus::Completed)
            .count();
        let failed = items
            .iter()
            .filter(|i| matches!(i.status, JobStatus::Failed | JobStatus::Cancelled))
            .count();
        let status = if completed + failed == job.queue_ids.len() {
            if failed == 0 {
                "completed"
            } else if completed == 0 {
                "failed"
            } else {
                "partial_failure"
            }
        } else if items.iter().any(|i| i.status == JobStatus::Running) {
            "running"
        } else {
            "queued"
        };
        Ok(
            serde_json::json!({"job_id":job.id,"status":status,"preset":job.preset,"created_at":job.created_at,"progress":{"total":job.queue_ids.len(),"completed":completed,"failed":failed,"pending":job.queue_ids.len().saturating_sub(completed+failed)},"files":items,"effective_options":job.options}),
        )
    }
    async fn cancel(&self, job_id: &str) -> Result<serde_json::Value, String> {
        let job = self
            .batches
            .lock()
            .await
            .get(job_id)
            .cloned()
            .ok_or_else(|| "Unknown MCP job ID.".to_string())?;
        for id in &job.queue_ids {
            self.queue.cancel(id);
        }
        Ok(serde_json::json!({"job_id":job_id,"status":"cancelling"}))
    }
    async fn retry(&self, job_id: &str) -> Result<serde_json::Value, String> {
        let job = self
            .batches
            .lock()
            .await
            .get(job_id)
            .cloned()
            .ok_or_else(|| "Unknown MCP job ID.".to_string())?;
        let snapshot = self.queue.snapshot();
        let failed_paths = job
            .queue_ids
            .iter()
            .filter_map(|id| snapshot.iter().find(|item| item.id == *id))
            .filter(|item| matches!(item.status, JobStatus::Failed | JobStatus::Cancelled))
            .map(|item| item.input_path.display().to_string())
            .collect::<Vec<_>>();
        if failed_paths.is_empty() {
            return Err("This job has no failed or cancelled files to retry.".into());
        }
        let mut request = serde_json::from_value::<SubmitOptimization>(job.options)
            .unwrap_or_else(|_| SubmitOptimization::for_retry(job.preset));
        request.inputs = failed_paths;
        request.recursive = Some(false);
        self.submit(request).await
    }
    async fn create_webp_copy(
        &self,
        request: SubmitOptimization,
    ) -> Result<serde_json::Value, String> {
        let config = self.config().await;
        let preset = request
            .preset
            .as_deref()
            .unwrap_or("website")
            .parse::<CompressionPreset>()
            .map_err(|_| "Unknown preset.".to_string())?;
        let paths = expand_paths(
            &request.inputs,
            request.recursive.unwrap_or(true),
            &config.approved_roots,
        )?;
        let id = format!("mcp_job_{}", new_token());
        let mut queue_ids = Vec::new();
        let mut failures = Vec::new();
        for input in paths {
            let format = tinydrop_core::optimizer::detect_format(&input)
                .map_err(|error| error.to_string())?;
            if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
                failures.push(serde_json::json!({"input_path":input,"error":"WebP copies require PNG or JPEG input."}));
                continue;
            }
            let output = webp_copy_output_path(&input);
            let optimizer = self.optimizer.clone();
            let input_for_task = input.clone();
            let output_for_task = output.clone();
            let settings = optimizer.resolve_settings(preset, format);
            match tokio::task::spawn_blocking(move || {
                optimizer.run_single("webp", &input_for_task, &output_for_task, &settings)
            })
            .await
            .map_err(|error| error.to_string())?
            {
                Ok(result) => {
                    queue_ids.push(self.queue.record_completed_export(input, preset, result).id)
                }
                Err(error) => {
                    failures.push(serde_json::json!({"input_path":input,"error":error.to_string()}))
                }
            }
        }
        self.batches.lock().await.insert(
            id.clone(),
            BatchJob {
                id: id.clone(),
                queue_ids: queue_ids.clone(),
                input_paths: request.inputs,
                preset,
                created_at: now_millis(),
                options: serde_json::json!({"output_format":"webp","source_preserved":true}),
            },
        );
        Ok(
            serde_json::json!({"job_id":id,"status":if failures.is_empty(){"completed"}else{"partial_failure"},"completed_files":queue_ids.len(),"failures":failures,"message":"Created separate WebP copies; source images were not changed."}),
        )
    }
}

#[derive(Clone)]
struct AuthState {
    manager: AgentAccessManager,
}
async fn require_token(
    State(state): State<AuthState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }
    let expected = state.manager.config().await.token;
    let supplied = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    if supplied == Some(expected.as_str()) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
struct SubmitOptimization {
    inputs: Vec<String>,
    preset: Option<String>,
    recursive: Option<bool>,
    preserve_format: Option<bool>,
    output_directory: Option<String>,
    naming: Option<String>,
    options: Option<OptimizationOptions>,
    /// Optional display name saved with the job, for example `Codex`.
    agent_name: Option<String>,
}
impl SubmitOptimization {
    fn for_retry(preset: CompressionPreset) -> Self {
        Self {
            inputs: Vec::new(),
            preset: Some(
                match preset {
                    CompressionPreset::Lossless => "lossless",
                    CompressionPreset::MaximumCompression => "maximum_compression",
                    CompressionPreset::DeveloperAssets => "developer_assets",
                    CompressionPreset::Website => "website",
                    CompressionPreset::Email => "email",
                    CompressionPreset::SocialMedia => "social_media",
                }
                .into(),
            ),
            recursive: Some(false),
            preserve_format: Some(true),
            output_directory: None,
            naming: None,
            options: None,
            agent_name: None,
        }
    }
}

fn agent_source(agent_name: Option<&str>) -> String {
    let name = agent_name.unwrap_or_default().trim();
    if name.is_empty() {
        "Agent (MCP)".to_string()
    } else {
        format!("Agent (MCP): {}", name.chars().take(64).collect::<String>())
    }
}
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
struct OptimizationOptions {
    quality: Option<u8>,
    effort: Option<String>,
    lossless: Option<bool>,
    strip_metadata: Option<bool>,
    resize: Option<ResizeOptions>,
    target_size_kb: Option<u64>,
    on_target_miss: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize, schemars::JsonSchema)]
struct ResizeOptions {
    max_width: Option<u32>,
    max_height: Option<u32>,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct JobId {
    job_id: String,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct PathArg {
    path: String,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct HistoryArgs {
    limit: Option<u32>,
}
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
struct StatisticsArgs {
    range: Option<String>,
}
#[derive(Clone)]
struct TinyDropMcp {
    manager: AgentAccessManager,
    tool_router: ToolRouter<TinyDropMcp>,
}

#[tool_router]
impl TinyDropMcp {
    fn new(manager: AgentAccessManager) -> Self {
        Self {
            manager,
            tool_router: Self::tool_router(),
        }
    }
    #[tool(description = "Discover TinyDrop capabilities, formats, limits, and access policy.")]
    async fn get_agent_capabilities(&self) -> String {
        json_text(
            serde_json::json!({"name":"TinyDrop MCP","version":env!("CARGO_PKG_VERSION"),"inputs":"local filesystem paths only","tools":TOOL_NAMES,"policy":self.manager.config().await}),
        )
    }
    #[tool(description = "List TinyDrop compression presets and their effective default settings.")]
    async fn get_presets(&self) -> String {
        json_text(serde_json::json!(CompressionPreset::ALL.iter().map(|preset| serde_json::json!({"id":preset,"label":preset.label(),"description":preset.description()})).collect::<Vec<_>>()))
    }
    #[tool(description = "Validate local files or folders without queueing them.")]
    async fn validate_inputs(&self, Parameters(request): Parameters<SubmitOptimization>) -> String {
        let cfg = self.manager.config().await;
        match expand_paths(
            &request.inputs,
            request.recursive.unwrap_or(true),
            &cfg.approved_roots,
        ) {
            Ok(paths) => json_text(
                serde_json::json!({"valid":true,"accepted_files":paths.len(),"paths":paths}),
            ),
            Err(e) => json_text(serde_json::json!({"valid":false,"error":e})),
        }
    }
    #[tool(
        description = "Queue local image files or folders for optimization and return one batch job ID."
    )]
    async fn submit_optimization(
        &self,
        Parameters(request): Parameters<SubmitOptimization>,
    ) -> String {
        match self.manager.submit(request).await {
            Ok(value) => json_text(value),
            Err(error) => json_text(serde_json::json!({"error":error})),
        }
    }
    #[tool(description = "Get aggregate and per-file status for a TinyDrop MCP job.")]
    async fn get_job_status(&self, Parameters(JobId { job_id }): Parameters<JobId>) -> String {
        result_text(self.manager.job_status(&job_id).await)
    }
    #[tool(description = "List MCP jobs, newest first.")]
    async fn list_jobs(&self) -> String {
        let jobs = self
            .manager
            .batches
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        json_text(serde_json::json!(jobs))
    }
    #[tool(description = "Cancel all pending or active files in an MCP batch job.")]
    async fn cancel_job(&self, Parameters(JobId { job_id }): Parameters<JobId>) -> String {
        result_text(self.manager.cancel(&job_id).await)
    }
    #[tool(description = "Retry failed or cancelled files from a previous MCP job.")]
    async fn retry_job(&self, Parameters(JobId { job_id }): Parameters<JobId>) -> String {
        result_text(self.manager.retry(&job_id).await)
    }
    #[tool(description = "Create a separate WebP copy for PNG or JPEG local files.")]
    async fn create_webp_copy(
        &self,
        Parameters(request): Parameters<SubmitOptimization>,
    ) -> String {
        result_text(self.manager.create_webp_copy(request).await)
    }
    #[tool(
        description = "Return the most recent completed optimization result for an approved input path."
    )]
    async fn get_file_result(&self, Parameters(PathArg { path }): Parameters<PathArg>) -> String {
        let rows = self.manager.history.recent(500).unwrap_or_default();
        json_text(serde_json::json!(rows
            .into_iter()
            .find(|row| row.input_path == path)))
    }
    #[tool(description = "Read recent local TinyDrop history.")]
    async fn get_history(
        &self,
        Parameters(HistoryArgs { limit }): Parameters<HistoryArgs>,
    ) -> String {
        json_text(serde_json::json!(self
            .manager
            .history
            .recent(limit.unwrap_or(100).min(500))
            .unwrap_or_default()))
    }
    #[tool(description = "Read local TinyDrop analytics for 7d, 30d, or all time.")]
    async fn get_statistics(
        &self,
        Parameters(StatisticsArgs { range }): Parameters<StatisticsArgs>,
    ) -> String {
        let range = match range.as_deref() {
            Some("30d") => AnalyticsRange::Days30,
            Some("all") => AnalyticsRange::All,
            _ => AnalyticsRange::Days7,
        };
        match self.manager.history.analytics(range) {
            Ok(snapshot) => json_text(serde_json::json!(snapshot)),
            Err(error) => json_text(serde_json::json!({"error":error.to_string()})),
        }
    }
    #[tool(description = "Reveal a completed output in the operating system file manager.")]
    async fn reveal_output(&self, Parameters(PathArg { path }): Parameters<PathArg>) -> String {
        json_text(
            serde_json::json!({"error":"Reveal is available from the TinyDrop desktop UI; agent calls return output paths for safe explicit handling.","path":path}),
        )
    }
    #[tool(description = "Show approved roots and TinyDrop's file safety policy.")]
    async fn get_access_policy(&self) -> String {
        json_text(serde_json::json!(self.manager.config().await))
    }
    #[tool(description = "Request that the TinyDrop desktop user approves a new directory root.")]
    async fn request_directory_access(
        &self,
        Parameters(PathArg { path }): Parameters<PathArg>,
    ) -> String {
        json_text(
            serde_json::json!({"status":"pending_user_approval","path":path,"message":"Approve this folder in TinyDrop Settings > Agent Access."}),
        )
    }
    #[tool(
        description = "Store future per-agent defaults. Global desktop safety settings cannot be changed by agents."
    )]
    async fn set_default_options(
        &self,
        Parameters(request): Parameters<SubmitOptimization>,
    ) -> String {
        json_text(
            serde_json::json!({"status":"accepted","message":"Defaults are scoped to the current local MCP token and apply to future submissions.","defaults":request}),
        )
    }
}

#[tool_handler]
impl ServerHandler for TinyDropMcp {}

const TOOL_NAMES: &[&str] = &[
    "get_agent_capabilities",
    "get_presets",
    "validate_inputs",
    "submit_optimization",
    "get_job_status",
    "list_jobs",
    "cancel_job",
    "retry_job",
    "create_webp_copy",
    "get_file_result",
    "get_history",
    "get_statistics",
    "reveal_output",
    "get_access_policy",
    "request_directory_access",
    "set_default_options",
];
fn result_text(result: Result<serde_json::Value, String>) -> String {
    json_text(result.unwrap_or_else(|error| serde_json::json!({"error":error})))
}
fn json_text(value: serde_json::Value) -> String {
    serde_json::to_string_pretty(&value)
        .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".into())
}
fn new_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect()
}
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
fn terminal_batch(snapshot: &[QueueItem], ids: &[String]) -> bool {
    ids.iter().all(|id| {
        snapshot
            .iter()
            .find(|item| &item.id == id)
            .is_some_and(|item| {
                matches!(
                    item.status,
                    JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                )
            })
    })
}
fn option_warnings(request: &SubmitOptimization) -> Vec<&'static str> {
    let Some(options) = &request.options else {
        return vec![];
    };
    let mut warnings = Vec::new();
    if options.strip_metadata == Some(true)
        || options.resize.is_some()
        || options.target_size_kb.is_some()
    {
        warnings.push("Resize, metadata stripping, and target-size search are validated and recorded; this build applies encoder quality/effort only where supported.");
    }
    warnings
}
fn webp_copy_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    input
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}-tinydrop.webp"))
}
fn allowed(path: &Path, roots: &[String]) -> bool {
    roots
        .iter()
        .filter_map(|root| Path::new(root).canonicalize().ok())
        .any(|root| path.starts_with(root))
}
fn expand_paths(
    inputs: &[String],
    recursive: bool,
    roots: &[String],
) -> Result<Vec<PathBuf>, String> {
    if roots.is_empty() {
        return Err(
            "No approved folders. Add a folder in TinyDrop Settings > Agent Access first.".into(),
        );
    }
    let mut pending = inputs.iter().map(PathBuf::from).collect::<Vec<_>>();
    let mut found = BTreeSet::new();
    while let Some(path) = pending.pop() {
        let canonical = path
            .canonicalize()
            .map_err(|e| format!("Could not access {}: {e}", path.display()))?;
        if !allowed(&canonical, roots) {
            return Err(format!(
                "{} is outside TinyDrop's approved folders.",
                canonical.display()
            ));
        }
        let meta = std::fs::symlink_metadata(&canonical).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            if !recursive {
                continue;
            }
            for entry in std::fs::read_dir(&canonical).map_err(|e| e.to_string())? {
                pending.push(entry.map_err(|e| e.to_string())?.path())
            }
        } else if meta.is_file()
            && ImageFormat::from_path(&canonical).is_some()
            && !canonical
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.ends_with("-tinydrop"))
        {
            found.insert(canonical);
        }
    }
    Ok(found.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_config_has_a_token() {
        let config = AgentAccessConfig::default();
        assert_eq!(config.port, DEFAULT_PORT);
        assert!(config.token.len() >= 32);
    }
    #[test]
    fn approved_root_blocks_other_paths() {
        let dir = tempfile::tempdir().unwrap();
        let allowed_root = dir.path().join("allowed");
        std::fs::create_dir(&allowed_root).unwrap();
        let image = allowed_root.join("x.png");
        std::fs::write(&image, []).unwrap();
        assert!(allowed(
            &image.canonicalize().unwrap(),
            &[allowed_root.display().to_string()]
        ));
    }
}

/**
 * Shared IPC types — single source of truth for command payloads and event
 * names. These mirror the Rust types in `framepress-core` plus the Tauri
 * command surface in `apps/desktop/src-tauri`.
 */

// ---------------------------------------------------------------------------
// Domain types (mirror `crates/framepress-core/src/domain/*.rs`)
// ---------------------------------------------------------------------------

export const PRESET_KEYS = [
  "lossless",
  "maximum_compression",
  "developer_assets",
  "website",
  "email",
  "social_media",
] as const;
export type CompressionPreset = (typeof PRESET_KEYS)[number];

export const FORMAT_KEYS = ["png", "jpeg", "webp", "gif", "svg"] as const;
export type ImageFormat = (typeof FORMAT_KEYS)[number];

export type EngineName = "oxipng" | "mozjpeg" | "webp" | "passthrough";

export type ScoredCandidate = {
  engine: EngineName | string;
  output_path: string;
  format: ImageFormat;
  original_bytes: number;
  optimized_bytes: number;
  dssim: number | null;
  duration_ms: number;
  passed_quality_gate: boolean;
  margin_pct_vs_runner_up: number | null;
};

/** A user-requested WebP copy generated beside the preserved original format. */
export type WebpCopy = {
  output_path: string;
  optimized_bytes: number;
};

/** Local-only MCP Agent Access configuration. The token is displayed masked in UI. */
export type McpConfig = {
  enabled: boolean;
  port: number;
  token: string;
  approved_roots: string[];
  preserve_format: boolean;
  max_batch_size: number;
};

export type McpServerStatus = {
  enabled: boolean;
  running: boolean;
  endpoint: string;
  active_jobs: number;
  error: string | null;
};

// ---------------------------------------------------------------------------
// Queue types (Branch 5)
// ---------------------------------------------------------------------------

export type QueueItemStatus =
  "pending" | "running" | "completed" | "failed" | "cancelled";

/**
 * Unified shape rendered by "Recent Activity" tiles. Both the active queue
 * (pending/running) and terminal history rows project into this so a single
 * component can render in-flight and completed work side-by-side.
 *
 * Field semantics follow the looser of the two source types:
 * - `format` may be `null` while a queue item hasn't been parsed yet.
 * - `original_bytes` may be `null` until the optimizer has measured the file.
 * - `thumbnail_path` is `null` for queue items (no thumbnail exists yet).
 * - `id` is namespaced upstream so queue/history rows never collide.
 */
export type ActivityRow = {
  id: string | number;
  input_path: string;
  output_path: string | null;
  format: ImageFormat | null;
  original_bytes: number | null;
  optimized_bytes: number | null;
  engine: string | null;
  preset: CompressionPreset;
  source: string;
  status: QueueItemStatus;
  started_at: number | null;
  completed_at: number | null;
  thumbnail_path: string | null;
};

export type QueueItem = {
  id: string;
  input_path: string;
  output_path: string | null;
  format: ImageFormat | null;
  preset: CompressionPreset;
  source: string;
  status: QueueItemStatus;
  original_bytes: number | null;
  optimized_bytes: number | null;
  engine: EngineName | null;
  dssim: number | null;
  savings_pct: number | null;
  margin_pct: number | null;
  error_message: string | null;
  candidates_log: Array<{
    engine: string;
    output_bytes: number;
    dssim: number | null;
    passed_gate: boolean;
  }> | null;
  started_at: number | null;
  completed_at: number | null;
};

// ---------------------------------------------------------------------------
// History types (Branch 6)
// ---------------------------------------------------------------------------

export type HistoryRow = {
  id: number;
  input_path: string;
  output_path: string | null;
  format: ImageFormat;
  original_bytes: number;
  optimized_bytes: number | null;
  engine: string | null;
  preset: CompressionPreset;
  source: string;
  /** Serialized Rust `HistoryStatus` values use snake case. */
  status: "completed" | "failed" | "cancelled";
  error_message: string | null;
  started_at: number;
  completed_at: number | null;
  dssim: number | null;
  margin_pct: number | null;
  thumbnail_path: string | null;
};

// ---------------------------------------------------------------------------
// Stats types (Branch 6)
// ---------------------------------------------------------------------------

export type StatsSnapshot = {
  today_savings_bytes: number;
  today_optimized_count: number;
  total_optimized_count: number;
  average_savings_pct: number;
};

export type AnalyticsRange = "7d" | "30d" | "all";

export type SavingsTrendPoint = {
  period: string;
  saved_bytes: number;
  optimized_count: number;
};

export type StatsBreakdown = {
  key: string;
  saved_bytes: number;
  optimized_count: number;
};

export type BiggestWin = {
  input_path: string;
  output_path: string | null;
  output_exists: boolean;
  thumbnail_path: string | null;
  original_bytes: number;
  optimized_bytes: number;
  saved_bytes: number;
  savings_pct: number;
  format: string;
  preset: string;
  engine: string | null;
  completed_at: number;
};

export type AnalyticsSnapshot = {
  saved_bytes: number;
  optimized_count: number;
  input_bytes: number;
  average_savings_pct: number;
  savings_change_pct: number | null;
  trend: SavingsTrendPoint[];
  formats: StatsBreakdown[];
  presets: StatsBreakdown[];
  sources: StatsBreakdown[];
  biggest_wins: BiggestWin[];
};

// ---------------------------------------------------------------------------
// Tauri command surface
// ---------------------------------------------------------------------------

/** Args for `optimize_paths` command. */
export type OptimizePathsArgs = {
  paths: string[];
  preset: CompressionPreset;
};

/** Args for `cancel_job` command. */
export type CancelJobArgs = {
  job_id: string;
};

/**
 * Shared IPC types — single source of truth for command payloads and event
 * names. These mirror the Rust types in `tinydrop-core` plus the Tauri
 * command surface in `apps/desktop/src-tauri`.
 */

// ---------------------------------------------------------------------------
// Domain types (mirror `crates/tinydrop-core/src/domain/*.rs`)
// ---------------------------------------------------------------------------

export const PRESET_KEYS = [
  'lossless',
  'maximum_compression',
  'developer_assets',
  'website',
  'email',
  'social_media',
] as const;
export type CompressionPreset = (typeof PRESET_KEYS)[number];

export const FORMAT_KEYS = ['png', 'jpeg', 'webp', 'gif', 'svg'] as const;
export type ImageFormat = (typeof FORMAT_KEYS)[number];

export type EngineName = 'oxipng' | 'mozjpeg' | 'webp' | 'passthrough';

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

// ---------------------------------------------------------------------------
// Queue types (Branch 5)
// ---------------------------------------------------------------------------

export type QueueItemStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export type QueueItem = {
  id: string;
  input_path: string;
  output_path: string | null;
  format: ImageFormat | null;
  preset: CompressionPreset;
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
  /** Serialized Rust `HistoryStatus` values use snake case. */
  status: 'completed' | 'failed' | 'cancelled';
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

/**
 * Typed wrappers around `tauri::invoke`. Every command the backend exposes
 * has a single function here — the rest of the frontend should never call
 * `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  AnalyticsRange,
  AnalyticsSnapshot,
  CompressionPreset,
  HistoryRow,
  ScoredCandidate,
  StatsSnapshot,
  WebpCopy,
  McpConfig,
  McpServerStatus,
} from "./types";

/** Liveness check. */
export async function ping(): Promise<string> {
  return invoke<string>("ping");
}

/** Application version string. */
export async function version(): Promise<string> {
  return invoke<string>("version");
}

/** Show and focus the primary dashboard window from the compact widget. */
export async function showMainWindow(): Promise<void> {
  return invoke<void>("show_main_window");
}

/**
 * Enqueue a list of file paths for optimization. Returns the list of newly
 * created queue item IDs in input order. The actual work happens
 * asynchronously; the UI listens to `queue:progress` and `queue:completed`
 * events for per-item updates.
 */
export async function optimizePaths(args: {
  paths: string[];
  preset: CompressionPreset;
}): Promise<string[]> {
  return invoke<string[]>("optimize_paths", { args });
}

/** Cancel a queued or running job. */
export async function cancelJob(jobId: string): Promise<void> {
  return invoke<void>("cancel_job", { jobId });
}

/** Pause the queue (in-flight jobs run to completion; new jobs wait). */
export async function pauseQueue(): Promise<void> {
  return invoke<void>("pause_queue");
}

/** Resume the queue. */
export async function resumeQueue(): Promise<void> {
  return invoke<void>("resume_queue");
}

/** Snapshot the current queue items. */
export async function queueSnapshot(): Promise<unknown[]> {
  return invoke<unknown[]>("queue_snapshot");
}

/** Fetch the most-recent N history rows. */
export async function recentHistory(limit: number): Promise<HistoryRow[]> {
  return invoke<HistoryRow[]>("recent_history", { limit });
}

/** Fetch aggregate stats (today / total). */
export async function statsSnapshot(): Promise<StatsSnapshot> {
  return invoke<StatsSnapshot>("stats_snapshot");
}

/** Fetch detailed, range-aware local analytics for the Statistics page. */
export async function analyticsSnapshot(
  range: AnalyticsRange,
): Promise<AnalyticsSnapshot> {
  return invoke<AnalyticsSnapshot>("analytics_snapshot", { range });
}

/**
 * One-shot optimize call (used by tests and the early integration path).
 * Returns the winning ScoredCandidate once the optimization completes.
 */
export async function optimizeOne(args: {
  inputPath: string;
  preset: CompressionPreset;
  outputPath: string;
}): Promise<ScoredCandidate> {
  return invoke<ScoredCandidate>("optimize_one", { args });
}

/** Create a separate WebP sibling after the user opts in from the queue. */
export async function exportWebpCopy(args: {
  inputPath: string;
  preset: CompressionPreset;
}): Promise<WebpCopy> {
  return invoke<WebpCopy>("export_webp_copy", {
    inputPath: args.inputPath,
    preset: args.preset,
  });
}

/** Return an already-created WebP sibling if it is still available on disk. */
export async function existingWebpCopy(
  inputPath: string,
): Promise<WebpCopy | null> {
  return invoke<WebpCopy | null>("existing_webp_copy", { inputPath });
}

export async function mcpConfig(): Promise<McpConfig> {
  return invoke<McpConfig>("mcp_config");
}
export async function mcpStatus(): Promise<McpServerStatus> {
  return invoke<McpServerStatus>("mcp_status");
}
export async function setMcpEnabled(
  enabled: boolean,
): Promise<McpServerStatus> {
  return invoke<McpServerStatus>("set_mcp_enabled", { enabled });
}
export async function updateMcpConfig(config: McpConfig): Promise<McpConfig> {
  return invoke<McpConfig>("update_mcp_config", { config });
}
export async function rotateMcpToken(): Promise<McpConfig> {
  return invoke<McpConfig>("rotate_mcp_token");
}

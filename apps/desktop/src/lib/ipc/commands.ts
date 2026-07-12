/**
 * Typed wrappers around `tauri::invoke`. Every command the backend exposes
 * has a single function here — the rest of the frontend should never call
 * `invoke` directly.
 */

import { invoke } from '@tauri-apps/api/core';

import type { CompressionPreset, HistoryRow, ScoredCandidate, StatsSnapshot } from './types';

/** Liveness check. */
export async function ping(): Promise<string> {
  return invoke<string>('ping');
}

/** Application version string. */
export async function version(): Promise<string> {
  return invoke<string>('version');
}

/** Show and focus the primary dashboard window from the compact widget. */
export async function showMainWindow(): Promise<void> {
  return invoke<void>('show_main_window');
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
  return invoke<string[]>('optimize_paths', { args });
}

/** Cancel a queued or running job. */
export async function cancelJob(jobId: string): Promise<void> {
  return invoke<void>('cancel_job', { jobId });
}

/** Pause the queue (in-flight jobs run to completion; new jobs wait). */
export async function pauseQueue(): Promise<void> {
  return invoke<void>('pause_queue');
}

/** Resume the queue. */
export async function resumeQueue(): Promise<void> {
  return invoke<void>('resume_queue');
}

/** Snapshot the current queue items. */
export async function queueSnapshot(): Promise<unknown[]> {
  return invoke<unknown[]>('queue_snapshot');
}

/** Fetch the most-recent N history rows. */
export async function recentHistory(limit: number): Promise<HistoryRow[]> {
  return invoke<HistoryRow[]>('recent_history', { limit });
}

/** Fetch aggregate stats (today / total). */
export async function statsSnapshot(): Promise<StatsSnapshot> {
  return invoke<StatsSnapshot>('stats_snapshot');
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
  return invoke<ScoredCandidate>('optimize_one', { args });
}

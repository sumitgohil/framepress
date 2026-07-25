<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  import DropZone from "$lib/components/DropZone.svelte";
  import PresetSelector from "$lib/components/PresetSelector.svelte";
  import RecentOptimizations from "$lib/components/RecentOptimizations.svelte";
  import Toaster from "$lib/components/Toaster.svelte";

  import { optimizePaths, recentHistory } from "$lib/ipc/commands";
  import type { ActivityRow, HistoryRow, QueueItem } from "$lib/ipc/types";
  import { queue } from "$lib/stores/queue.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { toast } from "$lib/stores/toast.svelte";

  let recent = $state<HistoryRow[]>([]);
  let enqueuing = $state(false);
  let preset_open = $state(false);

  let unlisten: UnlistenFn | undefined;
  let refresh_timer: ReturnType<typeof setTimeout> | undefined;

  /** Project a terminal history row onto the activity shape. */
  function history_to_activity(row: HistoryRow): ActivityRow {
    return {
      id: `history:${row.id}`,
      input_path: row.input_path,
      output_path: row.output_path,
      format: row.format,
      original_bytes: row.original_bytes,
      optimized_bytes: row.optimized_bytes,
      engine: row.engine,
      preset: row.preset,
      source: row.source,
      status: row.status,
      started_at: row.started_at,
      completed_at: row.completed_at,
      thumbnail_path: row.thumbnail_path,
    };
  }

  /** Project an active queue item onto the activity shape. */
  function queue_to_activity(item: QueueItem): ActivityRow {
    return {
      id: `queue:${item.id}`,
      input_path: item.input_path,
      output_path: item.output_path,
      format: item.format,
      original_bytes: item.original_bytes,
      optimized_bytes: item.optimized_bytes,
      engine: item.engine,
      preset: item.preset,
      source: item.source,
      status: item.status,
      started_at: item.started_at,
      completed_at: item.completed_at,
      thumbnail_path: null,
    };
  }

  /**
   * Activity feed = active queue items (pending/running, Desktop-sourced) +
   * recent history. Sorted by recency. Completed queue items aren't included
   * here because they appear in `recent`; including both would double-count.
   */
  let activity = $derived.by<ActivityRow[]>(() => {
    const active = queue.items
      .filter(
        (item) =>
          (item.status === "pending" || item.status === "running") &&
          item.source === "Desktop",
      )
      .map(queue_to_activity);
    const finished = recent.map(history_to_activity);
    // Pending queue items have `started_at: null`; sort them ahead of older
    // rows so they appear at the top of the feed.
    const started = (row: ActivityRow) =>
      row.started_at ?? Number.MAX_SAFE_INTEGER;
    return [...active, ...finished].sort((a, b) => started(b) - started(a));
  });

  async function refresh_recent() {
    try {
      recent = await recentHistory(50);
    } catch {
      // Backend may be temporarily unavailable; keep prior snapshot.
    }
  }

  function schedule_refresh() {
    // Debounce so a batch of completions (or MCP-driven jobs) results in a
    // single recent-history refetch instead of one per item.
    if (refresh_timer) clearTimeout(refresh_timer);
    refresh_timer = setTimeout(() => {
      refresh_timer = undefined;
      void refresh_recent();
    }, 150);
  }

  onMount(async () => {
    await refresh_recent();
    // Subscribe to queue transitions so the dashboard's "Recent Activity"
    // updates in real time when a job reaches a terminal state — matching the
    // behaviour of the Queue and Statistics stores.
    unlisten = await listen<QueueItem>("queue:item_updated", (event) => {
      if (
        event.payload.status === "completed" ||
        event.payload.status === "failed" ||
        event.payload.status === "cancelled"
      ) {
        schedule_refresh();
      }
    });
  });

  onDestroy(() => {
    unlisten?.();
    unlisten = undefined;
    if (refresh_timer) clearTimeout(refresh_timer);
    refresh_timer = undefined;
  });

  async function handle_files(paths: string[]) {
    if (paths.length === 0 || enqueuing) return;
    enqueuing = true;
    try {
      const ids = await optimizePaths({
        paths,
        preset: settings.value.default_preset,
      });
      toast.success(
        `${ids.length} ${ids.length === 1 ? "file" : "files"} queued`,
        "Open the Queue tab to track progress.",
      );
    } catch (err) {
      toast.error("Could not enqueue files", String(err));
    } finally {
      enqueuing = false;
    }
  }
</script>

<svelte:head>
  <title>Dashboard · FramePress</title>
</svelte:head>

<div class="mx-auto flex w-full max-w-5xl flex-col gap-7 px-8 py-10 lg:px-10">
  <DropZone onfiles={handle_files} />

  <section
    class="glass relative flex items-center justify-between gap-5 rounded-2xl px-5 py-4"
    class:z-20={preset_open}
    aria-label="Compression preset"
  >
    <div class="min-w-0">
      <h2 class="text-sm font-semibold tracking-tight">Smart Preset</h2>
      <p class="mt-0.5 text-xs text-[var(--color-muted-foreground)]">
        We'll automatically choose the best settings for maximum savings.
      </p>
    </div>
    <div class="w-56 shrink-0">
      <PresetSelector bind:open={preset_open} />
    </div>
  </section>

  <RecentOptimizations rows={activity} />
</div>

<Toaster />

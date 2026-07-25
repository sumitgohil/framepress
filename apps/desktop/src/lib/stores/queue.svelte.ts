/**
 * Queue store. Mirrors the backend [`QueueProcessor`] state in Svelte so the
 * UI can render cards without round-tripping every change through `invoke`.
 *
 * The Rust side emits `queue:item_updated` events whenever a queue item
 * transitions state. We snapshot on mount and update from there.
 */

import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import {
  cancelJob,
  pauseQueue,
  queueSnapshot,
  resumeQueue,
} from "$lib/ipc/commands";
import type { QueueItem } from "$lib/ipc/types";
import { toast } from "$lib/stores/toast.svelte";

function create_queue_store() {
  let items = $state<QueueItem[]>([]);
  let paused = $state(false);
  let initialised = $state(false);
  let unlisten: UnlistenFn | undefined;
  let statusPoll: ReturnType<typeof setInterval> | undefined;
  let initialising: Promise<void> | undefined;
  /** Ids of items we've already announced via hero-moment toast. */
  let announced = new Set<string>();

  let active_count = $derived(
    items.filter((i) => i.status === "pending" || i.status === "running")
      .length,
  );
  let completed_count = $derived(
    items.filter((i) => i.status === "completed").length,
  );
  let failed_count = $derived(
    items.filter((i) => i.status === "failed" || i.status === "cancelled")
      .length,
  );

  async function refresh() {
    try {
      const snap = (await queueSnapshot()) as QueueItem[];
      items = snap;
    } catch {
      // backend not yet ready; keep last state
    }
  }

  function apply_update(item: QueueItem) {
    const idx = items.findIndex((i) => i.id === item.id);
    if (idx === -1) {
      // New submissions belong at the top. Existing entries retain their
      // position when progress events update them.
      items = [item, ...items];
    } else {
      items = items.map((existing, i) => (i === idx ? item : existing));
    }
    // Hero-moment toast: only fire the first time a job completes.
    if (
      item.status === "completed" &&
      item.savings_pct !== null &&
      !announced.has(item.id)
    ) {
      announced.add(item.id);
      const filename = item.input_path.split("/").pop() ?? item.input_path;
      const margin = item.margin_pct ?? 0;
      const description =
        margin > 0
          ? `${item.engine ?? "engine"} beat the runner-up by ${margin.toFixed(0)}%`
          : `${item.engine ?? "engine"} won`;
      toast.success(
        `${filename} — saved ${item.savings_pct.toFixed(0)}%`,
        description,
      );
    } else if (item.status === "failed" && !announced.has(item.id)) {
      announced.add(item.id);
      const filename = item.input_path.split("/").pop() ?? item.input_path;
      toast.error(`${filename} failed`, item.error_message ?? "unknown error");
    }
  }

  async function init() {
    if (initialised) return;
    if (initialising) return initialising;

    // Subscribe before taking the snapshot so a fast completion cannot fall
    // into the gap between the two operations.
    initialising = (async () => {
      unlisten = await listen<QueueItem>("queue:item_updated", (event) => {
        apply_update(event.payload);
      });
      await refresh();
      // MCP jobs share the Rust queue but are submitted outside the Tauri
      // event channel. Polling keeps those agent-created jobs visible.
      statusPoll = setInterval(() => void refresh(), 2_000);
      initialised = true;
    })();
    try {
      await initialising;
    } finally {
      initialising = undefined;
    }
  }

  function dispose() {
    unlisten?.();
    unlisten = undefined;
    if (statusPoll) clearInterval(statusPoll);
    statusPoll = undefined;
    initialised = false;
  }

  async function cancel(job_id: string) {
    await cancelJob(job_id);
    await refresh();
  }

  async function set_paused(next: boolean) {
    if (next) await pauseQueue();
    else await resumeQueue();
    paused = next;
  }

  function toggle_pause() {
    set_paused(!paused);
  }

  return {
    get items() {
      return items;
    },
    get paused() {
      return paused;
    },
    get active_count() {
      return active_count;
    },
    get completed_count() {
      return completed_count;
    },
    get failed_count() {
      return failed_count;
    },
    get initialised() {
      return initialised;
    },
    init,
    dispose,
    refresh,
    cancel,
    set_paused,
    toggle_pause,
  };
}

export const queue = create_queue_store();

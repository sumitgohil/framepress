import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import { analyticsSnapshot, statsSnapshot } from "$lib/ipc/commands";
import type {
  AnalyticsSnapshot,
  QueueItem,
  StatsSnapshot,
} from "$lib/ipc/types";

function create_statistics_store() {
  let summary = $state<StatsSnapshot | null>(null);
  let weekly = $state<AnalyticsSnapshot | null>(null);
  let loading = $state(true);
  let unavailable = $state(false);
  let initialising: Promise<void> | undefined;
  let unlisten: UnlistenFn | undefined;
  let refresh_timer: ReturnType<typeof setTimeout> | undefined;

  async function refresh() {
    try {
      const [next_summary, next_weekly] = await Promise.all([
        statsSnapshot(),
        analyticsSnapshot("7d"),
      ]);
      summary = next_summary;
      weekly = next_weekly;
      unavailable = false;
    } catch {
      unavailable = true;
    } finally {
      loading = false;
    }
  }

  function refresh_after_completion() {
    if (refresh_timer) clearTimeout(refresh_timer);
    refresh_timer = setTimeout(() => void refresh(), 150);
  }

  async function init() {
    if (initialising) return initialising;
    initialising = (async () => {
      unlisten = await listen<QueueItem>("queue:item_updated", (event) => {
        if (
          event.payload.status === "completed" ||
          event.payload.status === "failed"
        ) {
          refresh_after_completion();
        }
      });
      await refresh();
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
    if (refresh_timer) clearTimeout(refresh_timer);
  }

  return {
    get summary() {
      return summary;
    },
    get weekly() {
      return weekly;
    },
    get loading() {
      return loading;
    },
    get unavailable() {
      return unavailable;
    },
    init,
    refresh,
    dispose,
  };
}

export const statistics = create_statistics_store();

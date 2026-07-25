<script lang="ts">
  import { onMount } from "svelte";
  import { Clock } from "lucide-svelte";

  import HistoryRow from "$lib/components/HistoryRow.svelte";
  import Toaster from "$lib/components/Toaster.svelte";

  import { recentHistory, statsSnapshot } from "$lib/ipc/commands";
  import type {
    HistoryRow as HistoryRowData,
    StatsSnapshot,
  } from "$lib/ipc/types";
  import { format_bytes } from "$lib/utils/format";

  let rows = $state<HistoryRowData[]>([]);
  let stats = $state<StatsSnapshot | null>(null);
  let loading = $state(true);

  onMount(async () => {
    try {
      [rows, stats] = await Promise.all([recentHistory(500), statsSnapshot()]);
    } catch {
      // backend not yet ready
    } finally {
      loading = false;
    }
  });
</script>

<svelte:head>
  <title>History · FramePress</title>
</svelte:head>

<div class="mx-auto flex max-w-3xl flex-col gap-6 px-8 py-10">
  <header class="space-y-1">
    <h1 class="text-2xl font-semibold tracking-tight">History</h1>
    {#if stats}
      <p class="text-sm text-[var(--color-muted-foreground)]">
        {stats.total_optimized_count.toLocaleString()} total · saved
        <span class="font-medium text-[var(--color-success)]"
          >{format_bytes(stats.today_savings_bytes)}</span
        >
        today · avg
        <span class="font-mono">{stats.average_savings_pct.toFixed(0)}%</span>
      </p>
    {/if}
  </header>

  {#if loading}
    <div
      class="glass flex items-center justify-center gap-3 rounded-2xl p-10 text-sm text-[var(--color-muted-foreground)]"
    >
      Loading…
    </div>
  {:else if rows.length === 0}
    <div
      class="glass flex flex-col items-center gap-3 rounded-2xl p-12 text-center"
    >
      <div
        class="flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
        aria-hidden="true"
      >
        <Clock size={22} />
      </div>
      <div>
        <p class="text-sm font-medium">No history yet</p>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">
          Drop some images on the Dashboard to start building your history.
        </p>
      </div>
    </div>
  {:else}
    <ul class="flex flex-col gap-1.5">
      {#each rows as row (row.id)}
        <li><HistoryRow {row} /></li>
      {/each}
    </ul>
  {/if}
</div>

<Toaster />

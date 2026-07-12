<script lang="ts">
  import { onMount } from 'svelte';
  import { ArrowUpRight, Droplet, Settings, X } from 'lucide-svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import DropZone from '$lib/components/DropZone.svelte';
  import PresetSelector from '$lib/components/PresetSelector.svelte';
  import { optimizePaths, recentHistory, showMainWindow } from '$lib/ipc/commands';
  import type { HistoryRow } from '$lib/ipc/types';
  import { settings } from '$lib/stores/settings.svelte';
  import { format_bytes } from '$lib/utils/format';

  let recent = $state<HistoryRow[]>([]);
  let enqueuing = $state(false);

  onMount(() => {
    void refresh_recent();
  });

  async function refresh_recent() {
    try {
      recent = await recentHistory(3);
    } catch {
      recent = [];
    }
  }

  async function handle_files(paths: string[]) {
    if (paths.length === 0 || enqueuing) return;
    enqueuing = true;
    try {
      await optimizePaths({ paths, preset: settings.value.default_preset });
      await refresh_recent();
    } finally {
      enqueuing = false;
    }
  }

  async function open_dashboard() {
    await showMainWindow();
  }

  async function hide_widget() {
    await getCurrentWindow().hide();
  }
</script>

<svelte:head>
  <title>TinyDrop</title>
</svelte:head>

<div class="min-h-full bg-[var(--color-card)] p-4 text-[var(--color-foreground)]">
  <header class="mb-5 flex items-center justify-between">
    <div class="flex items-center gap-2.5">
      <span
        class="flex h-8 w-8 items-center justify-center rounded-xl bg-[var(--color-brand-500)] text-white shadow-[var(--shadow-glow)]"
      >
        <Droplet size={17} fill="currentColor" />
      </span>
      <h1 class="text-lg font-semibold tracking-tight">TinyDrop</h1>
    </div>
    <div class="flex items-center gap-1">
      <button
        class="rounded-md p-1.5 text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
        type="button"
        aria-label="Open settings in dashboard"
        onclick={open_dashboard}
      >
        <Settings size={17} />
      </button>
      <button
        class="rounded-full bg-[var(--color-muted)] p-2 text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
        type="button"
        aria-label="Close compact widget"
        onclick={hide_widget}
      >
        <X size={17} />
      </button>
    </div>
  </header>

  <DropZone onfiles={handle_files} compact hint={enqueuing ? 'Adding images…' : undefined} />

  <section class="mt-4 space-y-2" aria-label="Compression preset">
    <p class="text-xs font-medium text-[var(--color-muted-foreground)]">Preset</p>
    <PresetSelector />
  </section>

  <section class="mt-5" aria-label="Recent optimizations">
    <div class="mb-2 flex items-center justify-between">
      <h2 class="text-sm font-semibold">Recent</h2>
      <button
        type="button"
        class="text-xs font-medium text-[var(--color-brand-500)] hover:underline"
        onclick={open_dashboard}
      >
        Show all
      </button>
    </div>
    {#if recent.length === 0}
      <p class="rounded-xl border border-[var(--color-border)] px-3 py-4 text-center text-xs text-[var(--color-muted-foreground)]">
        Optimized images will appear here.
      </p>
    {:else}
      <ul class="space-y-1.5">
        {#each recent as item (item.id)}
          {@const name = item.input_path.split('/').pop() ?? item.input_path}
          {@const saving = item.original_bytes > 0 && item.optimized_bytes !== null
            ? Math.round((1 - item.optimized_bytes / item.original_bytes) * 100)
            : null}
          <li class="flex items-center gap-2 rounded-xl px-2 py-2 hover:bg-[var(--color-muted)]">
            <div class="h-9 w-9 shrink-0 rounded-lg bg-[var(--color-muted)]"></div>
            <div class="min-w-0 flex-1">
              <p class="truncate text-xs font-medium">{name}</p>
              <p class="text-[11px] text-[var(--color-muted-foreground)]">
                {#if item.optimized_bytes !== null}
                  {format_bytes(item.optimized_bytes)}
                  {#if saving !== null && saving > 0} · <span class="text-[var(--color-success)]">−{saving}%</span>{/if}
                {:else}
                  Processing…
                {/if}
              </p>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <button
    type="button"
    class="mt-5 flex w-full items-center justify-center gap-1.5 rounded-lg border border-[var(--color-border)] py-2 text-xs font-medium text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
    onclick={open_dashboard}
  >
    Open Dashboard <ArrowUpRight size={14} />
  </button>
</div>

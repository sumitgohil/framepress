<script lang="ts">
  import { onMount } from 'svelte';

  import DropZone from '$lib/components/DropZone.svelte';
  import PresetSelector from '$lib/components/PresetSelector.svelte';
  import RecentOptimizations from '$lib/components/RecentOptimizations.svelte';
  import Toaster from '$lib/components/Toaster.svelte';

  import { optimizePaths, recentHistory } from '$lib/ipc/commands';
  import type { HistoryRow } from '$lib/ipc/types';
  import { settings } from '$lib/stores/settings.svelte';
  import { toast } from '$lib/stores/toast.svelte';

  let recent = $state<HistoryRow[]>([]);
  let enqueuing = $state(false);
  let preset_open = $state(false);

  onMount(async () => {
    try {
      recent = await recentHistory(50);
    } catch {
      // Backend not yet wired (Branch 6). Leave recent empty.
    }
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
        `${ids.length} ${ids.length === 1 ? 'file' : 'files'} queued`,
        'Open the Queue tab to track progress.',
      );
    } catch (err) {
      toast.error('Could not enqueue files', String(err));
    } finally {
      enqueuing = false;
    }
  }
</script>

<svelte:head>
  <title>Dashboard · TinyDrop</title>
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

  <RecentOptimizations rows={recent} />
</div>

<Toaster />

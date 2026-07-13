<script lang="ts">
  import { ChevronDown, ChevronUp, X, AlertCircle, CheckCircle2, Sparkles, FolderOpen } from 'lucide-svelte';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';

  import ImagePreview from '$lib/components/ImagePreview.svelte';
  import { existingWebpCopy, exportWebpCopy } from '$lib/ipc/commands';
  import type { QueueItem } from '$lib/ipc/types';
  import { PRESET_LABELS } from '$lib/stores/settings.svelte';
  import { toast } from '$lib/stores/toast.svelte';
  import { format_bytes } from '$lib/utils/format';
  import { cn } from '$lib/utils/cn';

  type Props = {
    item: QueueItem;
    oncancel: (id: string) => void;
  };

  let { item, oncancel }: Props = $props();
  let expanded = $state(false);
  let exporting_webp = $state(false);
  let webp_copy_path = $state<string | null>(null);

  let filename = $derived(item.input_path.split('/').pop() ?? item.input_path);
  let webp_recommendation = $derived(item.format === 'png' || item.format === 'jpeg');
  let preset_label = $derived(PRESET_LABELS[item.preset]);

  let status_label = $derived.by(() => {
    switch (item.status) {
      case 'pending':
        return 'Queued';
      case 'running':
        return 'Optimizing…';
      case 'completed':
        return item.savings_pct !== null && item.savings_pct > 0
          ? `Saved ${item.savings_pct.toFixed(0)}%`
          : 'Completed';
      case 'failed':
        return 'Failed';
      case 'cancelled':
        return 'Cancelled';
    }
  });

  let status_color = $derived.by(() => {
    switch (item.status) {
      case 'completed':
        return 'text-[var(--color-success)]';
      case 'failed':
        return 'text-[var(--color-danger)]';
      case 'cancelled':
        return 'text-[var(--color-muted-foreground)]';
      default:
        return 'text-[var(--color-brand-500)]';
    }
  });

  let progress_pct = $derived.by(() => {
    if (item.status === 'completed' || item.status === 'failed' || item.status === 'cancelled') {
      return 100;
    }
    if (item.status === 'running') return 50; // indeterminate mid-flight
    return 5;
  });

  onMount(async () => {
    if (!webp_recommendation) return;
    try {
      const copy = await existingWebpCopy(item.input_path);
      webp_copy_path = copy?.output_path ?? null;
    } catch {
      // The export affordance remains available when the file check is unavailable.
    }
  });

  async function create_webp_copy() {
    if (exporting_webp) return;
    exporting_webp = true;
    try {
      const copy = await exportWebpCopy({ inputPath: item.input_path, preset: item.preset });
      webp_copy_path = copy.output_path;
      toast.success('WebP copy created', `${format_bytes(copy.optimized_bytes)} · original unchanged`);
    } catch (error) {
      toast.error('WebP export failed', String(error));
    } finally {
      exporting_webp = false;
    }
  }

  async function show_webp_copy() {
    if (!webp_copy_path) return;
    try {
      await revealItemInDir(webp_copy_path);
    } catch {
      toast.error('Could not show WebP copy', 'The output may have been moved or deleted.');
    }
  }
</script>

<article
  class="glass rounded-xl p-4 transition-colors"
  data-status={item.status}
  aria-label="{filename} — {status_label}"
>
  <div class="flex items-start gap-3">
    <!-- Thumbnail / icon -->
    <div
      class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
      aria-hidden="true"
    >
      <ImagePreview paths={[item.output_path, item.input_path]} size={20} />
    </div>

    <!-- Middle: name + status -->
    <div class="min-w-0 flex-1 space-y-1.5">
      <div class="flex items-baseline justify-between gap-2">
        <p class="truncate text-sm font-semibold">{filename}</p>
        <button
          type="button"
          class="text-xs font-medium text-[var(--color-brand-500)] hover:underline disabled:opacity-50"
          onclick={() => (expanded = !expanded)}
          disabled={!item.candidates_log}
          aria-expanded={expanded}
        >
          {expanded ? 'Hide' : 'Details'}
        </button>
      </div>

      <div class="flex items-center gap-2 text-xs text-[var(--color-muted-foreground)]">
        <span class={cn('font-medium', status_color)}>
          {#if item.status === 'completed'}
            <CheckCircle2 size={12} class="inline -mt-0.5" />
          {:else if item.status === 'failed'}
            <AlertCircle size={12} class="inline -mt-0.5" />
          {/if}
          {status_label}
        </span>
        {#if item.engine && item.status === 'completed'}
          <span aria-hidden="true">·</span>
          <span>via {item.engine}</span>
        {/if}
        <span aria-hidden="true">·</span>
        <span>{preset_label}</span>
        {#if item.original_bytes !== null}
          <span aria-hidden="true">·</span>
          <span class="font-mono tabular-nums">
            {#if item.optimized_bytes !== null && item.status === 'completed'}
              {format_bytes(item.optimized_bytes)}
              <span class="text-[var(--color-muted-foreground)]">
                ({format_bytes(item.original_bytes)})
              </span>
            {:else}
              {format_bytes(item.original_bytes)}
            {/if}
          </span>
        {/if}
      </div>

      <!-- Progress bar -->
      <div
        class="h-1.5 w-full overflow-hidden rounded-full bg-[var(--color-muted)]"
        role="progressbar"
        aria-valuenow={progress_pct}
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <div
          class={cn(
            'h-full rounded-full transition-[width] duration-500 ease-out',
            item.status === 'completed'
              ? 'bg-[var(--color-success)]'
              : item.status === 'failed' || item.status === 'cancelled'
                ? 'bg-[var(--color-danger)]'
                : 'bg-[var(--color-brand-500)]',
          )}
          style="width: {progress_pct}%;"
        ></div>
      </div>

      {#if item.error_message && (item.status === 'failed' || item.status === 'cancelled')}
        <p class="text-xs text-[var(--color-danger)]">{item.error_message}</p>
      {/if}

      {#if item.status === 'completed' && webp_recommendation}
        <div class="flex items-start gap-2 rounded-lg bg-[var(--color-brand-500)]/8 px-2.5 py-2 text-xs leading-5 text-[var(--color-muted-foreground)]">
          <Sparkles size={14} class="mt-0.5 shrink-0 text-[var(--color-brand-500)]" aria-hidden="true" />
          <span>
            Need a smaller web asset? Export a separate WebP copy when your destination supports it—this {item.format?.toUpperCase()} stays unchanged.
          </span>
          <button
            type="button"
            class="ml-auto inline-flex shrink-0 items-center gap-1 rounded-md bg-[var(--color-brand-500)] px-2.5 py-1.5 font-semibold text-white transition-opacity hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
            onclick={webp_copy_path ? show_webp_copy : create_webp_copy}
            disabled={exporting_webp}
          >
            {#if webp_copy_path}
              <FolderOpen size={13} aria-hidden="true" />
              Show WebP copy
            {:else}
              {exporting_webp ? 'Creating…' : 'Create WebP copy'}
            {/if}
          </button>
        </div>
      {/if}
    </div>

    <!-- Cancel button -->
    {#if item.status === 'pending' || item.status === 'running'}
      <button
        type="button"
        class="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-danger)]"
        aria-label="Cancel {filename}"
        onclick={() => oncancel(item.id)}
      >
        <X size={14} strokeWidth={2} />
      </button>
    {/if}
  </div>

  {#if expanded && item.candidates_log && item.candidates_log.length > 0}
    <div class="mt-3 space-y-1.5 border-t border-[var(--color-border)] pt-3">
      <p class="text-[11px] font-medium tracking-wide text-[var(--color-muted-foreground)] uppercase">
        Engine log
      </p>
      <ul class="space-y-1 text-xs">
        {#each item.candidates_log as log (log.engine)}
          <li class="flex items-center justify-between gap-2">
            <span class="flex items-center gap-2">
              {#if log.passed_gate}
                <CheckCircle2 size={12} class="text-[var(--color-success)]" />
              {:else}
                <AlertCircle size={12} class="text-[var(--color-warning)]" />
              {/if}
              <span class="font-medium">{log.engine}</span>
            </span>
            <span class="flex items-center gap-3 font-mono tabular-nums text-[var(--color-muted-foreground)]">
              <span>{format_bytes(log.output_bytes)}</span>
              {#if log.dssim !== null}
                <span class="text-[10px]">visual difference {log.dssim.toFixed(4)}</span>
              {/if}
            </span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</article>

<script lang="ts">
  import { FolderOpen, ExternalLink, AlertCircle, CheckCircle2 } from 'lucide-svelte';
  import { revealItemInDir, openPath } from '@tauri-apps/plugin-opener';

  import ImagePreview from '$lib/components/ImagePreview.svelte';
  import type { HistoryRow } from '$lib/ipc/types';
  import { format_bytes, format_relative } from '$lib/utils/format';

  type Props = {
    row: HistoryRow;
  };

  let { row }: Props = $props();

  let filename = $derived(row.input_path.split('/').pop() ?? row.input_path);
  let savings = $derived(
    row.original_bytes > 0 && row.optimized_bytes !== null
      ? Math.round(((row.original_bytes - row.optimized_bytes) / row.original_bytes) * 100)
      : null,
  );

  async function reveal() {
    if (!row.output_path) return;
    try {
      await revealItemInDir(row.output_path);
    } catch {
      // best-effort
    }
  }

  async function open() {
    if (!row.output_path) return;
    try {
      await openPath(row.output_path);
    } catch {
      // best-effort
    }
  }
</script>

<article
  class="glass flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-[var(--color-muted)]"
  aria-label="{filename} — {row.status}"
>
  <div
    class="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
    aria-hidden="true"
  >
    <ImagePreview paths={[row.thumbnail_path, row.output_path, row.input_path]} size={16} />
  </div>

  <div class="min-w-0 flex-1">
    <p class="truncate text-sm font-medium">{filename}</p>
    <p class="text-xs text-[var(--color-muted-foreground)]">
      {#if row.optimized_bytes !== null && savings !== null}
        <span class="font-mono tabular-nums">{format_bytes(row.optimized_bytes)}</span>
        <span class="text-[var(--color-muted-foreground)]"> of {format_bytes(row.original_bytes)}</span>
        {#if savings > 0}
          <span class="ml-1 text-[var(--color-success)]">−{savings}%</span>
        {/if}
      {:else}
        <span class="italic">{row.error_message ?? row.status.toLowerCase()}</span>
      {/if}
      · {format_relative(row.completed_at ?? row.started_at)}
      {#if row.engine}
        · {row.engine}
      {/if}
    </p>
  </div>

  {#if row.status === 'completed'}
    <CheckCircle2 size={16} class="shrink-0 text-[var(--color-success)]" />
  {:else if row.status === 'failed'}
    <AlertCircle size={16} class="shrink-0 text-[var(--color-danger)]" />
  {/if}

  {#if row.output_path}
    <div class="flex shrink-0 items-center gap-1">
      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
        onclick={reveal}
        aria-label="Show {filename} in Finder"
        title="Show in Finder"
      >
        <FolderOpen size={14} />
      </button>
      <button
        type="button"
        class="flex h-7 w-7 items-center justify-center rounded-md text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
        onclick={open}
        aria-label="Open {filename}"
        title="Open file"
      >
        <ExternalLink size={14} />
      </button>
    </div>
  {/if}
</article>

<script lang="ts">
  import { CheckCircle2, CircleDashed, Image, XCircle } from 'lucide-svelte';

  import ImagePreview from '$lib/components/ImagePreview.svelte';
  import type { HistoryRow } from '$lib/ipc/types';
  import { format_bytes, format_relative } from '$lib/utils/format';

  type Props = {
    rows: HistoryRow[];
  };

  let { rows }: Props = $props();

  let visible = $derived(rows.slice(0, 3));
</script>

<section class="space-y-3" aria-label="Recent optimizations">
  <div class="flex items-center justify-between">
    <h2 class="text-sm font-semibold tracking-tight">Recent Optimizations</h2>
    <a
      href="/history"
      class="rounded-lg bg-[var(--color-muted)] px-3 py-1.5 text-xs font-medium text-[var(--color-muted-foreground)] transition-colors hover:text-[var(--color-foreground)]"
    >
      View All
    </a>
  </div>

  {#if visible.length === 0}
    <div
      class="glass flex min-h-22 items-center gap-3 rounded-xl px-5 text-sm text-[var(--color-muted-foreground)]"
    >
      <Image size={18} aria-hidden="true" />
      Drop an image to see your first optimization here.
    </div>
  {:else}
    <ul class="glass overflow-hidden rounded-xl divide-y divide-[var(--color-border)]">
      {#each visible as row (row.id)}
        {@const savings = row.original_bytes > 0 && row.optimized_bytes !== null
          ? Math.max(0, Math.round(((row.original_bytes - row.optimized_bytes) / row.original_bytes) * 100))
          : null}
        {@const saved_bytes = row.optimized_bytes === null
          ? null
          : Math.max(0, row.original_bytes - row.optimized_bytes)}
        <li>
          <article class="grid grid-cols-[minmax(0,1fr)_auto_auto_auto] items-center gap-x-5 px-4 py-3.5 sm:px-5">
            <div class="flex min-w-0 items-center gap-3.5">
              <div
                class="flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
                aria-hidden="true"
              >
                <ImagePreview paths={[row.thumbnail_path, row.output_path, row.input_path]} size={18} />
              </div>
              <div class="min-w-0">
                <p class="truncate text-sm font-semibold tracking-tight">
                  {row.input_path.split('/').pop() ?? row.input_path}
                </p>
                <p class="mt-0.5 text-xs text-[var(--color-muted-foreground)]">
                  {row.format.toUpperCase()} · {row.engine ?? row.preset}
                </p>
              </div>
            </div>

            <div class="hidden min-w-20 text-right text-sm font-semibold tabular-nums text-[var(--color-foreground)] sm:block">
              {saved_bytes === null ? '—' : `↓ ${format_bytes(saved_bytes)}`}
            </div>

            <div class="hidden min-w-11 text-right sm:block">
              {#if savings !== null}
                <span class="rounded-md bg-[var(--color-success)]/12 px-1.5 py-0.5 text-xs font-semibold tabular-nums text-[var(--color-success)]">
                  {savings}%
                </span>
              {/if}
            </div>

            <div class="flex min-w-20 items-center justify-end gap-3 text-xs text-[var(--color-muted-foreground)]">
              <span class="hidden md:inline">{format_relative(row.completed_at ?? row.started_at)}</span>
              {#if row.status === 'completed'}
                <CheckCircle2 size={19} strokeWidth={2.25} class="text-[var(--color-success)]" />
              {:else if row.status === 'failed'}
                <XCircle size={19} strokeWidth={2.25} class="text-[var(--color-danger)]" />
              {:else if row.status === 'cancelled'}
                <CircleDashed size={19} strokeWidth={2.25} class="text-[var(--color-muted-foreground)]" />
              {:else}
                <CircleDashed size={19} strokeWidth={2.25} class="animate-spin text-[var(--color-brand-500)]" />
              {/if}
            </div>
          </article>
        </li>
      {/each}
    </ul>
  {/if}
</section>

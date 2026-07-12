<script lang="ts">
  import { Image, CheckCircle2, ArrowRight } from 'lucide-svelte';

  import type { HistoryRow } from '$lib/ipc/types';
  import { format_bytes, format_relative } from '$lib/utils/format';

  type Props = {
    rows: HistoryRow[];
  };

  let { rows }: Props = $props();

  let visible = $derived(rows.slice(0, 5));
</script>

<section class="space-y-3" aria-label="Recent optimizations">
  <div class="flex items-center justify-between">
    <h2 class="text-base font-semibold tracking-tight">Recent Optimizations</h2>
    {#if rows.length > 0}
      <a
        href="/history"
        class="text-xs font-medium text-[var(--color-brand-500)] hover:underline"
      >
        View All
      </a>
    {/if}
  </div>

  {#if visible.length === 0}
    <div
      class="glass flex items-center gap-3 rounded-xl p-4 text-sm text-[var(--color-muted-foreground)]"
    >
      <Image size={18} aria-hidden="true" />
      Drop an image to see your first optimization here.
    </div>
  {:else}
    <ul class="space-y-1.5">
      {#each visible as row (row.id)}
        {@const savings = row.original_bytes > 0 && row.optimized_bytes !== null
          ? Math.round(((row.original_bytes - row.optimized_bytes) / row.original_bytes) * 100)
          : null}
        <li>
          <article
            class="glass flex items-center gap-3 rounded-xl px-3 py-2.5 transition-colors hover:bg-[var(--color-muted)]"
          >
            <div
              class="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
              aria-hidden="true"
            >
              {#if row.thumbnail_path}
                <img
                  src="file://{row.thumbnail_path}"
                  alt=""
                  class="h-full w-full object-cover"
                />
              {:else}
                <Image size={16} />
              {/if}
            </div>
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium">
                {row.input_path.split('/').pop() ?? row.input_path}
              </p>
              <p class="text-xs text-[var(--color-muted-foreground)]">
                {#if row.optimized_bytes !== null}
                  <span class="font-mono">{format_bytes(row.optimized_bytes)}</span>
                  {#if savings !== null && savings > 0}
                    <span class="ml-1 text-[var(--color-success)]">−{savings}%</span>
                  {/if}
                {:else}
                  <span class="italic">pending</span>
                {/if}
                · {format_relative(row.completed_at ?? row.started_at)}
              </p>
            </div>
            {#if row.status === 'Completed'}
              <CheckCircle2 size={16} class="text-[var(--color-success)]" />
            {:else}
              <ArrowRight size={16} class="text-[var(--color-muted-foreground)]" />
            {/if}
          </article>
        </li>
      {/each}
    </ul>
  {/if}
</section>
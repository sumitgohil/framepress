<script lang="ts">
  import { ListTodo, Pause, Play } from "lucide-svelte";

  import QueueCard from "$lib/components/QueueCard.svelte";
  import Toaster from "$lib/components/Toaster.svelte";

  import { queue } from "$lib/stores/queue.svelte";
  import { toast } from "$lib/stores/toast.svelte";

  async function handle_cancel(job_id: string) {
    try {
      await queue.cancel(job_id);
      toast.info("Cancelled", "The job was stopped.");
    } catch (err) {
      toast.error("Cancel failed", String(err));
    }
  }
</script>

<svelte:head>
  <title>Queue · FramePress</title>
</svelte:head>

<div class="mx-auto flex max-w-3xl flex-col gap-6 px-8 py-10">
  <header class="flex items-end justify-between gap-4">
    <div class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">Queue</h1>
      <p class="text-sm text-[var(--color-muted-foreground)]">
        {queue.active_count} active · {queue.completed_count} completed
        {#if queue.failed_count > 0}
          · {queue.failed_count} stopped
        {/if}
      </p>
    </div>
    {#if queue.active_count > 0 || queue.items.length > 0}
      <button
        type="button"
        class="flex h-9 items-center gap-1.5 rounded-lg border border-[var(--color-border)] bg-transparent px-3 text-sm font-medium hover:bg-[var(--color-muted)]"
        onclick={() => queue.toggle_pause()}
        aria-label={queue.paused ? "Resume queue" : "Pause queue"}
      >
        {#if queue.paused}
          <Play size={14} aria-hidden="true" />
          Resume
        {:else}
          <Pause size={14} aria-hidden="true" />
          Pause
        {/if}
      </button>
    {/if}
  </header>

  {#if queue.items.length === 0}
    <div
      class="glass flex flex-col items-center justify-center gap-3 rounded-2xl p-12 text-center"
    >
      <div
        class="flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--color-brand-500)]/10 text-[var(--color-brand-500)]"
        aria-hidden="true"
      >
        <ListTodo size={22} />
      </div>
      <div>
        <p class="text-sm font-medium">Nothing in the queue</p>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">
          Drop some images on the Dashboard and they'll show up here.
        </p>
      </div>
      <a
        href="/"
        class="mt-1 inline-flex h-9 items-center rounded-lg bg-[var(--color-brand-500)] px-4 text-sm font-medium text-white shadow-[var(--shadow-glow)] hover:opacity-90"
      >
        Open Dashboard
      </a>
    </div>
  {:else}
    <div class="flex flex-col gap-2">
      {#each queue.items as item (item.id)}
        <QueueCard {item} oncancel={handle_cancel} />
      {/each}
    </div>
  {/if}
</div>

<Toaster />

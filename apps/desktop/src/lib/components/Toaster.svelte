<script lang="ts">
  import { CheckCircle2, AlertCircle, Info, X } from "lucide-svelte";

  import { toast, type Toast } from "$lib/stores/toast.svelte";

  function icon_for(t: Toast) {
    if (t.variant === "success") return CheckCircle2;
    if (t.variant === "error") return AlertCircle;
    return Info;
  }
</script>

<!-- Toast container — fixed top-right, glassmorphic. -->
<div
  aria-live="polite"
  aria-atomic="false"
  class="pointer-events-none fixed top-4 right-4 z-[100] flex w-96 max-w-[calc(100vw-2rem)] flex-col gap-2"
>
  {#each toast.items as t (t.id)}
    {@const Icon = icon_for(t)}
    <div
      role="status"
      class="glass-strong pointer-events-auto flex items-start gap-3 rounded-xl p-3.5 shadow-[var(--shadow-elevated)]"
      data-variant={t.variant}
    >
      <span
        class="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center"
        aria-hidden="true"
      >
        <Icon
          size={18}
          strokeWidth={2}
          class={t.variant === "success"
            ? "text-[var(--color-success)]"
            : t.variant === "error"
              ? "text-[var(--color-danger)]"
              : "text-[var(--color-info)]"}
        />
      </span>
      <div class="flex-1 space-y-0.5">
        <p class="text-sm font-semibold leading-snug">{t.title}</p>
        {#if t.description}
          <p class="text-xs text-[var(--color-muted-foreground)]">
            {t.description}
          </p>
        {/if}
        {#if t.action}
          <button
            type="button"
            class="mt-1 text-xs font-medium text-[var(--color-brand-500)] hover:underline"
            onclick={t.action.on_click}
          >
            {t.action.label}
          </button>
        {/if}
      </div>
      <button
        type="button"
        aria-label="Dismiss notification"
        class="ml-1 flex h-6 w-6 shrink-0 items-center justify-center rounded-md text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)]"
        onclick={() => toast.dismiss(t.id)}
      >
        <X size={14} strokeWidth={2} />
      </button>
    </div>
  {/each}
</div>

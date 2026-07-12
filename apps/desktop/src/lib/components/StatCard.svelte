<script lang="ts">
  /**
   * Small stat card. Shown in the sidebar bottom block (Today / Cumulative).
   * No charts/sparklines in v1 — pure numeric display.
   */
  type Props = {
    label: string;
    value: string;
    hint?: string;
  };

  let { label, value, hint }: Props = $props();
</script>

<div
  class="glass flex flex-col gap-0.5 rounded-xl p-3"
  aria-label={label}
>
  <span class="text-[11px] font-medium tracking-wide text-[var(--color-muted-foreground)] uppercase">
    {label}
  </span>
  <span class="text-2xl font-semibold tabular-nums tracking-tight">
    {value}
  </span>
  {#if hint}
    <span class="text-[11px] text-[var(--color-muted-foreground)]">
      {hint}
    </span>
  {/if}
</div>
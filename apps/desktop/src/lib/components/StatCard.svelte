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

<div class="flex flex-col gap-1" aria-label={label}>
  <span class="text-sm font-medium text-[var(--color-muted-foreground)]">
    {label}
  </span>
  <span class="text-[1.65rem] font-semibold tabular-nums tracking-tight">
    {value}
  </span>
  {#if hint}
    <span class="text-xs text-[var(--color-muted-foreground)]">
      {hint}
    </span>
  {/if}
</div>

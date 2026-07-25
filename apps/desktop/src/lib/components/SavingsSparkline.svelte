<script lang="ts">
  import type { SavingsTrendPoint } from "$lib/ipc/types";

  type Props = { points: SavingsTrendPoint[] };
  let { points }: Props = $props();

  let values = $derived(points.map((point) => point.saved_bytes));
  let max = $derived(Math.max(...values, 1));
  let line = $derived.by(() =>
    values
      .map((value, index) => {
        const x = values.length <= 1 ? 50 : (index / (values.length - 1)) * 100;
        const y = 34 - (value / max) * 28;
        return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
      })
      .join(" "),
  );
  let area = $derived(values.length ? `${line} L 100 38 L 0 38 Z` : "");
</script>

<svg
  viewBox="0 0 100 40"
  class="h-10 w-24"
  role="img"
  aria-label="Savings over the last seven days"
>
  <defs>
    <linearGradient id="spark-fill" x1="0" x2="0" y1="0" y2="1">
      <stop
        offset="0%"
        stop-color="var(--color-brand-400)"
        stop-opacity="0.42"
      />
      <stop
        offset="100%"
        stop-color="var(--color-brand-400)"
        stop-opacity="0"
      />
    </linearGradient>
  </defs>
  {#if values.some((value) => value > 0)}
    <path d={area} fill="url(#spark-fill)" />
    <path
      d={line}
      fill="none"
      stroke="var(--color-brand-400)"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  {:else}
    <path
      d="M 0 30 L 100 30"
      fill="none"
      stroke="var(--color-border)"
      stroke-width="2"
      stroke-linecap="round"
    />
  {/if}
</svg>

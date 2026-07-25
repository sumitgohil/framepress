<script lang="ts">
  import { onMount } from "svelte";
  import {
    FolderOpen,
    LoaderCircle,
    Sparkles,
    TrendingDown,
  } from "lucide-svelte";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  import ImagePreview from "$lib/components/ImagePreview.svelte";
  import { analyticsSnapshot } from "$lib/ipc/commands";
  import type {
    AnalyticsRange,
    AnalyticsSnapshot,
    BiggestWin,
  } from "$lib/ipc/types";
  import { PRESET_LABELS } from "$lib/stores/settings.svelte";
  import { format_bytes } from "$lib/utils/format";

  const ranges: Array<{ key: AnalyticsRange; label: string }> = [
    { key: "7d", label: "7 days" },
    { key: "30d", label: "30 days" },
    { key: "all", label: "All time" },
  ];
  const chart_colors = [
    "var(--color-brand-400)",
    "var(--color-success)",
    "var(--color-info)",
    "var(--color-warning)",
    "var(--color-brand-200)",
  ];
  const chart_width = 720;
  const chart_height = 240;
  const chart_left = 12;
  const chart_right = 12;
  const chart_top = 14;
  const chart_bottom = 28;

  let range = $state<AnalyticsRange>("7d");
  let analytics = $state<AnalyticsSnapshot | null>(null);
  let loading = $state(true);
  let error = $state(false);
  let active_point = $state<number | null>(null);

  async function load() {
    loading = true;
    error = false;
    try {
      analytics = await analyticsSnapshot(range);
    } catch {
      error = true;
    } finally {
      loading = false;
    }
  }

  function choose_range(next: AnalyticsRange) {
    if (range === next) return;
    range = next;
    void load();
  }

  function readable_period(period: string) {
    const source =
      period.length === 7 ? `${period}-01T12:00:00` : `${period}T12:00:00`;
    const date = new Date(source);
    return period.length === 7
      ? new Intl.DateTimeFormat(undefined, {
          month: "short",
          year: "numeric",
        }).format(date)
      : new Intl.DateTimeFormat(undefined, {
          month: "short",
          day: "numeric",
        }).format(date);
  }

  function preset_label(preset: string) {
    return PRESET_LABELS[preset as keyof typeof PRESET_LABELS] ?? preset;
  }

  async function reveal(win: BiggestWin) {
    if (!win.output_path || !win.output_exists) return;
    try {
      await revealItemInDir(win.output_path);
    } catch {
      /* file may have moved after data loaded */
    }
  }

  let trend = $derived(analytics?.trend ?? []);
  let max_saved = $derived(
    Math.max(...trend.map((point) => point.saved_bytes), 1),
  );
  let chart_points = $derived.by(() =>
    trend.map((point, index) => ({
      ...point,
      x:
        trend.length <= 1
          ? chart_width / 2
          : chart_left +
            (index / (trend.length - 1)) *
              (chart_width - chart_left - chart_right),
      y:
        chart_height -
        chart_bottom -
        (point.saved_bytes / max_saved) *
          (chart_height - chart_top - chart_bottom),
    })),
  );
  let line_path = $derived(
    chart_points
      .map(
        (point, index) =>
          `${index ? "L" : "M"} ${point.x.toFixed(2)} ${point.y.toFixed(2)}`,
      )
      .join(" "),
  );
  let area_path = $derived(
    chart_points.length
      ? `${line_path} L ${chart_width - chart_right} ${chart_height - chart_bottom} L ${chart_left} ${chart_height - chart_bottom} Z`
      : "",
  );
  let format_total = $derived(
    analytics?.formats.reduce((total, item) => total + item.saved_bytes, 0) ??
      0,
  );
  let format_gradient = $derived.by(() => {
    if (!analytics?.formats.length || format_total === 0)
      return "conic-gradient(var(--color-muted) 0 100%)";
    let cursor = 0;
    const parts = analytics.formats.map((item, index) => {
      const end = cursor + (item.saved_bytes / format_total) * 100;
      const part = `${chart_colors[index % chart_colors.length]} ${cursor}% ${end}%`;
      cursor = end;
      return part;
    });
    return `conic-gradient(${parts.join(", ")})`;
  });

  onMount(() => {
    void load();
  });
</script>

<svelte:head><title>Statistics · FramePress</title></svelte:head>

<div class="mx-auto flex w-full max-w-6xl flex-col gap-6 px-6 py-8 lg:px-10">
  <header class="flex flex-wrap items-end justify-between gap-4">
    <div>
      <p class="mb-1 text-sm font-medium text-[var(--color-brand-400)]">
        Your optimization impact
      </p>
      <h1 class="text-3xl font-semibold tracking-tight">Statistics</h1>
      <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
        See where FramePress is making your images lighter.
      </p>
    </div>
    <div
      class="inline-flex rounded-xl border border-[var(--color-border)] bg-[var(--color-muted)] p-1"
      aria-label="Statistics period"
    >
      {#each ranges as option (option.key)}
        <button
          type="button"
          class="rounded-lg px-3 py-1.5 text-sm font-medium transition-colors {range ===
          option.key
            ? 'bg-[var(--color-card)] text-[var(--color-foreground)] shadow-sm'
            : 'text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]'}"
          aria-pressed={range === option.key}
          onclick={() => choose_range(option.key)}
        >
          {option.label}
        </button>
      {/each}
    </div>
  </header>

  {#if loading}
    <div
      class="glass flex min-h-80 items-center justify-center gap-3 rounded-2xl text-sm text-[var(--color-muted-foreground)]"
    >
      <LoaderCircle size={18} class="animate-spin" /> Loading statistics…
    </div>
  {:else if error || !analytics}
    <div class="glass rounded-2xl p-8 text-center">
      <p class="font-medium">Statistics are unavailable</p>
      <button
        type="button"
        class="mt-3 text-sm font-medium text-[var(--color-brand-400)]"
        onclick={load}>Try again</button
      >
    </div>
  {:else}
    <section
      class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"
      aria-label="Summary statistics"
    >
      <div class="glass rounded-2xl p-4">
        <p class="text-sm text-[var(--color-muted-foreground)]">Space saved</p>
        <p class="mt-2 text-2xl font-semibold tabular-nums">
          {format_bytes(analytics.saved_bytes)}
        </p>
        <p
          class="mt-1 flex items-center gap-1 text-xs {analytics.savings_change_pct !==
            null && analytics.savings_change_pct < 0
            ? 'text-[var(--color-danger)]'
            : 'text-[var(--color-success)]'}"
        >
          {#if analytics.savings_change_pct !== null}{analytics.savings_change_pct >=
            0
              ? "↑"
              : "↓"}
            {Math.abs(analytics.savings_change_pct).toFixed(0)}% vs previous
            period{:else}All recorded savings{/if}
        </p>
      </div>
      <div class="glass rounded-2xl p-4">
        <p class="text-sm text-[var(--color-muted-foreground)]">
          Images optimized
        </p>
        <p class="mt-2 text-2xl font-semibold tabular-nums">
          {analytics.optimized_count.toLocaleString()}
        </p>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">
          Completed in this range
        </p>
      </div>
      <div class="glass rounded-2xl p-4">
        <p class="text-sm text-[var(--color-muted-foreground)]">
          Average reduction
        </p>
        <p
          class="mt-2 text-2xl font-semibold tabular-nums text-[var(--color-success)]"
        >
          {analytics.average_savings_pct.toFixed(0)}%
        </p>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">
          Across completed images
        </p>
      </div>
      <div class="glass rounded-2xl p-4">
        <p class="text-sm text-[var(--color-muted-foreground)]">
          Original data processed
        </p>
        <p class="mt-2 text-2xl font-semibold tabular-nums">
          {format_bytes(analytics.input_bytes)}
        </p>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">
          Before optimization
        </p>
      </div>
    </section>

    {#if analytics.optimized_count === 0}
      <section
        class="glass flex min-h-72 flex-col items-center justify-center rounded-2xl p-8 text-center"
      >
        <div
          class="flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--color-brand-500)]/10 text-[var(--color-brand-400)]"
        >
          <Sparkles size={26} />
        </div>
        <h2 class="mt-4 text-lg font-semibold">
          Your savings story starts here
        </h2>
        <p class="mt-1 max-w-sm text-sm text-[var(--color-muted-foreground)]">
          Optimize an image to see trends, format insights, and your biggest
          file-size wins.
        </p>
        <a
          href="/"
          class="mt-5 rounded-xl bg-[var(--color-brand-500)] px-4 py-2 text-sm font-semibold text-white"
          >Optimize images</a
        >
      </section>
    {:else}
      <section class="glass rounded-2xl p-5">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="font-semibold">Savings over time</h2>
            <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
              Bytes removed from completed optimizations.
            </p>
          </div>
          <TrendingDown size={20} class="text-[var(--color-brand-400)]" />
        </div>
        <div
          class="relative mt-5 h-64 overflow-hidden rounded-xl bg-[var(--color-muted)]/25"
        >
          <svg
            viewBox={`0 0 ${chart_width} ${chart_height}`}
            preserveAspectRatio="none"
            class="h-full w-full"
            role="img"
            aria-label="Savings trend chart"
          >
            <defs
              ><linearGradient id="statistics-area" x1="0" x2="0" y1="0" y2="1"
                ><stop
                  offset="0%"
                  stop-color="var(--color-brand-400)"
                  stop-opacity="0.42"
                /><stop
                  offset="100%"
                  stop-color="var(--color-brand-400)"
                  stop-opacity="0"
                /></linearGradient
              ></defs
            >
            {#each [chart_top, (chart_top + chart_height - chart_bottom) / 2, chart_height - chart_bottom] as y}
              <path
                d={`M ${chart_left} ${y} L ${chart_width - chart_right} ${y}`}
                stroke="var(--color-border)"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
                stroke-dasharray={y === chart_height - chart_bottom
                  ? undefined
                  : "3 5"}
              />
            {/each}
            <path d={area_path} fill="url(#statistics-area)" />
            <path
              d={line_path}
              fill="none"
              stroke="var(--color-brand-400)"
              stroke-width="1.5"
              vector-effect="non-scaling-stroke"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
            {#each chart_points as point, index (point.period)}
              {#if point.saved_bytes > 0}
                <circle
                  cx={point.x}
                  cy={point.y}
                  r="3.5"
                  fill="var(--color-card)"
                  stroke="var(--color-brand-400)"
                  stroke-width="2"
                  vector-effect="non-scaling-stroke"
                  tabindex="0"
                  role="button"
                  aria-label={`${readable_period(point.period)}: ${format_bytes(point.saved_bytes)} saved`}
                  onmouseenter={() => (active_point = index)}
                  onfocus={() => (active_point = index)}
                />
              {/if}
            {/each}
          </svg>
          {#if active_point !== null && trend[active_point]}
            {@const point = trend[active_point]}
            <div
              class="absolute right-3 top-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] px-3 py-2 text-xs shadow-[var(--shadow-elevated)]"
            >
              <p class="font-medium">{readable_period(point.period)}</p>
              <p class="mt-1 text-[var(--color-success)]">
                {format_bytes(point.saved_bytes)} saved · {point.optimized_count}
                images
              </p>
            </div>
          {/if}
        </div>
        <div
          class="mt-2 flex justify-between text-[11px] text-[var(--color-muted-foreground)]"
        >
          <span>{trend[0] ? readable_period(trend[0].period) : ""}</span><span
            >{trend.at(-1)
              ? readable_period(trend.at(-1)?.period ?? "")
              : ""}</span
          >
        </div>
      </section>

      <section class="grid gap-5 lg:grid-cols-2">
        <div class="glass rounded-2xl p-5">
          <div class="text-center">
            <h2 class="font-semibold">Savings by format</h2>
            <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
              Which original formats shed the most weight.
            </p>
          </div>
          <div class="mt-6 flex flex-col items-center">
            <div
              class="relative grid h-32 w-32 shrink-0 place-items-center rounded-full"
              style:background={format_gradient}
            >
              <div
                class="flex h-20 w-20 flex-col items-center justify-center rounded-full bg-[var(--color-card)] text-center"
              >
                <span class="text-lg font-semibold leading-none"
                  >{format_bytes(format_total)}</span
                ><span
                  class="mt-2 text-[10px] leading-none text-[var(--color-muted-foreground)]"
                  >saved</span
                >
              </div>
            </div>
            <div
              class="mt-5 grid w-full max-w-sm grid-cols-1 gap-2 sm:grid-cols-2"
            >
              {#each analytics.formats as item, index (item.key)}<div
                  class="flex items-center justify-between gap-2 rounded-xl bg-[var(--color-muted)]/70 px-3 py-2 text-sm"
                >
                  <span class="flex min-w-0 items-center gap-2 truncate"
                    ><i
                      class="h-2.5 w-2.5 shrink-0 rounded-full"
                      style:background={chart_colors[
                        index % chart_colors.length
                      ]}
                    ></i>{item.key.toUpperCase()}</span
                  ><span
                    class="shrink-0 text-right font-mono text-[11px] leading-4 text-[var(--color-muted-foreground)]"
                    >{format_bytes(item.saved_bytes)}<br />{format_total
                      ? Math.round((item.saved_bytes / format_total) * 100)
                      : 0}%</span
                  >
                </div>{/each}
            </div>
          </div>
        </div>
        <div class="glass rounded-2xl p-5">
          <h2 class="font-semibold">Savings by preset</h2>
          <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
            The presets delivering the most impact.
          </p>
          <div class="mt-5 space-y-4">
            {#each analytics.presets as item (item.key)}<div>
                <div class="flex justify-between gap-3 text-sm">
                  <span class="truncate">{preset_label(item.key)}</span><span
                    class="shrink-0 font-mono text-xs text-[var(--color-muted-foreground)]"
                    >{format_bytes(item.saved_bytes)} · {item.optimized_count}</span
                  >
                </div>
                <div
                  class="mt-1.5 h-2 overflow-hidden rounded-full bg-[var(--color-muted)]"
                >
                  <div
                    class="h-full rounded-full bg-[var(--color-brand-400)] transition-[width] duration-500"
                    style:width={`${analytics.saved_bytes ? Math.max(4, (item.saved_bytes / analytics.saved_bytes) * 100) : 0}%`}
                  ></div>
                </div>
              </div>{/each}
          </div>
        </div>
      </section>

      <section class="glass rounded-2xl p-5">
        <h2 class="font-semibold">Usage by source</h2>
        <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
          See what was optimized in FramePress versus through an agent using
          MCP.
        </p>
        <div class="mt-5 space-y-4">
          {#each analytics.sources as item (item.key)}
            <div>
              <div class="flex justify-between gap-3 text-sm">
                <span
                  class={item.key.startsWith("Agent (MCP)")
                    ? "font-medium text-[var(--color-brand-400)]"
                    : "font-medium"}>{item.key}</span
                ><span
                  class="shrink-0 font-mono text-xs text-[var(--color-muted-foreground)]"
                  >{format_bytes(item.saved_bytes)} · {item.optimized_count} images</span
                >
              </div>
              <div
                class="mt-1.5 h-2 overflow-hidden rounded-full bg-[var(--color-muted)]"
              >
                <div
                  class="h-full rounded-full bg-[var(--color-info)] transition-[width] duration-500"
                  style:width={`${analytics.saved_bytes ? Math.max(4, (item.saved_bytes / analytics.saved_bytes) * 100) : 0}%`}
                ></div>
              </div>
            </div>
          {/each}
        </div>
      </section>

      <section class="glass rounded-2xl p-5">
        <div class="flex items-center justify-between gap-4">
          <div>
            <h2 class="font-semibold">Biggest wins</h2>
            <p class="mt-1 text-sm text-[var(--color-muted-foreground)]">
              Your largest individual reductions in this range.
            </p>
          </div>
          <a
            href="/history"
            class="text-sm font-medium text-[var(--color-brand-400)]"
            >View History</a
          >
        </div>
        <div class="mt-4 divide-y divide-[var(--color-border)]">
          {#each analytics.biggest_wins as win (win.input_path + win.completed_at)}<div
              class="flex items-center gap-3 py-3"
            >
              <div
                class="flex h-10 w-10 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-[var(--color-muted)] text-[var(--color-muted-foreground)]"
              >
                <ImagePreview
                  paths={[win.thumbnail_path, win.output_path, win.input_path]}
                  size={16}
                />
              </div>
              <div class="min-w-0 flex-1">
                <p class="truncate text-sm font-medium">
                  {win.input_path.split("/").pop() ?? win.input_path}
                </p>
                <p class="mt-0.5 text-xs text-[var(--color-muted-foreground)]">
                  {win.format.toUpperCase()} · {preset_label(
                    win.preset,
                  )}{#if win.engine}
                    · {win.engine}{/if}
                </p>
              </div>
              <div class="text-right">
                <p class="text-sm font-semibold text-[var(--color-success)]">
                  ↓ {format_bytes(win.saved_bytes)}
                </p>
                <p class="text-xs text-[var(--color-muted-foreground)]">
                  {win.savings_pct.toFixed(0)}% smaller
                </p>
              </div>
              {#if win.output_exists}<button
                  type="button"
                  class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
                  onclick={() => reveal(win)}
                  title="Show output in Finder"
                  aria-label="Show {win.input_path.split('/').pop()} in Finder"
                  ><FolderOpen size={15} /></button
                >{/if}
            </div>{/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

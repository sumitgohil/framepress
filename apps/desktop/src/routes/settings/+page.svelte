<script lang="ts">
  import { settings, PRESET_LABELS, PRESET_DESCRIPTIONS } from '$lib/stores/settings.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import { Sun, Moon, Monitor, Sliders } from 'lucide-svelte';
  import type { CompressionPreset } from '$lib/ipc/types';
  import { PRESET_KEYS } from '$lib/ipc/types';

  let preset: CompressionPreset = $derived(settings.value.default_preset);
  let threshold = $state(settings.value.dssim_threshold);
  let advanced = $state(settings.value.show_advanced);

  function set_preset(value: CompressionPreset) {
    settings.set({ default_preset: value });
  }

  function set_threshold(value: number) {
    threshold = Math.max(0, Math.min(0.05, value));
    settings.set({ dssim_threshold: threshold });
  }
</script>

<svelte:head>
  <title>Settings · TinyDrop</title>
</svelte:head>

<div class="mx-auto flex max-w-2xl flex-col gap-8 px-8 py-10">
  <header class="space-y-1">
    <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>
    <p class="text-sm text-[var(--color-muted-foreground)]">
      Personalize TinyDrop. Changes save automatically.
    </p>
  </header>

  <!-- Default preset -->
  <section class="glass rounded-2xl p-5" aria-label="Default preset">
    <h2 class="mb-3 text-sm font-semibold tracking-tight">Default Preset</h2>
    <div class="grid grid-cols-1 gap-1.5 sm:grid-cols-2">
      {#each PRESET_KEYS as p (p)}
        <button
          type="button"
          onclick={() => set_preset(p)}
          aria-pressed={preset === p}
          class="flex flex-col items-start gap-0.5 rounded-lg border p-2.5 text-left transition-colors"
          class:border-[var(--color-brand-500)]={preset === p}
          class:bg-[var(--color-brand-500)]={false}
          class:border-[var(--color-border)]={preset !== p}
          class:hover:bg-[var(--color-muted)]={preset !== p}
          style={preset === p
            ? 'background: color-mix(in oklch, var(--color-brand-500) 10%, transparent);'
            : ''}
        >
          <span class="text-sm font-medium">{PRESET_LABELS[p]}</span>
          <span class="text-xs text-[var(--color-muted-foreground)]">{PRESET_DESCRIPTIONS[p]}</span>
        </button>
      {/each}
    </div>
  </section>

  <!-- Theme -->
  <section class="glass rounded-2xl p-5" aria-label="Appearance">
    <h2 class="mb-3 text-sm font-semibold tracking-tight">Appearance</h2>
    <div class="flex gap-1.5">
      {#each [{ id: 'light', label: 'Light', icon: Sun }, { id: 'dark', label: 'Dark', icon: Moon }, { id: 'system', label: 'System', icon: Monitor }] as opt (opt.id)}
        {@const Icon = opt.icon}
        <button
          type="button"
          onclick={() => theme.set(opt.id as 'light' | 'dark' | 'system')}
          aria-pressed={theme.mode === opt.id}
          class="flex h-10 flex-1 items-center justify-center gap-1.5 rounded-lg border text-sm font-medium transition-colors"
          class:border-[var(--color-brand-500)]={theme.mode === opt.id}
          class:border-[var(--color-border)]={theme.mode !== opt.id}
          class:hover:bg-[var(--color-muted)]={theme.mode !== opt.id}
          style={theme.mode === opt.id
            ? 'background: color-mix(in oklch, var(--color-brand-500) 10%, transparent);'
            : ''}
        >
          <Icon size={14} />
          {opt.label}
        </button>
      {/each}
    </div>
  </section>

  <!-- Advanced -->
  <section class="glass rounded-2xl p-5" aria-label="Advanced">
    <button
      type="button"
      class="flex w-full items-center justify-between gap-2"
      onclick={() => {
        advanced = !advanced;
        settings.set({ show_advanced: advanced });
      }}
      aria-expanded={advanced}
    >
      <h2 class="flex items-center gap-2 text-sm font-semibold tracking-tight">
        <Sliders size={14} />
        Advanced
      </h2>
      <span class="text-xs text-[var(--color-muted-foreground)]">{advanced ? 'Hide' : 'Show'}</span>
    </button>

    {#if advanced}
      <div class="mt-4 space-y-4 border-t border-[var(--color-border)] pt-4">
        <div>
          <label for="dssim" class="block text-sm font-medium">
            DSSIM threshold
            <span class="ml-1 text-xs font-normal text-[var(--color-muted-foreground)]">
              lower = closer to original
            </span>
          </label>
          <div class="mt-2 flex items-center gap-3">
            <input
              id="dssim"
              type="range"
              min="0"
              max="0.05"
              step="0.0005"
              value={threshold}
              oninput={(e) => set_threshold(Number((e.target as HTMLInputElement).value))}
              class="flex-1 accent-[var(--color-brand-500)]"
            />
            <span class="w-16 text-right font-mono text-xs tabular-nums">
              {threshold.toFixed(4)}
            </span>
          </div>
        </div>
      </div>
    {/if}
  </section>
</div>
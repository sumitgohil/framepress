<script lang="ts">
  import { ChevronDown } from "lucide-svelte";
  import {
    PRESET_DESCRIPTIONS,
    PRESET_LABELS,
    settings,
    type SettingsState,
  } from "$lib/stores/settings.svelte";
  import { cn } from "$lib/utils/cn";
  import type { CompressionPreset } from "$lib/ipc/types";
  import { PRESET_KEYS } from "$lib/ipc/types";

  type Props = {
    /** Lets the containing surface raise its stacking layer while the menu is open. */
    open?: boolean;
  };

  let { open = $bindable(false) }: Props = $props();
  let root_el: HTMLDivElement | null = $state(null);

  function close_on_outside_click(event: MouseEvent) {
    if (!root_el) return;
    if (!root_el.contains(event.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener("mousedown", close_on_outside_click);
    return () =>
      document.removeEventListener("mousedown", close_on_outside_click);
  });

  function select(preset: CompressionPreset) {
    settings.set({ default_preset: preset });
    open = false;
  }

  function handle_key(event: KeyboardEvent, preset: CompressionPreset) {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      select(preset);
    }
  }

  let current: SettingsState["default_preset"] = $derived(
    settings.value.default_preset,
  );
</script>

<div class="relative w-full" bind:this={root_el}>
  <button
    type="button"
    onclick={() => (open = !open)}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-label="Choose compression preset"
    class={cn(
      "flex h-11 w-full items-center justify-between gap-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] px-3 text-sm font-medium text-[var(--color-foreground)] shadow-sm transition-colors",
      "hover:bg-[var(--color-muted)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-ring)]",
    )}
  >
    <span class="flex items-center gap-2">
      <span
        class="flex h-6 w-6 items-center justify-center rounded-md bg-[var(--color-brand-500)]/10 text-[var(--color-brand-500)]"
        aria-hidden="true"
      >
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none">
          <path
            d="M2 4h12M2 8h12M2 12h6"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </span>
      {PRESET_LABELS[current]}
    </span>
    <ChevronDown
      size={16}
      strokeWidth={2}
      class={cn("transition-transform duration-200", open && "rotate-180")}
    />
  </button>

  {#if open}
    <div
      role="listbox"
      class="absolute top-full left-0 z-50 mt-1.5 w-full overflow-hidden rounded-lg border border-[var(--color-border)] bg-[var(--color-card)] shadow-[var(--shadow-elevated)]"
    >
      <ul class="max-h-72 overflow-y-auto py-1">
        {#each PRESET_KEYS as preset (preset)}
          {@const selected = preset === current}
          <li>
            <button
              type="button"
              role="option"
              aria-selected={selected}
              onclick={() => select(preset)}
              onkeydown={(e) => handle_key(e, preset)}
              class={cn(
                "flex w-full flex-col items-start gap-0.5 px-3 py-2.5 text-left transition-colors",
                selected
                  ? "bg-[var(--color-brand-500)]/10 text-[var(--color-brand-600)] dark:text-[var(--color-brand-300)]"
                  : "text-[var(--color-foreground)] hover:bg-[var(--color-muted)]",
              )}
            >
              <span class="text-sm font-medium">{PRESET_LABELS[preset]}</span>
              <span class="text-xs text-[var(--color-muted-foreground)]">
                {PRESET_DESCRIPTIONS[preset]}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>

<script lang="ts">
  import { CloudUpload, ImagePlus } from 'lucide-svelte';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  import { cn } from '$lib/utils/cn';

  type Props = {
    /** Called with the resolved file paths when the user drops or selects. */
    onfiles: (paths: string[]) => void;
    /** Optional: a small message under the drop zone (e.g. loading state). */
    hint?: string;
    /** Compact treatment for the menu-bar widget. */
    compact?: boolean;
  };

  let { onfiles, hint, compact = false }: Props = $props();

  let drag_active = $state(false);
  let listener_cleanup: (() => void) | null = null;

  const SUPPORTED = ['PNG', 'JPG', 'JPEG', 'WebP', 'AVIF', 'GIF', 'SVG', 'TIFF', 'HEIC'];

  // Tauri v2 emits a native file-drop event when files are dropped onto the
  // window from Finder/Explorer. We listen for it and forward to the parent.
  onMount(async () => {
    const unlisten = await listen<{ paths: string[] }>('tauri://drag-drop', (event) => {
      drag_active = false;
      onfiles(event.payload.paths);
    });
    listener_cleanup = unlisten;
  });

  onDestroy(() => listener_cleanup?.());

  async function pick_files() {
    const selected = await openDialog({
      multiple: true,
      directory: false,
      title: 'Select images to optimize',
      filters: [
        {
          name: 'Images',
          extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'svg'],
        },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    onfiles(paths);
  }

  function handle_key(event: KeyboardEvent) {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      pick_files();
    }
  }
</script>

<button
  type="button"
  class={cn(
    'group relative flex w-full flex-col items-center justify-center gap-3 overflow-hidden rounded-2xl border-2 border-dashed text-center transition-all duration-200 ease-out',
    compact ? 'px-5 py-8' : 'min-h-[15.25rem] px-8 py-12',
    drag_active
      ? 'scale-[1.02] border-[var(--color-brand-500)] bg-[var(--color-brand-500)]/10 shadow-[var(--shadow-glow)]'
      : 'border-[var(--color-brand-500)]/40 bg-[var(--color-brand-500)]/5 hover:border-[var(--color-brand-500)]/70 hover:bg-[var(--color-brand-500)]/10',
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-ring)] focus-visible:ring-offset-2',
  )}
  onclick={pick_files}
  onkeydown={handle_key}
  ondragenter={(e) => {
    e.preventDefault();
    drag_active = true;
  }}
  ondragover={(e) => {
    e.preventDefault();
    drag_active = true;
  }}
  ondragleave={() => (drag_active = false)}
  ondrop={(e) => {
    e.preventDefault();
    drag_active = false;
    // Real files arrive via the Tauri event; the browser-level drop is a
    // no-op fallback for in-page drag of items.
  }}
  aria-label="Drop images here, or press Enter to browse"
>
  <!-- Soft animated radial wash -->
  <div
    aria-hidden="true"
    class={cn(
      'pointer-events-none absolute inset-0 -z-10 opacity-0 transition-opacity duration-500',
      drag_active && 'opacity-100',
    )}
    style="background: radial-gradient(circle at center, color-mix(in oklch, var(--color-brand-500) 20%, transparent), transparent 60%);"
  ></div>

  <div
    class={cn(
      'flex items-center justify-center rounded-full bg-[var(--color-brand-500)]/10 text-[var(--color-brand-500)] transition-transform duration-300',
      compact ? 'h-12 w-12' : 'h-16 w-16',
      drag_active ? 'scale-110' : 'group-hover:scale-105',
    )}
  >
    {#if drag_active}
      <ImagePlus size={compact ? 25 : 32} strokeWidth={1.75} />
    {:else}
      <CloudUpload size={compact ? 25 : 32} strokeWidth={1.75} />
    {/if}
  </div>

  <div class="space-y-1.5">
    <p class={cn('text-base font-semibold tracking-tight', !compact && 'text-lg')}>
      {drag_active ? 'Release to add files' : compact ? 'Drop to optimize' : 'Drop images or folders here'}
    </p>
    {#if !compact}
      <p class="text-xs tracking-wide text-[var(--color-muted-foreground)] uppercase">
        {SUPPORTED.join(', ')}
      </p>
    {/if}
  </div>

  <p class="text-sm text-[var(--color-muted-foreground)]">
    or
    <span class="font-medium text-[var(--color-brand-500)] underline-offset-4 hover:underline">
      click to browse
    </span>
  </p>

  {#if hint}
    <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">{hint}</p>
  {/if}
</button>

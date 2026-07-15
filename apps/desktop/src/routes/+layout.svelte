<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { ModeWatcher } from 'mode-watcher';
  import { onMount } from 'svelte';
  import { queue } from '$lib/stores/queue.svelte';
  import { statistics } from '$lib/stores/statistics.svelte';
  import { draggable_titlebar } from '$lib/utils/draggable_titlebar';

  let { children } = $props();
  onMount(() => {
    // Apply theme on first mount.
    document.documentElement.classList.toggle(
      'dark',
      window.matchMedia('(prefers-color-scheme: dark)').matches,
    );
    void queue.init();
    void statistics.init();
    return () => {
      queue.dispose();
      statistics.dispose();
    };
  });
</script>

<ModeWatcher />

<!--
  The window uses titleBarStyle "Overlay" + hiddenTitle, so macOS renders no
  native titlebar to grab. This strip reclaims the top of the window as a
  draggable region. The draggable_titlebar action calls startDragging() on
  mousedown only when the target isn't an interactive child, so buttons /
  links / inputs inside still work. Native traffic-light controls render
  above this element and keep working. z-40 keeps it below the Toaster
  (z-[100]).
-->
<div
  use:draggable_titlebar
  class="fixed inset-x-0 top-0 z-40 h-10"
></div>

<div class="flex h-full overflow-hidden bg-[var(--color-background)]">
  <Sidebar />
  <main class="relative flex-1 overflow-y-auto">
    {@render children()}
  </main>
</div>

<script lang="ts">
  import '../app.css';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { ModeWatcher } from 'mode-watcher';
  import { onMount } from 'svelte';
  import { queue } from '$lib/stores/queue.svelte';
  import { statistics } from '$lib/stores/statistics.svelte';

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

<div class="flex h-full overflow-hidden bg-[var(--color-background)]">
  <Sidebar />
  <main class="relative flex-1 overflow-y-auto">
    {@render children()}
  </main>
</div>

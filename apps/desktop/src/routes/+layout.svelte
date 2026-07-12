<script lang="ts">
  import '../app.css';
  import { page } from '$app/stores';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { ModeWatcher } from 'mode-watcher';
  import { onMount } from 'svelte';
  import { queue } from '$lib/stores/queue.svelte';

  let { children } = $props();
  let is_widget = $derived($page.url.pathname === '/widget');

  onMount(() => {
    // Apply theme on first mount.
    document.documentElement.classList.toggle(
      'dark',
      window.matchMedia('(prefers-color-scheme: dark)').matches,
    );
    void queue.init();
    return () => queue.dispose();
  });
</script>

<ModeWatcher />

<div class="flex h-full overflow-hidden bg-[var(--color-background)]">
  {#if !is_widget}
    <Sidebar />
  {/if}
  <main class:overflow-y-auto={!is_widget} class="relative flex-1">
    {@render children()}
  </main>
</div>

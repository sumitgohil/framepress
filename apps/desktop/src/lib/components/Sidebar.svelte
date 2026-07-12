<script lang="ts">
  import { page } from '$app/stores';
  import { Home, List, Clock, Settings, Droplet, Layers } from 'lucide-svelte';
  import { cn } from '$lib/utils/cn';
  import { queue } from '$lib/stores/queue.svelte';
  import StatCard from './StatCard.svelte';

  type NavItem = {
    href: string;
    label: string;
    icon: typeof Home;
    badge?: number;
  };

  // Spec-required nav for Phase 1 (see ARCHITECTURE.md). Watch Folders and
  // Presets are explicitly deferred to Phase 2/3.
  let nav_items: NavItem[] = $derived.by(() => {
    const items: NavItem[] = [
      { href: '/', label: 'Dashboard', icon: Home },
      { href: '/queue', label: 'Queue', icon: List, badge: queue.active_count },
      { href: '/history', label: 'History', icon: Clock },
      { href: '/settings', label: 'Settings', icon: Settings },
    ];
    return items;
  });

  let is_active = (href: string) => {
    if (href === '/') return $page.url.pathname === '/';
    return $page.url.pathname.startsWith(href);
  };
</script>

<aside
  class="glass-strong flex h-full w-60 shrink-0 flex-col border-r border-[var(--color-border)] p-4"
  aria-label="Primary navigation"
>
  <!-- Brand -->
  <div class="flex items-center gap-2.5 px-2 py-3">
    <div
      class="flex h-8 w-8 items-center justify-center rounded-lg bg-[var(--color-brand-500)] text-white shadow-[var(--shadow-glow)]"
      aria-hidden="true"
    >
      <Droplet size={18} fill="currentColor" />
    </div>
    <span class="text-base font-semibold tracking-tight">TinyDrop</span>
  </div>

  <!-- Nav -->
  <nav class="mt-4 flex flex-col gap-0.5">
    {#each nav_items as item (item.href)}
      {@const Icon = item.icon}
      {@const active = is_active(item.href)}
      <a
        href={item.href}
        aria-current={active ? 'page' : undefined}
        class={cn(
          'group flex h-9 items-center justify-between gap-2 rounded-lg px-2.5 text-sm font-medium transition-colors',
          active
            ? 'bg-[var(--color-brand-500)]/10 text-[var(--color-brand-600)] dark:text-[var(--color-brand-300)]'
            : 'text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]',
        )}
      >
        <span class="flex items-center gap-2.5">
          <Icon size={16} strokeWidth={2} aria-hidden="true" />
          {item.label}
        </span>
        {#if item.badge !== undefined && item.badge > 0}
          <span
            class="flex h-5 min-w-5 items-center justify-center rounded-full bg-[var(--color-brand-500)] px-1.5 text-[10px] font-semibold text-white"
            aria-label="{item.badge} pending"
          >
            {item.badge}
          </span>
        {/if}
      </a>
    {/each}
  </nav>

  <!-- Spacer pushes stats to the bottom -->
  <div class="mt-auto flex flex-col gap-2 pt-6">
    <StatCard label="Today's Savings" value="—" hint="coming online" />
    <StatCard label="Images Optimized" value="0" hint="cumulative" />
    <button
      class="mt-1 flex h-9 items-center justify-center gap-1.5 rounded-lg border border-[var(--color-border)] bg-transparent text-xs font-medium text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
      type="button"
      disabled
      title="Phase 2 — view full statistics"
    >
      <Layers size={14} aria-hidden="true" />
      View All Statistics
    </button>
  </div>
</aside>

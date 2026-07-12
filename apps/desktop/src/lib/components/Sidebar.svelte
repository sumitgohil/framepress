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
  class="glass-strong flex h-full w-60 shrink-0 flex-col border-r border-[var(--color-border)] p-5"
  aria-label="Primary navigation"
>
  <!-- Brand -->
  <div class="flex items-center gap-3 px-2 py-2">
    <div
      class="flex h-10 w-10 items-center justify-center rounded-xl bg-[var(--color-brand-500)] text-white shadow-[var(--shadow-glow)]"
      aria-hidden="true"
    >
      <Droplet size={22} fill="currentColor" />
    </div>
    <span class="text-xl font-semibold tracking-tight">TinyDrop</span>
  </div>

  <!-- Nav -->
  <nav class="mt-7 flex flex-col gap-1">
    {#each nav_items as item (item.href)}
      {@const Icon = item.icon}
      {@const active = is_active(item.href)}
      <a
        href={item.href}
        aria-current={active ? 'page' : undefined}
        class={cn(
          'group flex h-11 items-center justify-between gap-2 rounded-xl px-3 text-[15px] font-medium transition-colors',
          active
            ? 'bg-[var(--color-brand-500)]/10 text-[var(--color-brand-600)] dark:text-[var(--color-brand-300)]'
            : 'text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]',
        )}
      >
        <span class="flex items-center gap-2.5">
          <Icon size={20} strokeWidth={2} aria-hidden="true" />
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
  <div class="glass mt-auto flex flex-col gap-3 rounded-2xl p-5">
    <StatCard label="Today's Savings" value="—" hint="coming online" />
    <div class="h-px bg-[var(--color-border)]"></div>
    <StatCard label="Images Optimized" value="0" hint="cumulative" />
    <button
      class="mt-1 flex h-11 items-center justify-center gap-1.5 rounded-xl bg-[var(--color-muted)] text-sm font-medium text-[var(--color-muted-foreground)] hover:text-[var(--color-foreground)]"
      type="button"
      disabled
      title="Statistics are not available yet"
    >
      <Layers size={14} aria-hidden="true" />
      View All Statistics
    </button>
  </div>
</aside>

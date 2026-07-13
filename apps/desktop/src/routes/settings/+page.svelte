<script lang="ts">
  import { onMount } from 'svelte';
  import { settings, PRESET_LABELS, PRESET_DESCRIPTIONS } from '$lib/stores/settings.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import { Sun, Moon, Monitor, Server, Copy, RefreshCw, Plus, X, CheckCircle2 } from 'lucide-svelte';
  import { mcpConfig, mcpStatus, rotateMcpToken, setMcpEnabled, updateMcpConfig } from '$lib/ipc/commands';
  import type { CompressionPreset, McpConfig, McpServerStatus } from '$lib/ipc/types';
  import { PRESET_KEYS } from '$lib/ipc/types';

  let preset: CompressionPreset = $derived(settings.value.default_preset);
  let agentConfig = $state<McpConfig | null>(null);
  let agentStatus = $state<McpServerStatus | null>(null);
  let rootInput = $state('');
  let copied = $state(false);
  let connectionMessage = $state<string | null>(null);
  let agentError = $state<string | null>(null);

  async function refreshAgentStatus() {
    try {
      agentStatus = await mcpStatus();
      agentError = null;
    } catch (error) {
      agentError = String(error);
    }
  }

  onMount(() => {
    void (async () => {
      try {
        [agentConfig, agentStatus] = await Promise.all([mcpConfig(), mcpStatus()]);
        agentError = null;
      } catch (error) {
        agentError = String(error);
      }
    })();

    const statusPoll = window.setInterval(() => void refreshAgentStatus(), 2_000);
    return () => window.clearInterval(statusPoll);
  });

  function set_preset(value: CompressionPreset) {
    settings.set({ default_preset: value });
  }

  async function toggleAgentAccess(enabled: boolean) {
    try { agentStatus = await setMcpEnabled(enabled); agentConfig = await mcpConfig(); agentError = null; }
    catch (error) { agentError = String(error); }
  }
  async function saveAgentConfig(next: McpConfig) {
    try { agentConfig = await updateMcpConfig(next); agentStatus = await mcpStatus(); agentError = null; }
    catch (error) { agentError = String(error); }
  }
  async function addRoot() {
    if (!agentConfig || !rootInput.trim()) return;
    await saveAgentConfig({ ...agentConfig, approved_roots: [...agentConfig.approved_roots, rootInput.trim()] }); rootInput = '';
  }
  async function copyConfig() {
    if (!agentConfig || !agentStatus) return;
    const snippet = JSON.stringify({ mcpServers: { framepress: { url: agentStatus.endpoint, headers: { Authorization: `Bearer ${agentConfig.token}` } } } }, null, 2);
    await navigator.clipboard.writeText(snippet); copied = true; setTimeout(() => copied = false, 1600);
  }
  async function testConnection() {
    try { const status = await mcpStatus(); connectionMessage = status.running ? `Connected to ${status.endpoint}` : 'MCP server is not running.'; }
    catch (error) { connectionMessage = `Connection failed: ${String(error)}`; }
  }
</script>

<svelte:head>
  <title>Settings · FramePress</title>
</svelte:head>

<div class="mx-auto flex max-w-2xl flex-col gap-8 px-8 py-10">
  <header class="space-y-1">
    <h1 class="text-2xl font-semibold tracking-tight">Settings</h1>
    <p class="text-sm text-[var(--color-muted-foreground)]">
      Personalize FramePress. Changes save automatically.
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

  <!-- Agent access -->
  <section class="glass rounded-2xl p-5" aria-label="Agent Access MCP">
    <div class="flex items-start justify-between gap-4">
      <div>
        <h2 class="flex items-center gap-2 text-sm font-semibold tracking-tight"><Server size={15} /> Agent Access (MCP)</h2>
        <p class="mt-1 text-xs text-[var(--color-muted-foreground)]">Let trusted local agents optimize files through FramePress. Files never leave this computer.</p>
      </div>
      <button type="button" role="switch" aria-label="Expose local MCP server" aria-checked={agentConfig?.enabled ?? false} onclick={() => toggleAgentAccess(!(agentConfig?.enabled ?? false))} class="relative h-6 w-11 shrink-0 rounded-full transition-colors" class:bg-[var(--color-brand-500)]={agentConfig?.enabled} class:bg-[var(--color-muted)]={!agentConfig?.enabled}>
        <span class="absolute top-0.5 h-5 w-5 rounded-full bg-white transition-transform" class:left-0.5={!agentConfig?.enabled} class:left-5={agentConfig?.enabled}></span>
      </button>
    </div>

    {#if agentConfig}
      <div class="mt-4 space-y-4 border-t border-[var(--color-border)] pt-4">
        <div class="flex items-center justify-between rounded-lg bg-[var(--color-muted)] px-3 py-2 text-xs">
          <span class="text-[var(--color-muted-foreground)]">{agentStatus?.running ? 'Running locally' : 'Disabled'}</span>
          <code class="font-mono text-[var(--color-foreground)]">{agentStatus?.endpoint ?? `http://127.0.0.1:${agentConfig.port}/mcp`}</code>
        </div>
        <div class="flex flex-wrap gap-2">
          <button type="button" onclick={copyConfig} disabled={!agentStatus?.running} class="inline-flex h-9 items-center gap-1.5 rounded-lg bg-[var(--color-brand-500)] px-3 text-xs font-medium text-white disabled:opacity-50"><Copy size={13} /> {copied ? 'Copied configuration' : 'Copy MCP configuration'}</button>
          <button type="button" onclick={async () => { agentConfig = await rotateMcpToken(); agentStatus = await mcpStatus(); }} class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-[var(--color-border)] px-3 text-xs font-medium hover:bg-[var(--color-muted)]"><RefreshCw size={13} /> Rotate token</button>
          <button type="button" onclick={testConnection} class="inline-flex h-9 items-center gap-1.5 rounded-lg border border-[var(--color-border)] px-3 text-xs font-medium hover:bg-[var(--color-muted)]"><CheckCircle2 size={13} /> Test connection</button>
        </div>
        {#if connectionMessage}<p class="text-xs text-[var(--color-muted-foreground)]">{connectionMessage}</p>{/if}
        <p class="text-xs text-[var(--color-muted-foreground)]">Use the copied configuration in Codex, Claude Code, Cursor, or another Streamable HTTP MCP client. It includes the local endpoint and bearer token.</p>
        <div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label class="text-xs font-medium">Port<input type="number" min="1" max="65535" value={agentConfig.port} oninput={(event) => saveAgentConfig({ ...agentConfig!, port: Number((event.target as HTMLInputElement).value) })} class="mt-1 block h-9 w-full rounded-lg border border-[var(--color-border)] bg-transparent px-2 font-mono text-xs outline-none" /></label>
          <label class="text-xs font-medium">Maximum batch<input type="number" min="1" max="500" value={agentConfig.max_batch_size} oninput={(event) => saveAgentConfig({ ...agentConfig!, max_batch_size: Number((event.target as HTMLInputElement).value) })} class="mt-1 block h-9 w-full rounded-lg border border-[var(--color-border)] bg-transparent px-2 font-mono text-xs outline-none" /></label>
          <label class="flex items-end gap-2 pb-2 text-xs font-medium"><input type="checkbox" checked={agentConfig.preserve_format} onchange={(event) => saveAgentConfig({ ...agentConfig!, preserve_format: (event.target as HTMLInputElement).checked })} class="accent-[var(--color-brand-500)]" /> Preserve source formats</label>
        </div>
        <div>
          <div class="mb-2 flex items-center justify-between"><p class="text-sm font-medium">Approved folders</p><span class="text-xs text-[var(--color-muted-foreground)]">Agents cannot read outside these roots</span></div>
          <div class="flex gap-2"><input bind:value={rootInput} placeholder="/Users/me/project" class="h-9 min-w-0 flex-1 rounded-lg border border-[var(--color-border)] bg-transparent px-3 text-xs outline-none focus:border-[var(--color-brand-500)]" /><button type="button" onclick={addRoot} class="inline-flex h-9 items-center gap-1 rounded-lg border border-[var(--color-border)] px-3 text-xs font-medium hover:bg-[var(--color-muted)]"><Plus size={13} /> Add</button></div>
          {#if agentConfig.approved_roots.length > 0}<ul class="mt-2 space-y-1">{#each agentConfig.approved_roots as root (root)}<li class="flex items-center justify-between gap-2 rounded-md bg-[var(--color-muted)] px-2.5 py-2 font-mono text-xs"><span class="truncate">{root}</span><button type="button" onclick={() => saveAgentConfig({ ...agentConfig!, approved_roots: agentConfig!.approved_roots.filter((item) => item !== root) })} aria-label="Remove {root}" class="text-[var(--color-muted-foreground)] hover:text-red-400"><X size={14} /></button></li>{/each}</ul>{:else}<p class="mt-2 text-xs text-amber-400">Add a project folder before agents can submit work.</p>{/if}
        </div>
        <details class="rounded-lg border border-[var(--color-border)] p-3 text-xs"><summary class="cursor-pointer font-medium">Available agent tools</summary><p class="mt-2 leading-5 text-[var(--color-muted-foreground)]">Optimize: validate inputs, submit optimization, create WebP copy. Jobs: get status, list, cancel, retry. Results: get file result, history, reveal output. Insights: presets, statistics, capabilities, and access policy.</p></details>
        {#if agentStatus?.running}<p class="flex items-center gap-1.5 text-xs text-[var(--color-success)]"><CheckCircle2 size={13} /> {agentStatus.active_jobs} active agent job{agentStatus.active_jobs === 1 ? '' : 's'}</p>{/if}
      </div>
    {/if}
    {#if agentError}<p class="mt-3 text-xs text-red-400">{agentError}</p>{/if}
  </section>

</div>

<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { LanguageServerSetting, LspInfo, LspLogLine, LspStatus } from '../../lib/types';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // « Serveurs de langage » (Général › Avancé, ESC-446): each server of the catalog, for the project shown (its
  // node_modules, its .escouade.yml): turned on or off, its path and its arguments (the draft of the settings), where it
  // was found or the command that installs it, and the trust a server of the project waits for. Those running: their
  // state, their log, a restart.
  const s = $derived(settingsForm.settings);
  const projectId = $derived(settingsForm.projectId ?? app.project?.id ?? null);
  let servers = $state<LspStatus[]>([]);
  let running = $state<LspInfo[]>([]);
  let log = $state<{ id: number; lines: LspLogLine[] } | null>(null);

  async function refresh() {
    servers = await api.lspServers(projectId).catch(() => []);
    running = await api.lspRunning().catch(() => []);
  }
  onMount(() => void refresh());

  const own = (id: string): LanguageServerSetting => s.languageServers?.[id] ?? {};
  function set(id: string, change: LanguageServerSetting) {
    s.languageServers = { ...(s.languageServers ?? {}), [id]: { ...own(id), ...change } };
  }
  /** The arguments as typed, kept as they are while typed (a space ends none yet). */
  let typed = $state<Record<string, string>>({});
  /** The arguments as typed, split on spaces; empty: the catalog's. */
  function setArgs(id: string, text: string) {
    typed[id] = text;
    const args = text.split(/\s+/).filter(Boolean);
    set(id, { args: args.length ? args : null });
  }

  function where(sv: LspStatus): string {
    if (sv.found) return t('settings.lsp.found', { program: sv.found.program, origin: t(`settings.lsp.origin.${sv.found.origin}`) });
    const m = sv.missing;
    if (m?.kind === 'manual') return t('settings.lsp.manualMissing', { path: m.path });
    if (m?.kind === 'projectFile') return t('settings.lsp.fileMissing', { path: m.path });
    return t('settings.lsp.absent', { install: m?.install ?? sv.install });
  }

  function stateOf(i: LspInfo): string {
    if (i.state === 'restarting') return t('settings.lsp.state.restarting', { s: Math.ceil((i.retryInMs ?? 0) / 1000) });
    return t(`settings.lsp.state.${i.state}`);
  }

  async function trust(on: boolean) {
    if (!projectId) return;
    await app.run(api.lspTrust(projectId, on));
    await refresh();
  }

  async function showLog(id: number) {
    const lines = await app.run(api.lspLog(id));
    if (lines) log = { id, lines };
  }

  async function restart(id: number) {
    await app.run(api.lspRestart(id));
    await refresh();
  }
</script>

{#each servers as sv (sv.id)}
  <Row label={sv.name} desc={where(sv)} shared={`language_servers.${sv.id}`}>
    {#snippet extra()}
      {#if sv.enabledBy === 'project'}<span class="note">{t('settings.lsp.byProject')}</span>{/if}
      {#if sv.runsProjectCode}
        <span class="note"
          >{t('settings.lsp.runsProjectCode', { what: t(`settings.lsp.runs.${sv.id as 'rust' | 'typescript' | 'go'}`) })}</span
        >
      {/if}
      {#if sv.needsTrust && projectId && !sv.trusted}
        {#if sv.ofProject}<span class="note warn">{t('settings.lsp.needsTrust')}</span>{/if}
        <button class="link" onclick={() => trust(true)}>{t('settings.lsp.trust')}</button>
      {:else if sv.needsTrust && projectId}
        <span class="note">{t('settings.lsp.trusted')}</span>
        <button class="link" onclick={() => trust(false)}>{t('settings.lsp.untrust')}</button>
      {/if}
      <div class="fields">
        <input
          class="field mono"
          aria-label={`${sv.name} · ${t('settings.lsp.path')}`}
          spellcheck="false"
          placeholder={t('settings.lsp.pathPlaceholder')}
          bind:value={() => own(sv.id).path ?? '', (v) => set(sv.id, { path: v })}
        />
        <input
          class="field mono"
          aria-label={`${sv.name} · ${t('settings.lsp.args')}`}
          spellcheck="false"
          placeholder={sv.args.join(' ') || t('settings.lsp.argsPlaceholder')}
          bind:value={() => typed[sv.id] ?? (own(sv.id).args ?? []).join(' '), (v) => setArgs(sv.id, v)}
        />
      </div>
    {/snippet}
    <Switch
      label={`${sv.name} · ${t('settings.lsp.on')}`}
      bind:on={() => own(sv.id).enabled ?? true, (on) => set(sv.id, { enabled: on })}
    />
  </Row>
{/each}

{#each running as i (i.id)}
  <Row label={`${i.server} · ${i.root}`} desc={i.error ? `${stateOf(i)} · ${i.error}` : stateOf(i)}>
    <span class="actions">
      <button class="btn" onclick={() => showLog(i.id)}>{t('settings.lsp.log')}</button>
      {#if i.state === 'failed'}<button class="btn" onclick={() => restart(i.id)}>{t('settings.lsp.restart')}</button>{/if}
    </span>
  </Row>
{/each}

{#if log}
  <div class="log" role="log">
    <button class="link" onclick={() => (log = null)}>{t('settings.lsp.closeLog')}</button>
    {#if log.lines.length}
      <pre class="mono">{log.lines.map((l) => `${new Date(l.at).toLocaleTimeString()} ${l.source} ${l.text}`).join('\n')}</pre>
    {:else}
      <p>{t('settings.lsp.noLog')}</p>
    {/if}
  </div>
{/if}

<style>
  .note {
    display: block;
    font-size: 11.5px;
    color: var(--soft);
  }
  .warn {
    color: var(--warn-ink);
  }
  .fields {
    display: flex;
    gap: 8px;
    margin-top: 6px;
    flex-wrap: wrap;
  }
  .fields .field {
    flex: 1 1 200px;
    min-width: 0;
  }
  .actions {
    display: inline-flex;
    gap: 6px;
  }
  .log pre {
    max-height: 240px;
    overflow: auto;
    font-size: 11.5px;
    white-space: pre-wrap;
    word-break: break-all;
  }
</style>

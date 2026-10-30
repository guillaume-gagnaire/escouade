<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { pluginsApi, type JournalKind, type PluginRun, type PluginView } from '../../lib/plugins';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import { phaseText } from './phase';

  // A plugin's journal (§3.4): its starts, stops and crashes, its standard error, its messages, the calls past their
  // delay and what it was refused, read again every 2 seconds while it is shown; filtered by kind, copied whole. Above it,
  // where its process stands, with « Démarrer » (not started yet) or « Relancer » (stopped).
  let { plugin }: { plugin: PluginView } = $props();

  const KINDS: JournalKind[] = [
    'start',
    'ready',
    'stop',
    'crash',
    'stderr',
    'message',
    'timeout',
    'refused',
    'dropped',
    'webhook',
    'command',
    'decision',
  ];
  let run = $state<PluginRun | null>(null);
  let kind = $state<'' | JournalKind>('');
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;

  const shown = $derived((run?.journal ?? []).filter((e) => !kind || e.kind === kind));
  const programmed = $derived(!plugin.declarative && !!plugin.manifest);

  async function read() {
    try {
      run = await pluginsApi.run(plugin.id);
    } catch (e) {
      error = String(e);
    }
  }

  async function start() {
    error = null;
    try {
      run = await pluginsApi.start(plugin.id);
    } catch (e) {
      error = String(e);
    }
  }

  async function copy() {
    const text = shown.map((e) => `${e.at} ${e.kind} ${e.text}`).join('\n');
    try {
      await navigator.clipboard.writeText(text);
      app.toast(t('plugins.journal.copied'), 'ok');
    } catch (e) {
      app.toast(String(e), 'error');
    }
  }

  const clock = (at: string) => {
    const d = new Date(at);
    return Number.isNaN(d.getTime()) ? at : d.toLocaleTimeString(undefined, { hour12: false });
  };

  onMount(() => {
    if (!programmed) return;
    void read();
    timer = setInterval(read, 2000);
  });
  onDestroy(() => clearInterval(timer));
</script>

{#if !programmed}
  <p class="hint">{t('plugins.journal.declarative')}</p>
{:else}
  <div class="bar">
    <span class="phase {run?.status.phase ?? 'idle'}"
      ><i aria-hidden="true"></i>{run ? phaseText(run.status, plugin) : '…'}{#if run?.status.pid}<span class="pid"
          >{t('plugins.journal.pid', { pid: String(run.status.pid) })}</span
        >{/if}{#if run && run.status.restarts > 0}<span class="pid">{t('plugins.journal.restarts', { count: run.status.restarts })}</span
        >{/if}</span
    >
    {#if run && (run.status.phase === 'idle' || run.status.phase === 'stopped')}
      <button class="btn" disabled={!plugin.choices.enabled || plugin.state.kind !== 'ready'} onclick={start}
        ><Icon name={run.status.phase === 'idle' ? 'play' : 'restart'} size={14} />{run.status.phase === 'idle'
          ? t('plugins.journal.start')
          : t('plugins.journal.restart')}</button
      >
    {/if}
    <span class="grow"></span>
    <label class="filter"
      ><span>{t('plugins.journal.filter')}</span>
      <select class="field" bind:value={kind}>
        <option value="">{t('plugins.journal.all')}</option>
        {#each KINDS as k (k)}<option value={k}>{t(`plugins.journal.kinds.${k}`)}</option>{/each}
      </select></label
    >
    <button class="btn" disabled={!shown.length} onclick={copy}><Icon name="copy" size={14} />{t('plugins.journal.copy')}</button>
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  <p class="live">{t('plugins.journal.live')}</p>
  {#if shown.length}
    <ol class="log" aria-label={t('plugins.page.tabs.journal')}>
      {#each shown as e, i (i)}
        <li class={e.kind}>
          <time datetime={e.at}>{clock(e.at)}</time><span class="k">{t(`plugins.journal.kinds.${e.kind}`)}</span><span class="tx"
            >{e.text}</span
          >
        </li>
      {/each}
    </ol>
  {:else}
    <p class="hint">{run?.journal.length ? t('plugins.journal.filteredEmpty') : t('plugins.journal.empty')}</p>
  {/if}
{/if}

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .grow {
    flex: 1;
  }
  .phase {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    font-weight: 560;
  }
  .phase i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--idle);
  }
  .phase.running i {
    background: var(--run);
  }
  .phase.starting i,
  .phase.restarting i {
    background: var(--wait);
  }
  .phase.stopped i {
    background: var(--del);
  }
  .pid {
    color: var(--muted);
    font-weight: 400;
    font-size: 11.5px;
  }
  .filter {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .filter select {
    height: 30px;
  }
  .live {
    margin: 8px 0 6px;
    color: var(--muted);
    font-size: 11px;
  }
  .log {
    margin: 0;
    padding: 8px 10px;
    list-style: none;
    border-radius: 9px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font: 11.5px/1.6 var(--mono);
    max-height: 420px;
    overflow: auto;
    user-select: text;
  }
  .log li {
    display: grid;
    grid-template-columns: 64px 108px minmax(0, 1fr);
    gap: 10px;
  }
  .log time {
    color: var(--muted);
  }
  .log .k {
    color: var(--dim);
  }
  .log .tx {
    color: var(--soft);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .log li.crash .k,
  .log li.refused .k,
  .log li.crash .tx {
    color: var(--del);
  }
  .log li.timeout .k,
  .log li.dropped .k,
  .log li.stderr .k {
    color: var(--warn-ink);
  }
  .log li.ready .k {
    color: var(--teal-ink);
  }
  .hint {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 8px 0 0;
    color: var(--del);
    font-size: 12px;
  }
</style>

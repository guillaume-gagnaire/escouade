<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { fAgo, fBytes, fCountdown, fDateTime, fPct } from '../lib/format';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { gitSync, syncInfo, type SyncOp } from '../lib/git-sync.svelte';
  import { api } from '../lib/ipc';
  import { estimateHint, fSpentUsd } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';

  const agents = $derived(Object.values(app.agents).filter((a) => !a.archived));
  const running = $derived(agents.filter((a) => a.status === 'running').length);
  const waiting = $derived(agents.filter((a) => a.status === 'waiting').length);
  const done = $derived(agents.filter((a) => a.status === 'done').length);
  // The Claude processes running, each with the tools and MCP servers it started.
  const procs = $derived(app.resources);
  const procsTitle = $derived(
    [
      t('shell.status.procsTitle'),
      ...[...procs.agents]
        .sort((a, b) => b.memory - a.memory)
        .map((r) => `${app.agents[r.id]?.name ?? '?'} : ${fBytes(r.memory)} · ${fPct(r.cpu)}`),
    ].join('\n'),
  );
  const five = $derived(app.usage.fiveHour);
  const week = $derived(app.usage.sevenDay);

  function toggleSound() {
    app.settings.sound = !app.settings.sound;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }

  /** Seconds left before the automatic restart for the update, when one is planned. */
  const restartIn = $derived(app.restartAt === null ? null : Math.max(0, Math.ceil((app.restartAt - app.now) / 1000)));

  function postpone() {
    app.restartAt = null;
    app.run(api.updatePostpone());
  }

  /** What the branch button says while a sync runs. */
  const runningLabel = (op: SyncOp) =>
    ({ pull: t('branches.sync.pulling'), push: t('branches.sync.pushing'), fetch: t('branches.sync.fetching') })[op];

  /** The active project's checkout, when it is a repository with a branch (or a detached HEAD) checked out. */
  const repo = $derived.by(() => {
    const p = app.ui.view === 'project' ? app.project : null;
    const g = p ? app.git[p.id] : undefined;
    return p && g?.isRepo && g.branch ? { projectId: p.id, git: g, detached: g.branch === '(detached)' } : null;
  });
  /** What it can sync with, when it has a remote. */
  const sync = $derived(repo ? syncInfo(repo.git) : null);
  const busy = $derived(repo ? gitSync.running[repo.projectId] : undefined);

  /** The branch picker, open over the button. */
  let pickerOpen = $state(false);
  let branchButton = $state<HTMLButtonElement>();
  // Nothing to pick a branch of any more (another view, another folder): the picker goes.
  $effect(() => {
    if (!repo) pickerOpen = false;
  });

  const branchTitle = $derived.by(() => {
    if (!sync) return t('branches.picker.switchTitle');
    const fetched = sync.lastFetch ? fAgo(sync.lastFetch / 1000, app.now) : t('shell.status.sync.never');
    return [
      sync.tracked
        ? t('shell.status.sync.tracked', { upstream: sync.upstream ?? '', behind: sync.behind, ahead: sync.ahead })
        : sync.upstream
          ? t('shell.status.sync.gone', { upstream: sync.upstream })
          : t('shell.status.sync.unpublished'),
      t('shell.status.sync.lastFetch', { when: fetched }),
    ].join('\n');
  });
</script>

<footer class="bar mono">
  <span class="it"
    ><span class="dot" style="width:7px;height:7px;background:var(--ok)"></span>{t('shell.status.active', { count: running })}</span
  >
  <button
    class="it link"
    style:color={waiting ? 'var(--wait)' : 'var(--muted)'}
    onclick={() => app.nextWaiting()}
    title={t('shell.status.nextWaiting', { key: keyLabel('Ctrl+J') })}
  >
    {#if waiting}<span class="pulse" style="width:7px;height:7px"></span>{:else}<span
        class="dot"
        style="width:7px;height:7px;background:var(--dim)"
      ></span>{/if}
    {t('shell.status.waiting', { n: waiting })}
  </button>
  <span class="it"><span style="color:var(--ok)">✓</span>{t('shell.status.done', { count: done })}</span>
  {#if procs.instances}
    <span class="vsep"></span>
    <span class="it" title={procsTitle}
      ><Rich k="shell.status.procs" instances={procs.instances}
        >{#snippet memory()}<span class="v">{fBytes(procs.memory)}</span>{/snippet}{#snippet cpu()}<span class="v">{fPct(procs.cpu)}</span
          >{/snippet}</Rich
      ></span
    >
  {/if}
  <span class="vsep"></span>
  <span
    class="it"
    title={five?.resetsAt ? t('shell.status.resetsAt', { date: fDateTime(five.resetsAt) }) : t('shell.status.sessionUnavailable')}
  >
    {t('shell.status.session')}
    <span class="meter"
      ><span style:width="{Math.min(100, five?.pct ?? 0)}%" style:background={(five?.pct ?? 0) > 80 ? 'var(--wait)' : 'var(--accent)'}
      ></span></span
    >
    <span class="v">{five ? fPct(five.pct) : '—'}</span>
    {#if five?.resetsAt}<span class="d">{t('shell.status.reset', { countdown: fCountdown(five.resetsAt, app.now) })}</span>{/if}
  </span>
  <span
    class="it"
    title={week?.resetsAt ? t('shell.status.resetsAt', { date: fDateTime(week.resetsAt) }) : t('shell.status.weekUnavailable')}
  >
    {t('shell.status.week')}
    <span class="meter"
      ><span style:width="{Math.min(100, week?.pct ?? 0)}%" style:background={(week?.pct ?? 0) > 80 ? 'var(--wait)' : 'var(--accent)'}
      ></span></span
    >
    <span class="v">{week ? fPct(week.pct) : '—'}</span>
    {#if week?.resetsAt}<span class="d">{t('shell.status.reset', { countdown: fCountdown(week.resetsAt, app.now) })}</span>{/if}
  </span>
  <span class="vsep"></span>
  <span class="it" title={app.liveCost > 0 ? estimateHint() : undefined}
    >{t('shell.status.today')}
    <span class="v strong">{fSpentUsd({ cost: app.usage.todayCost + app.liveCost, estimated: app.liveCost > 0 })}</span></span
  >
  {#if repo}
    <span class="vsep"></span>
    <button
      class="it link sync"
      bind:this={branchButton}
      disabled={!!busy}
      aria-haspopup="dialog"
      aria-expanded={pickerOpen}
      onclick={() => (pickerOpen = !pickerOpen)}
      title={branchTitle}
    >
      <span class="v">⎇ {repo.detached ? t('branches.picker.detached') : repo.git.branch}</span>
      {#if busy}
        <span>{runningLabel(busy)}</span>
      {:else if sync?.tracked}
        <span style:color={sync.ahead ? 'var(--text)' : 'var(--dim)'}>↑{sync.ahead}</span>
        <span style:color={sync.behind ? 'var(--wait)' : 'var(--dim)'}>↓{sync.behind}</span>
      {:else if sync}
        <span class="d">{sync.upstream ? t('shell.status.sync.goneShort') : t('shell.status.sync.unpublishedShort')}</span>
      {/if}
    </button>
  {/if}
  <div style="flex:1"></div>
  {#if restartIn !== null}
    <span class="it v">{t('shell.status.restartIn', { seconds: restartIn })}</span>
    <button class="small" onclick={postpone}>{t('common.later')}</button>
  {:else if app.update?.ready}
    <button class="upd" onclick={() => (app.modal = { kind: 'update' })}
      >{t('shell.status.updateReady', { version: app.update.version })}</button
    >
  {:else if app.update}
    <span class="it">{t('shell.status.updating', { version: app.update.version })}</span>
  {/if}
  <button class="small" onclick={toggleSound} title={t('shell.status.sound')}
    >♪ {app.settings.sound ? t('shell.status.soundOn') : t('shell.status.soundOff')}</button
  >
  <button
    class="small"
    onclick={() => {
      app.modal = { kind: 'settings' };
    }}
    title={t('shell.status.settings', { key: keyLabel('Ctrl+,') })}>⚙</button
  >
</footer>

{#if pickerOpen && repo}
  {#key repo.projectId}
    <BranchPicker projectId={repo.projectId} anchor={branchButton} onclose={() => (pickerOpen = false)} />
  {/key}
{/if}

<style>
  .bar {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 8px 0 16px;
    background: var(--panel);
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--muted);
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
  }
  .it {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .link {
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
    padding: 0;
  }
  .sync:hover:not(:disabled) .v {
    text-decoration: underline;
  }
  .sync:disabled {
    cursor: default;
  }
  .vsep {
    width: 1px;
    height: 14px;
    background: var(--line2);
  }
  .meter {
    width: 48px;
    height: 5px;
    border-radius: 3px;
    background: var(--elev2);
    overflow: hidden;
    margin-left: 2px;
  }
  .meter span {
    display: block;
    height: 100%;
    transition: width 0.4s;
  }
  .v {
    color: var(--text);
  }
  .strong {
    font-weight: 600;
  }
  .d {
    color: var(--dim);
  }
  .small {
    height: 22px;
    padding: 0 8px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .small:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .upd {
    height: 22px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
</style>

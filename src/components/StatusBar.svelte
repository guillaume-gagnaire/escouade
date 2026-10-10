<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { accountName, newAgentAccount, quotaName, quotaTip } from '../lib/accounts';
  import { fAgo, fBytes, fPct } from '../lib/format';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { gitSync, syncInfo, type SyncOp } from '../lib/git-sync.svelte';
  import { api } from '../lib/ipc';
  import { estimateHint, fSpentUsd } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import AccountsPanel from './AccountsPanel.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';
  import QuotaMeter from './QuotaMeter.svelte';

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
        .map((r) => t('shell.status.procRow', { name: app.agents[r.id]?.name ?? '?', memory: fBytes(r.memory), cpu: fPct(r.cpu) })),
    ].join('\n'),
  );
  // The quota of the current account: with several, the name in front, and the whole group opens the panel of every account's.
  // (The bar is there before the settings are: no accounts yet.)
  const several = $derived((app.settings.accounts?.length ?? 0) > 1);
  const current = $derived(several ? newAgentAccount(undefined) : undefined);
  const currentName = $derived(current ? accountName(current) : '');
  let panelOpen = $state(false);
  // Down to one account: nothing left to compare.
  $effect(() => {
    if (!several) panelOpen = false;
  });
  // The two windows as their bars say them. Under the button the bars are not reached one by one, so the button says them
  // (to a screen reader as its description, `aria-describedby` rather than `aria-description`, which not every engine reads;
  // to the keyboard as a tooltip when it gets the focus).
  const quotaLines = $derived([
    t('accounts.quota.line', { name: quotaName('fiveHour'), tip: quotaTip(app.usage.fiveHour) }),
    t('accounts.quota.line', { name: quotaName('sevenDay'), tip: quotaTip(app.usage.sevenDay) }),
  ]);
  let quotaTipShown = $state(false);
  let quotaTipSpot = $state({ left: 8, bottom: 38 });
  /** A pointer pressed the button: its focus is no keyboard's, and the click opens the panel, not a tooltip. */
  let pointerOnGroup = false;

  function showQuotaTip(button: HTMLElement) {
    if (pointerOnGroup || panelOpen) return;
    const r = button.getBoundingClientRect();
    quotaTipSpot = { left: Math.max(8, r.left), bottom: Math.max(8, window.innerHeight - r.top + 8) };
    quotaTipShown = true;
  }

  function onGroupKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !quotaTipShown) return;
    // Away without the focus moving (the panel is not what Escape closes here, nor the composer's turn).
    e.stopPropagation();
    quotaTipShown = false;
  }

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
  let quotaButton = $state<HTMLButtonElement>();
  // The button is disabled while a sync runs, and a browser takes the focus off a control that becomes disabled: the
  // sync started from the picker, which gave the focus to the button, so the button gets it back when the sync ends (unless
  // the user has put it somewhere since).
  let wasBusy = false;
  $effect(() => {
    const now = !!busy;
    const lost = !document.activeElement || document.activeElement === document.body;
    if (wasBusy && !now && lost) branchButton?.focus();
    wasBusy = now;
  });
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

{#snippet quotas(focusable: boolean)}
  {#if several}<span class="v acct" title={currentName}>{currentName}</span><span class="d">·</span>{/if}
  <QuotaMeter kind="fiveHour" usage={app.usage.fiveHour} now={app.now} {focusable} />
  <span class="vsep"></span>
  <QuotaMeter kind="sevenDay" usage={app.usage.sevenDay} now={app.now} {focusable} />
{/snippet}

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
  {#if several}
    <!-- The panel's button: its bars are only seen here (their values are in the panel, which the keyboard opens). -->
    <button
      class="it link group"
      bind:this={quotaButton}
      aria-haspopup="dialog"
      aria-expanded={panelOpen}
      aria-controls="accounts-panel"
      aria-label={t('accounts.panel.open', { name: currentName })}
      aria-describedby="quota-description"
      onpointerdown={() => (pointerOnGroup = true)}
      onfocus={(e) => showQuotaTip(e.currentTarget)}
      onblur={() => {
        quotaTipShown = false;
        pointerOnGroup = false;
      }}
      onkeydown={onGroupKeydown}
      onclick={() => {
        quotaTipShown = false;
        panelOpen = !panelOpen;
      }}
    >
      {@render quotas(false)}
    </button>
    <span id="quota-description" hidden>{quotaLines.join('. ')}</span>
  {:else}
    <span class="it group">{@render quotas(true)}</span>
  {/if}
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

{#if quotaTipShown && !panelOpen}
  <div class="tip" role="tooltip" style:left="{quotaTipSpot.left}px" style:bottom="{quotaTipSpot.bottom}px">
    {#each quotaLines as line (line)}<div>{line}</div>{/each}
  </div>
{/if}

{#if panelOpen}
  <AccountsPanel id="accounts-panel" anchor={quotaButton} onclose={() => (panelOpen = false)} />
{/if}

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
  .group {
    gap: 8px;
  }
  button.group {
    color: inherit;
    height: 22px;
    padding: 0 6px;
    margin: 0 -6px;
    border-radius: var(--r-sm);
  }
  /* Lighter than the bars' track, which would otherwise melt into it. */
  button.group:hover {
    background: var(--elev);
  }
  .v {
    color: var(--text);
  }
  /* A long name is cut short (it is whole in its title), not the bars after it. */
  .acct {
    max-width: 160px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tip {
    position: fixed;
    z-index: 60;
    padding: 4px 8px;
    background: var(--elev);
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
    color: var(--text);
    font-size: 11.5px;
    line-height: 1.4;
    white-space: nowrap;
    pointer-events: none;
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

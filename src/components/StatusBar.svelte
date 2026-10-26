<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import {
    accountName,
    engineOrder,
    metersShown,
    meterName,
    newAgentAccount,
    principalId,
    projectedDayCost,
    projectionInBar,
    projectionLevel,
    projectionMode,
    projectionTip,
    quotaTip,
    statusEngine,
  } from '../lib/accounts';
  import { capsOf, engineName, usagePage } from '../lib/engines';
  import { fAgo, fBytes, fPct, fUsd } from '../lib/format';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { gitSync, syncInfo, type SyncOp } from '../lib/git-sync.svelte';
  import { api } from '../lib/ipc';
  import { estimatedBy, estimateHint, fSpentUsd } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import type { EngineId } from '../lib/types';
  import AccountsPanel from './AccountsPanel.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';
  import { branchSummary, isMulti, repoLabel } from '../lib/repos';
  import { menu } from '../lib/menu.svelte';
  import Icon from './Icon.svelte';
  import QuotaMeter from './QuotaMeter.svelte';
  import { tip } from '../lib/tip';

  // The status bar of the 2.0 design: the agents at work, waiting (a button, Ctrl+J) and finished, their processes; then
  // the update, a group of quota gauges per harness at work (a click opens every account's), the day's cost, the branch
  // and the sound of the notifications. The settings are at the bottom of the projects' rail (ESC-158).

  const agents = $derived(Object.values(app.agents).filter((a) => !a.archived));
  // The day's cost: an estimate while a turn runs, or once a harness that tells none (Codex) spent some of it.
  const today = $derived.by(() => {
    const byHarness = app.usage.accounts.some((u) => u.todayCost > 0 && estimatedBy(u.engine));
    return { cost: app.usage.todayCost + app.liveCost, estimated: byHarness || app.liveCost > 0, byHarness };
  });
  const running = $derived(agents.filter((a) => a.status === 'running').length);
  const waiting = $derived(agents.filter((a) => a.status === 'waiting').length);
  const done = $derived(agents.filter((a) => a.status === 'done').length);
  // The agents' processes running, each with the tools and MCP servers it started.
  const procs = $derived(app.resources);
  const procsTitle = $derived(
    [
      t('shell.status.procsTitle'),
      ...[...procs.agents]
        .sort((a, b) => b.memory - a.memory)
        .map((r) => t('shell.status.procRow', { name: app.agents[r.id]?.name ?? '?', memory: fBytes(r.memory), cpu: fPct(r.cpu) })),
    ].join('\n'),
  );

  const several = $derived((app.settings.accounts?.length ?? 0) > 1);

  /**
   * One group of gauges per harness at work (lot 0 of the 2.0 design): the harness of the agent shown (else the project's
   * default one), then each other harness that has agents, in « Ordre des harness ». Each shows the account new agents of
   * that harness go to (every account reserved to projects, Principal too: `usage.current`). (The bar is there before the
   * settings are: no accounts yet.)
   */
  const groups = $derived.by(() => {
    const main = statusEngine();
    const used = new Set(agents.map((a) => a.engine ?? 'claude'));
    const engines = [main, ...engineOrder().filter((e) => e !== main && used.has(e))];
    return engines.map(groupOf).filter((g) => g.meters.length || g.day);
  });

  function groupOf(engine: EngineId) {
    const shown = newAgentAccount(undefined, engine) ?? app.settings.accounts?.find((a) => a.id === app.usage.current[engine]);
    const usage = app.usage.accounts.find((u) => u.id === (shown?.id ?? principalId(engine))) ?? null;
    const meters = metersShown(usage, engine);
    const harness = engineName(engine);
    // A harness without a quota (OpenCode, which goes through the providers' APIs): its account's cost of the day stands
    // in for the bars, out of its « Budget par jour ($) » when it has one (choice 23).
    let day: {
      cost: number;
      budget: number | null;
      over: boolean;
      tip: string;
      projection: { text: string; level: string } | null;
    } | null = null;
    // Cursor CLI tells no cost either: nothing stands in for its bars (its usage is on its dashboard, ESC-43).
    if (capsOf(engine)?.quota === 'none' && !usagePage(engine) && usage) {
      const cost = usage.todayCost;
      const budget = usage.dailyBudget || null;
      const dayTip = budget
        ? t('shell.status.dayBudgetTip', { harness, cost: fUsd(cost), budget: fUsd(budget) })
        : t('shell.status.dayCostTip', { harness, cost: fUsd(cost) });
      // Where the day's cost would end at midnight, against the budget: in the bar as the quotas' projections are
      // (`projectionInBar`), in its tooltip always.
      const end = budget && projectionMode() !== 'off' ? projectedDayCost(cost, app.now) : null;
      const level = end === null ? null : projectionLevel((end / budget!) * 100);
      day = {
        cost,
        budget,
        over: !!budget && cost >= budget,
        tip: end === null ? dayTip : `${dayTip} ${t('shell.status.dayProjectedTip', { amount: fUsd(end) })}`,
        projection: end === null || !projectionInBar(level!) ? null : { text: fUsd(end), level: level! },
      };
    }
    // The accounts of the harness whose agents go on answering as extra usage, billed apart (choice 22).
    const inOverage = app.usage.accounts.filter((u) => u.overage && u.engine === engine);
    const overage = inOverage.length
      ? t(
          inOverage.some((u) => u.id === app.usage.current[engine])
            ? 'accounts.quota.overageCurrent'
            : (app.settings.switchAt ?? 100) < 100
              ? 'accounts.quota.overageMoving'
              : 'accounts.quota.overage',
          {
            names: inOverage
              .map((u) => {
                const a = app.settings.accounts?.find((x) => x.id === u.id);
                return a ? accountName(a) : u.id;
              })
              .join(', '),
          },
        )
      : '';
    // With several accounts, the account shown is named where the group is said (its tooltip, a screen reader).
    const name = several && shown ? t('shell.status.harnessAccount', { harness, account: accountName(shown) }) : harness;
    const lines = [
      ...(overage ? [overage] : []),
      ...meters.map((m) =>
        t('accounts.quota.line', { name: meterName(m.minutes), tip: quotaTip(m.usage) + projectionTip(m.minutes, m.usage, app.now) }),
      ),
      ...(day ? [day.tip] : []),
    ];
    return { engine, short: harness.split(' ')[0], name, account: shown, meters, day, overage: !!overage, lines };
  }

  /** What the quota button says (its tooltip, and its description to a screen reader): harness by harness when they are several. */
  const quotaLines = $derived(groups.length > 1 ? groups.flatMap((g) => [g.name, ...g.lines]) : (groups[0]?.lines ?? []));
  let panelOpen = $state(false);

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
    // Of several repositories (ESC-169): shown even when they are not on one branch.
    return p && g?.isRepo && (g.branch || isMulti(p))
      ? { projectId: p.id, git: g, detached: g.branch === '(detached)', multi: isMulti(p) }
      : null;
  });
  /** Of several repositories: « main · 4 dépôts », or « 3 branches · 4 dépôts ». */
  const summary = $derived(repo?.multi ? branchSummary(repo.git) : null);
  /** The repository whose branch picker is open, of several. */
  let pickerRepo = $state<string | null>(null);
  /** Of several repositories: their menu first, each opening its own branch picker. */
  function openBranches() {
    if (!repo?.multi) {
      pickerRepo = null;
      pickerOpen = !pickerOpen;
      return;
    }
    if (!branchButton) return;
    const repos = repo.git.repos ?? [];
    const projectId = repo.projectId;
    menu.showAt(branchButton, [
      { label: t('repos.status.menuTitle', { count: repos.length }), heading: true },
      ...repos.map((r) => {
        const why = r.gone
          ? t('repos.status.gone')
          : !r.hasCommit
            ? t('repos.status.noCommit')
            : r.branch === '(detached)'
              ? t('repos.status.detached')
              : undefined;
        const ab =
          r.ahead || r.behind
            ? t('repos.status.sync', { branch: r.branch, behind: r.behind, ahead: r.ahead })
            : `${r.branch} · ${t('repos.status.upToDate')}`;
        return {
          label: repoLabel(r.repo),
          icon: 'branch' as const,
          detail: why ? undefined : ab,
          reason: why,
          disabled: !!r.gone,
          onClick: () => {
            pickerRepo = r.repo;
            pickerOpen = true;
          },
        };
      }),
      { label: '', separator: true },
      {
        label: t('repos.status.rescan'),
        icon: 'restart' as const,
        detail: t('repos.status.rescanHint'),
        onClick: () => void app.run(api.refreshRepos(projectId)),
      },
    ]);
  }
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
  // No gauge left to open the panel from.
  $effect(() => {
    if (!groups.length) panelOpen = false;
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
  /** A shortcut as the bar writes it: « Ctrl J ». */
  const kbd = (s: string) => keyLabel(s).split('+').join(' ');
</script>

<footer class="bar">
  <!-- The first start, no project yet: empty, as the rest of the shell (choice 49 of the 2.0 design). -->
  {#if app.projects.length}
    <!-- What the bar tells, which gives way when the window is narrow. -->
    <div class="main">
      <span class="it"><span class="dot run"></span><span><b>{running}</b> {t('shell.status.activeWord', { count: running })}</span></span>
      <button
        class="it link wait"
        class:none={!waiting}
        onclick={() => app.nextWaiting()}
        use:tip={t('shell.status.nextWaiting', { key: keyLabel('Ctrl+J') })}
      >
        {#if waiting}<span class="pulse"></span>{:else}<span class="dot idle"></span>{/if}
        <span><b>{waiting}</b> {t('shell.status.waitingWord')}</span>
        <kbd>{kbd('Ctrl+J')}</kbd>
      </button>
      <span class="it"><Icon name="check" size={14} /><span><b>{done}</b> {t('shell.status.doneWord', { count: done })}</span></span>
      {#if procs.instances}
        <span class="vsep"></span>
        <span class="it procs" use:tip={procsTitle}
          ><Rich k="shell.status.procs" count={procs.instances}
            >{#snippet memory()}<b>{fBytes(procs.memory)}</b>{/snippet}{#snippet cpu()}<b>{fPct(procs.cpu)}</b>{/snippet}</Rich
          ></span
        >
      {/if}
    </div>
    <div class="end">
      {#if restartIn !== null}
        <span class="it"><b>{t('shell.status.restartIn', { seconds: restartIn })}</b></span>
        <button class="small" onclick={postpone}>{t('common.later')}</button>
      {:else if app.update?.ready}
        <button class="it link upd" onclick={() => (app.modal = { kind: 'update' })}
          ><Icon name="download" size={14} /><Rich k="shell.status.updateReady" version={app.update.version}
            >{#snippet restart()}<b>{t('shell.status.restart')}</b>{/snippet}</Rich
          ></button
        >
      {:else if app.update}
        <span class="it">{t('shell.status.updating', { version: app.update.version })}</span>
      {/if}
      {#if groups.length}
        <!-- The gauges, harness by harness, are one button: it opens every account's (the panel writes their values). -->
        <button
          class="it link quotas"
          bind:this={quotaButton}
          aria-haspopup="dialog"
          aria-expanded={panelOpen}
          aria-controls="accounts-panel"
          aria-label={several && groups[0]?.account
            ? t('accounts.panel.open', { name: accountName(groups[0].account) })
            : t('accounts.panel.label')}
          aria-describedby="quota-description"
          use:tip={panelOpen ? null : { text: quotaLines.join('\n'), aria: false }}
          onclick={() => (panelOpen = !panelOpen)}
        >
          {#each groups as g (g.engine)}
            <span class="grp" data-engine={g.engine}>
              <span class="hn">{g.short}</span>
              {#each g.meters as m (m.minutes ?? 'credits')}
                <QuotaMeter minutes={m.minutes} usage={m.usage} now={app.now} focusable={false} />
              {/each}
              {#if g.day}
                <span class="day" class:over={g.day.over}
                  ><b>{fUsd(g.day.cost)}</b>{#if g.day.budget}<span class="of">{` / ${fUsd(g.day.budget)}`}</span
                    >{/if}{#if g.day.projection}<span class="p {g.day.projection.level}"
                      ><span class="sr">{t('accounts.quota.projectedSr')}</span>{t('accounts.quota.projectedCost', {
                        amount: g.day.projection.text,
                      })}</span
                    >{/if}</span
                >
              {/if}
              {#if g.overage}<span class="over-word">{t('shell.status.overage')}</span>{/if}
            </span>
          {/each}
        </button>
        <span id="quota-description" hidden>{quotaLines.join('. ')}</span>
      {/if}
      <span class="it" use:tip={today.estimated ? estimateHint(today) : undefined}>{t('shell.status.today')} <b>{fSpentUsd(today)}</b></span
      >
      {#if repo}
        <span class="vsep"></span>
        <button
          class="it link sync branch"
          bind:this={branchButton}
          disabled={!!busy}
          aria-haspopup="dialog"
          aria-expanded={pickerOpen}
          onclick={openBranches}
          use:tip={summary ? summary.text : branchTitle}
        >
          <Icon name="branch" size={14} />
          {#if summary}
            {#if summary.branch}<b class="name mono">{summary.branch}</b>{/if}<span class="d repos"
              >{summary.branch ? '· ' : ''}{summary.text}</span
            >
            {#if repo.git.ahead || repo.git.behind}<span class="ahead some"
                ><Icon name="up" size={12} /><span aria-hidden="true">{repo.git.ahead}</span><span class="sr"
                  >{t('shell.status.sync.aheadShort', { count: repo.git.ahead })}</span
                ></span
              ><span class="ahead some"
                ><Icon name="pull" size={12} /><span aria-hidden="true">{repo.git.behind}</span><span class="sr"
                  >{t('shell.status.sync.behindShort', { count: repo.git.behind })}</span
                ></span
              >{/if}
          {:else}
            <b class="name mono">{repo.detached ? t('branches.picker.detached') : repo.git.branch}</b>
          {/if}
          {#if summary}<!-- each repository's state is in its menu -->{:else if busy}
            <span>{runningLabel(busy)}</span>
          {:else if sync?.tracked}
            <!-- The commits to push, then to pull, as in the picker: an arrow and a number, said in words to a screen reader. -->
            <span class="ahead" class:some={sync.ahead > 0}
              ><Icon name="up" size={12} /><span aria-hidden="true">{sync.ahead}</span><span class="sr"
                >{t('shell.status.sync.aheadShort', { count: sync.ahead })}</span
              ></span
            >
            <span class="ahead" class:some={sync.behind > 0}
              ><Icon name="pull" size={12} /><span aria-hidden="true">{sync.behind}</span><span class="sr"
                >{t('shell.status.sync.behindShort', { count: sync.behind })}</span
              ></span
            >
          {:else if sync}
            <span class="d">{sync.upstream ? t('shell.status.sync.goneShort') : t('shell.status.sync.unpublishedShort')}</span>
          {/if}
        </button>
      {/if}
      <button
        class="icon"
        aria-pressed={!!app.settings.sound}
        aria-label={t('shell.status.sound')}
        use:tip={t('shell.status.soundTip', { state: app.settings.sound ? t('shell.status.soundOn') : t('shell.status.soundOff') })}
        onclick={toggleSound}><Icon name={app.settings.sound ? 'sound' : 'mute'} size={14} /></button
      >
    </div>
  {/if}
</footer>

{#if panelOpen}
  <AccountsPanel id="accounts-panel" anchor={quotaButton} onclose={() => (panelOpen = false)} />
{/if}

{#if pickerOpen && repo}
  {#key repo.projectId + '|' + (pickerRepo ?? '')}
    <BranchPicker projectId={repo.projectId} repo={pickerRepo} anchor={branchButton} onclose={() => (pickerOpen = false)} />
  {/key}
{/if}

<style>
  .bar {
    grid-column: 1 / -1;
    grid-row: 3;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 0 10px 0 14px;
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
    container: status / inline-size;
  }
  /* Past its width, what it tells is cut at its edge rather than pushing the rest out of the window. Its height is the
     bar's, so a focus ring is not cut. */
  .main {
    flex: 1;
    min-width: 0;
    align-self: stretch;
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
  }
  .main > * {
    flex: none;
  }
  /* The processes give way first, cut short (their tooltip says them whole). */
  .main > .procs {
    display: block;
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* What it keeps whole; only the branch gives way, its name cut short (the picker it opens names it). */
  .end {
    flex: 0 1 auto;
    min-width: 0;
    align-self: stretch;
    display: flex;
    align-items: center;
    gap: 18px;
  }
  .end > * {
    flex: none;
  }
  .end > .branch {
    flex: 0 1 auto;
    min-width: 6em;
    overflow: hidden;
  }
  .it {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  b {
    color: var(--soft);
    font-weight: 550;
  }
  .dot {
    width: 6px;
    height: 6px;
  }
  .dot.run {
    background: var(--run);
  }
  .dot.idle {
    background: var(--idle);
  }
  .pulse {
    width: 6px;
    height: 6px;
  }
  .link {
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
    color: inherit;
    height: 22px;
    padding: 0 7px;
    margin: 0 -7px;
    border-radius: 6px;
  }
  .link:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .wait {
    color: var(--wait);
  }
  .wait b {
    color: var(--wait);
  }
  .wait.none {
    color: var(--muted);
  }
  .wait.none b {
    color: var(--soft);
  }
  .quotas {
    gap: 16px;
    margin: 0 -3px;
    padding: 0 3px;
  }
  .grp {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .hn {
    color: var(--soft);
    font-weight: 550;
  }
  .day.over b {
    color: var(--wait);
  }
  /* Extra usage, billed apart: an amber word beside its harness (choice 22). */
  .over-word {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 5px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--warn-ink);
    font-size: 11px;
    font-weight: 560;
  }
  .p {
    margin-left: 4px;
    font-weight: 550;
  }
  .p.warn {
    color: var(--wait);
  }
  .p.over {
    color: var(--del);
  }
  /* Read by screen readers only. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .upd {
    color: var(--p-ink);
  }
  .upd b {
    color: var(--text);
  }
  .branch {
    min-width: 6em;
  }
  /* A long name is cut short: the picker it opens names it whole. */
  .branch .name {
    min-width: 0;
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11.5px;
  }
  .ahead {
    display: inline-flex;
    align-items: center;
    color: var(--dim);
  }
  .ahead.some {
    color: var(--soft);
  }
  .sync:disabled {
    cursor: default;
  }
  .d {
    color: var(--dim);
  }
  .vsep {
    width: 1px;
    height: 12px;
    background: var(--line-2);
  }
  .small,
  .icon {
    height: 22px;
    padding: 0 7px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 24px;
    padding: 0;
  }
  .small:hover,
  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }
  /* A narrow window (under 1240 pixels): the gauges keep their harness and lose their percentages, projections and the
     word of extra usage, which the panel writes (lot 0 and lot 0 bis of the 2.0 design). */
  @container status (max-width: 1239px) {
    .bar,
    .end {
      gap: 14px;
    }
    .quotas :global(.q .pct),
    .quotas :global(.q .p),
    .quotas .p,
    .over-word {
      display: none;
    }
  }
</style>

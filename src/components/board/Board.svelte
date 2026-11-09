<script lang="ts">
  import { addAccount, defaultEngine, hasAccount, pausedAccounts, pauseOf, quotaOf } from '../../lib/accounts';
  import { busy, COLUMNS, columnTickets, pauseLabel, placesLabel, settingsSummary } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { hasPending } from '../../lib/shared-file';
  import { shownOnBoard } from '../../lib/sharing';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import { ciShown, redTargetHolds, targetCiOf } from '../../lib/branch-ci';
  import { fAgo, fSince } from '../../lib/format';
  import CiPill from '../CiPill.svelte';
  import Icon from '../Icon.svelte';
  import BoardColumn from './BoardColumn.svelte';
  import EnvironmentsView from './EnvironmentsView.svelte';
  import { tip } from '../../lib/tip';
  import { prColumnShown } from '../../lib/ticket-prs';
  import { hasTab } from '../../lib/code-host';
  import { arrange, notInProduction } from '../../lib/environments';
  import { envsView } from '../../lib/envs-view.svelte';
  import EnvsMenu from './EnvsMenu.svelte';

  let { project: given }: { project: Project } = $props();

  // The project as the app holds it now (its board settings change under this view).
  const project = $derived(app.projects.find((p) => p.id === given.id) ?? given);
  const s = $derived(project.board);
  const all = $derived(Object.values(app.tickets).filter((x) => x.projectId === project.id));
  /** « Masquer chez moi » (ESC-190): another one's tickets taken off this board, shown back on demand. */
  let showHidden = $state(false);
  const hiddenCount = $derived(all.filter((x) => x.shared?.hidden).length);
  const tickets = $derived(shownOnBoard(all, showHidden));
  const busyCount = $derived(busy(tickets));
  /** « En revue » (ESC-59) only when the board opens pull requests, or while a ticket is in it. */
  const columns = $derived(COLUMNS.filter((c) => c.id !== 'pr' || prColumnShown(s, all)));
  const looping = $derived(tickets.filter((x) => x.column === 'doing').length);
  /** When the agents waiting for their quota resume, if every account this project could use waits for its own. */
  const quota = $derived(quotaOf(project));
  const target = $derived(s.target || app.git[project.id]?.branch || 'main');
  const sub = $derived(t('board.header.sub', { count: tickets.length, looping }));
  const summary = $derived(settingsSummary(s, target, project.ticketBranchPrefix));
  const places = $derived(placesLabel(busyCount, s.maxParallel, quota));
  /** Why no ticket starts from the target branch (no commit yet, or gone), as the backend says. */
  const issue = $derived(app.boardIssues[project.id] ?? null);
  /** Why no ticket of this project starts for now (a quota window, a usage limit), as the backend says. */
  const pause = $derived(pauseOf(project));
  const paused = $derived(pause ? pauseLabel(pause, app.now) : '');
  /** The accounts past the threshold, with several accounts. */
  const over = $derived(pause ? pausedAccounts(pause) : '');
  /** The project's harness has no account (ESC-128): « Ajouter un compte… » beside why nothing starts. */
  const engine = $derived(project.engine ?? defaultEngine());
  const noAccount = $derived(!hasAccount(engine));
  /** « Environnements » only for a project whose code host reads deployments (ESC-390). */
  const envsTab = $derived(hasTab(app.codeHosts[project.id], 'deployments'));
  /** The view « Environnements » in place of the tickets (ESC-67). */
  const envs = $derived(envsTab && !!app.envView[project.id]);
  const envData = $derived(app.environments[project.id]);
  /** The environments the view shows (ESC-390): its tab counts them, the tickets not in their production are said. */
  const shownEnvs = $derived(arrange(envData?.environments ?? [], envsView.of(project.id)).shown);
  const late = $derived(envs ? notInProduction(all, shownEnvs, app.now) : []);
  /** The CI of its target branch (ESC-63), as last read: the pill of the header; none without a host Escouade reads. */
  const targetCi = $derived(targetCiOf(project));
  /** Its pill: none for a target without any CI. */
  const targetPill = $derived(ciShown(targetCi) ? targetCi : null);
  /** « Pause quand la CI de la cible est rouge » holds the autopilot's tickets back now. */
  const redHold = $derived(s.autopilot && redTargetHolds(s, targetCi));

  /** « Escouade · GitHub · lu il y a 2 min », the line of the environments. */
  const envLine = $derived(
    t('envs.view.sub', { project: project.name, host: envData?.host || '—' }) +
      (envData?.readAt ? ` · ${t('envs.view.read', { since: fAgo(envData.readAt / 1000, app.now) })}` : ''),
  );

  async function toggleAutopilot() {
    // The project comes back through the backend's event, in order with any newer one: its answer is not written over it.
    await app.run(api.boardSet(project.id, { ...$state.snapshot(s), autopilot: !s.autopilot }));
  }
</script>

<main class="board">
  <!-- Two rows (direction A of design/kanban-envs-2-2, ESC-390): the title and what acts on the board; then its views
       and what the board says, each in a place of its own at every width. -->
  <header class="head">
    <div class="h1">
      <h1>{envs ? t('envs.view.title') : 'Kanban'}</h1>
      <span class="crumbs" use:tip={{ text: `${project.name} · ${sub}`, cut: true }}>{project.name} · <span class="num">{sub}</span></span>
      <div class="actions">
        <!-- Its label goes in a narrow window, its icon stays. -->
        <button
          class="btn imp"
          aria-label={t('common.import')}
          use:tip={t('board.header.importTitle')}
          onclick={() => (app.modal = { kind: 'import', projectId: project.id })}
          ><Icon name="import" size={14} /><span class="l">{t('common.import')}</span></button
        >
        <button
          class="auto"
          role="switch"
          aria-checked={s.autopilot}
          aria-label={t('board.header.autopilot')}
          use:tip={t('board.header.autopilotTitle')}
          onclick={toggleAutopilot}
        >
          <span class="l">{s.autopilot ? t('board.header.autopilot') : t('board.header.autopilotOff')}</span><span
            class="knob"
            aria-hidden="true"
          ></span>
        </button>
      </div>
    </div>
    <div class="h2">
      <div class="tabs" role="group" aria-label={t('envs.tabs.label')}>
        <button class:on={!envs} aria-pressed={!envs} aria-label={t('envs.tabs.tickets')} onclick={() => (app.envView[project.id] = false)}
          ><Icon name="kanban" size={14} />{t('envs.tabs.tickets')}<span class="c">{tickets.length}</span></button
        >
        {#if envsTab}
          <button
            class:on={envs}
            aria-pressed={envs}
            aria-label={t('envs.tabs.environments')}
            data-env-tab
            onclick={() => (app.envView[project.id] = true)}
            ><Icon name="globe" size={14} />{t('envs.tabs.environments')}{#if envData?.environments.length}<span class="c"
                >{shownEnvs.length}</span
              >{/if}</button
          >
        {/if}
      </div>
      <span class="sp"></span>
      {#if envs}
        <span class="said" use:tip={{ text: envLine, cut: true }}>{envLine}</span>
        {#if late.length}
          <span class="late" data-not-in-prod><Icon name="clock" size={14} />{t('envs.view.notInProd', { count: late.length })}</span>
        {/if}
        <i class="sep" aria-hidden="true"></i>
        <EnvsMenu {project} />
        <button
          class="icon-btn"
          aria-label={t('envs.view.refresh')}
          use:tip={t('envs.view.refreshTitle')}
          aria-busy={!!envsView.reading[project.id]}
          disabled={!!envsView.reading[project.id]}
          onclick={() => envsView.refresh(project.id)}><Icon name="restart" size={14} /></button
        >
      {:else}
        {#if targetPill}
          <!-- The CI of its target branch (ESC-63): « main ✗ depuis 2 h »; its jobs, their logs and their re-run on a click. -->
          <span class="target-ci"><CiPill ci={targetPill} of={{ kind: 'target', id: project.id }} projectId={project.id} /></span>
        {/if}
        {#if !app.enginesFound.length}
          <!-- No place is worth showing: nothing starts without a harness. -->
          <span class="places missing" use:tip={{ text: t('board.header.noEngine'), cut: true }}>{t('board.header.noEngine')}</span>
        {:else if issue}
          <!-- Nor while the target branch cannot start a ticket. -->
          <span class="places missing" use:tip={{ text: issue, cut: true }}>{issue}</span>
          {#if noAccount}<button class="btn add-account" onclick={() => addAccount(engine)}>{t('accounts.tab.add')}</button>{/if}
        {:else if pause || redHold}
          <!-- Nor while the autopilot is paused (a quota, a limit, a red target): the band under the header says why. -->
        {:else}
          <span class="places" use:tip={{ text: places, cut: true }}>{places}</span>
        {/if}
        <i class="sep" aria-hidden="true"></i>
        <!-- Its tooltip holds the summary in full: the summary is cut in a narrow window. Its name stays « Après
             validation : <résumé> » at every width, when « Après validation : » is no longer shown (ESC-393). -->
        <button
          class="cfg"
          aria-label="{t('board.header.afterApproval')} {summary}"
          use:tip={t('board.header.settingsTitle', { summary })}
          onclick={() => (app.modal = { kind: 'settings', page: 'board', projectId: project.id })}
        >
          <span class="k">{t('board.header.afterApproval')}</span><b class="v">{summary}</b><Icon name="down" size={12} />
        </button>
      {/if}
    </div>
  </header>
  {#if pause}
    <!-- A band of its own under the header: its text and its button would not fit in it. -->
    <div class="band pause" role="status">
      <Icon name="clock" size={14} />
      <span class="why" use:tip={{ text: paused, cut: true }}>{paused}</span>
      {#if over}<span class="over" use:tip={{ text: over, cut: true }}>{over}</span>{/if}
      <button class="btn" onclick={() => app.run(api.autopilotResume())}>{t('board.header.resumeNow')}</button>
    </div>
  {/if}
  {#if redHold && targetCi}
    <!-- « Pause quand la CI de la cible est rouge » (ESC-63): nothing starts by itself until it is green again. -->
    {@const why = t('board.ci.paused', { branch: targetCi.branch, since: targetCi.redSince ? fSince(targetCi.redSince, app.now) : '' })}
    <div class="band pause" role="status" data-red-target>
      <Icon name="pause" size={14} />
      <span class="why" use:tip={{ text: why, cut: true }}>{why}</span>
      <button class="btn" onclick={() => (app.modal = { kind: 'ci', of: { kind: 'target', id: project.id }, projectId: project.id })}
        >{t('board.ci.pausedShow')}</button
      >
    </div>
  {/if}
  {#if hasPending(project)}
    <!-- Its .escouade.yml brought commands that wait for the user: nothing of them runs until then (ESC-146, choice 29). -->
    <div class="band shared" role="status">
      <Icon name="shield" size={14} />
      <span class="why" use:tip={{ text: t('settings.shared.banner'), cut: true }}>{t('settings.shared.banner')}</span>
      <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'team', projectId: project.id, at: 'shared' })}
        >{t('settings.shared.bannerButton')}</button
      >
    </div>
  {/if}
  {#if hiddenCount}
    <div class="band hidden-band" role="status">
      <Icon name="team" size={14} />
      <span class="why">{t('share.board.hidden', { count: hiddenCount })}</span>
      <button class="btn" aria-pressed={showHidden} onclick={() => (showHidden = !showHidden)}
        >{showHidden ? t('share.board.hide') : t('share.board.show')}</button
      >
    </div>
  {/if}
  {#if envs}
    <EnvironmentsView {project} />
  {:else}
    <div class="cols" style:--n={columns.length}>
      {#each columns as c (c.id)}
        <BoardColumn column={c} tickets={columnTickets(tickets, c.id)} {project} {busyCount} {quota} />
      {/each}
    </div>
  {/if}
</main>

<style>
  .board {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /*
   * Two rows (direction A, ESC-390). The first: the title, its line, and what acts on the board (import, autopilot). The
   * second, under a line: the board's views, then what it says (CI, places) and « Après validation ». Each keeps its
   * place at every width: what is long is cut with its tooltip, what is said twice goes in a narrow window.
   */
  .board {
    container: board / inline-size;
  }
  .head {
    flex: none;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .h1 {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 52px;
    padding: 0 14px 0 22px;
  }
  h1 {
    flex: none;
    margin: 0;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .crumbs {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    font-size: 12px;
  }
  .actions {
    flex: none;
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .h2 {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 40px;
    padding: 0 14px 0 14px;
  }
  .sp {
    flex: 1 0 0;
  }
  /* « Tickets · Environnements » (ESC-67): the board's two views, underlined as the tabs of a page. */
  .tabs {
    flex: none;
    align-self: stretch;
    display: flex;
    gap: 2px;
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 8px;
    border: none;
    background: transparent;
    box-shadow: inset 0 -2px 0 transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
    transition:
      color 0.15s,
      box-shadow 0.15s;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--text);
  }
  .tabs button:focus-visible {
    outline: 2px solid var(--ring);
    outline-offset: -4px;
  }
  .tabs .c {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .target-ci {
    display: flex;
    flex: 0 1 auto;
    min-width: 0;
    max-width: 240px;
  }
  /* What the places say, on one line; the full text in its tooltip. */
  .places,
  .said {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 36ch;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    font-size: 12.5px;
  }
  .said {
    max-width: 44ch;
  }
  .places.missing {
    color: var(--del);
    font-weight: 550;
  }
  .late {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--wait);
    font-size: 12.5px;
    font-weight: 550;
  }
  .sep {
    flex: none;
    width: 1px;
    height: 16px;
    background: var(--line-2);
  }
  .add-account {
    flex: none;
  }
  .icon-btn {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .icon-btn:disabled {
    opacity: 0.5;
  }
  /* « Après validation : merge squash → main », the button of the Kanban's settings, quiet in the second row. */
  .cfg {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    flex: 0 1 auto;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition: background-color 0.15s;
  }
  .cfg:hover {
    background: var(--hover);
  }
  .cfg .k {
    flex: none;
  }
  .v {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    font-weight: 550;
  }
  /* « Pilote auto », a switch with its label. */
  .auto {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    height: 30px;
    padding: 0 4px 0 11px;
    border: none;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 13px;
    font-weight: 550;
    cursor: pointer;
  }
  .knob {
    position: relative;
    flex: none;
    width: 30px;
    height: 18px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.14);
    transition: background-color 0.2s;
  }
  .knob::after {
    content: '';
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, calc(0.4 * var(--shade)));
    transition: left 0.25s var(--ease);
  }
  .auto[aria-checked='true'] .knob {
    background: color-mix(in oklch, var(--p) 75%, black);
  }
  .auto[aria-checked='true'] .knob::after {
    left: 14px;
  }
  /* A narrower board: « Après validation : » goes (in its tooltip), the summary is cut. */
  @container board (max-width: 1000px) {
    .cfg .k {
      display: none;
    }
    .v {
      max-width: 24ch;
    }
    .said {
      max-width: 26ch;
    }
  }
  /* The narrowest: « Importer » keeps its icon, the views their names without their counts. */
  @container board (max-width: 860px) {
    .imp .l {
      display: none;
    }
    .imp {
      width: 30px;
      padding: 0;
      justify-content: center;
    }
    .tabs .c {
      display: none;
    }
    .places,
    .said {
      max-width: 18ch;
    }
    .v {
      max-width: 22ch;
    }
  }
  /* A band under the header, amber: what waits for the user (the autopilot paused, commands to read). */
  .band {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 22px 10px;
    padding: 8px 10px 8px 12px;
    border-radius: 10px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .band > :global(.icon) {
    color: var(--wait);
  }
  /* Hidden tickets of the team (ESC-190): said quietly, not a warning. */
  .band.hidden-band {
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .band.hidden-band > :global(.icon) {
    color: var(--muted);
  }
  .band .btn {
    flex: none;
    height: 28px;
    font-size: 12.5px;
  }
  /* Cut with an ellipsis in a narrow window, in full in its tooltip. */
  .why {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The accounts past the threshold keep their words, the line before gives way first. */
  .over {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  /* Columns without a box, side by side; under four times 220 pixels the board scrolls sideways, as the preview does. */
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(var(--n, 4), minmax(220px, 1fr));
    gap: 14px;
    padding: 16px 16px 16px 18px;
    overflow: auto;
  }
</style>

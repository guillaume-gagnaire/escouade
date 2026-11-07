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
  import { fSince } from '../../lib/format';
  import CiPill from '../CiPill.svelte';
  import Icon from '../Icon.svelte';
  import BoardColumn from './BoardColumn.svelte';
  import EnvironmentsView from './EnvironmentsView.svelte';
  import { tip } from '../../lib/tip';
  import { prColumnShown } from '../../lib/ticket-prs';

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
  /** The view « Environnements » in place of the tickets (ESC-67). */
  const envs = $derived(!!app.envView[project.id]);
  /** The CI of its target branch (ESC-63), as last read: the pill of the header; none without a host Escouade reads. */
  const targetCi = $derived(targetCiOf(project));
  /** Its pill: none for a target without any CI. */
  const targetPill = $derived(ciShown(targetCi) ? targetCi : null);
  /** « Pause quand la CI de la cible est rouge » holds the autopilot's tickets back now. */
  const redHold = $derived(s.autopilot && redTargetHolds(s, targetCi));

  async function toggleAutopilot() {
    // The project comes back through the backend's event, in order with any newer one: its answer is not written over it.
    await app.run(api.boardSet(project.id, { ...$state.snapshot(s), autopilot: !s.autopilot }));
  }
</script>

<main class="board">
  <header class="head">
    <div class="h-title">
      <h1>{envs ? t('envs.view.title') : 'Kanban'}</h1>
      <div class="crumbs" use:tip={{ text: `${project.name} · ${sub}`, cut: true }}>
        <span class="proj-name">{project.name}</span><span aria-hidden="true">·</span><span class="num sub">{sub}</span>
      </div>
      <!-- Beside the title and its line, in the room they leave: the controls keep their place (ESC-67). -->
      <div class="tabs" role="group" aria-label={t('envs.tabs.label')}>
        <button
          class:on={!envs}
          aria-pressed={!envs}
          aria-label={t('envs.tabs.tickets')}
          use:tip={t('envs.tabs.tickets')}
          onclick={() => (app.envView[project.id] = false)}
          ><Icon name="kanban" size={14} /><span class="tl">{t('envs.tabs.tickets')}</span></button
        >
        <button
          class:on={envs}
          aria-pressed={envs}
          aria-label={t('envs.tabs.environments')}
          use:tip={t('envs.tabs.environments')}
          data-env-tab
          onclick={() => (app.envView[project.id] = true)}
          ><Icon name="globe" size={14} /><span class="tl">{t('envs.tabs.environments')}</span></button
        >
      </div>
    </div>
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
    <div class="actions">
      <!-- Its label goes in a narrow window, its icon and its name stay. -->
      <button
        class="btn imp"
        aria-label={t('common.import')}
        use:tip={t('board.header.importTitle')}
        onclick={() => (app.modal = { kind: 'import', projectId: project.id })}
        ><Icon name="import" size={14} /><span class="l">{t('common.import')}</span></button
      >
      <!-- Its tooltip holds the summary in full: the summary is cut in a narrow window. -->
      <button
        class="cfg"
        use:tip={t('board.header.settingsTitle', { summary })}
        onclick={() => (app.modal = { kind: 'settings', tab: 'board', projectId: project.id })}
      >
        <span class="k">{t('board.header.afterApproval')}</span><b class="v">{summary}</b><Icon name="down" size={14} />
      </button>
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
      <button class="btn" onclick={() => (app.modal = { kind: 'settings', tab: 'projects', projectId: project.id })}
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
  /* As the conversation's header (lot 4): the title and its line, then what the board says and its controls. */
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 0 14px 0 22px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .target-ci {
    display: flex;
    flex: 0 1 auto;
    min-width: 0;
    max-width: 240px;
  }
  .h-title {
    display: grid;
    grid-template-columns: auto minmax(0, auto);
    justify-content: start;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }
  .h-title > :not(.tabs) {
    grid-column: 1;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
  }
  .proj-name {
    flex: none;
  }
  .sub {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /*
   * What the places say, on up to three lines when it is long; the full text in its tooltip. Under 1240 pixels the usual
   * ones give their room to the controls, as in the preview; why nothing starts stays, capped.
   */
  .places {
    flex: 0 1 auto;
    max-width: 30ch;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
    white-space: normal;
    text-align: right;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.3;
  }
  .places.missing {
    color: var(--del);
    font-weight: 550;
  }
  .add-account {
    flex: none;
  }
  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  /* « Tickets · Environnements » (ESC-67): the board's two views. */
  .tabs {
    grid-column: 2;
    grid-row: 1 / 3;
    align-self: center;
    margin-left: 14px;
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 9px;
    background: var(--raise);
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 10px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
  }
  .tabs button.on {
    background: rgba(var(--fg-rgb), 0.08);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  /* « Après validation : merge squash → main », the button of the Kanban's settings. */
  .cfg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 30px;
    padding: 0 9px 0 11px;
    border: none;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: box-shadow 0.15s;
  }
  .cfg:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
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
  @media (max-width: 1240px) {
    .places:not(.missing) {
      display: none;
    }
    /* The tabs keep their icons alone: their names are in their tooltips. */
    .tabs .tl {
      display: none;
    }
    .tabs button {
      padding: 0 7px;
    }
    .places.missing {
      max-width: 132px;
      font-size: 11.5px;
    }
  }
  /* A narrow window: a long summary is cut, whole in its tooltip. */
  @media (max-width: 1100px) {
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

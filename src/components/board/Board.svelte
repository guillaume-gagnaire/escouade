<script lang="ts">
  import { busy, COLUMNS, columnTickets, pauseLabel, placesLabel, quotaUntil, settingsSummary } from '../../lib/board';
  import { plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import BoardColumn from './BoardColumn.svelte';

  let { project: given }: { project: Project } = $props();

  const MISSING = 'Claude Code introuvable — aucun ticket ne démarre';

  // The project as the app holds it now (its board settings change under this view).
  const project = $derived(app.projects.find((p) => p.id === given.id) ?? given);
  const s = $derived(project.board);
  const tickets = $derived(Object.values(app.tickets).filter((t) => t.projectId === project.id));
  const busyCount = $derived(busy(tickets));
  const looping = $derived(tickets.filter((t) => t.column === 'doing').length);
  const quota = $derived(quotaUntil(Object.values(app.agents)));
  const target = $derived(s.target || app.git[project.id]?.branch || 'main');
  const sub = $derived(`${project.name} · ${plural(tickets.length, 'ticket', 'tickets')} · ${looping} en boucle`);
  const summary = $derived(settingsSummary(s, target));
  const places = $derived(placesLabel(busyCount, s.maxParallel, quota));
  /** Why no ticket starts from the target branch (no commit yet, or gone), as the backend says. */
  const issue = $derived(app.boardIssues[project.id] ?? null);
  /** Why no ticket of any board starts for now (a quota window, a usage limit), as the backend says. */
  const pause = $derived(app.autopilotPause);
  const paused = $derived(pause ? pauseLabel(pause, app.now) : '');

  async function toggleAutopilot() {
    // The project comes back through the backend's event, in order with any newer one: its answer is not written over it.
    await app.run(api.boardSet(project.id, { ...$state.snapshot(s), autopilot: !s.autopilot }));
  }
</script>

<main class="board">
  <header class="head">
    <div class="who">
      <span class="t">Kanban</span>
      <span class="sub mono" title={sub}>{sub}</span>
    </div>
    <div style="flex:1"></div>
    {#if !app.claudeFound}
      <!-- No place is worth showing: nothing starts without Claude Code. -->
      <span class="places missing" title={MISSING}>{MISSING}</span>
    {:else if issue}
      <!-- Nor while the target branch cannot start a ticket. -->
      <span class="places missing" title={issue}>{issue}</span>
    {:else if pause}
      <!-- Nor while the autopilot is paused: the line under the header says why. -->
    {:else}
      <span class="places" title={places}>{places}</span>
    {/if}
    <!-- Its label goes in a narrow window, its icon and its name stay. -->
    <button
      class="imp"
      aria-label="Importer"
      title="Importer des tickets de Jira, Trello ou GitHub Issues"
      onclick={() => (app.modal = { kind: 'import', projectId: project.id })}
      ><span class="ic" aria-hidden="true">⤓</span><span class="l">Importer</span></button
    >
    <!-- Its tooltip holds the summary in full: the summary is cut in a narrow window. -->
    <button
      class="cfg"
      title={`Réglages du Kanban — ${summary}`}
      onclick={() => (app.modal = { kind: 'settings', tab: 'board', projectId: project.id })}
    >
      <span class="gear">⚙</span><span class="k">Après validation :</span><span class="v mono">{summary}</span>
    </button>
    <div class="auto" title="Les tickets « À faire » partent seuls dès qu'une place se libère">
      <span class="l">{s.autopilot ? 'Pilote auto' : 'Pilote auto · off'}</span>
      <button
        class="switch"
        class:on={s.autopilot}
        role="switch"
        aria-checked={s.autopilot}
        aria-label="Pilote auto"
        onclick={toggleAutopilot}
      ></button>
    </div>
  </header>
  {#if pause}
    <!-- A line of its own under the header: its text and its button would not fit in a narrow one. -->
    <div class="pause" role="status">
      <span class="dot" aria-hidden="true"></span>
      <span class="why" title={paused}>{paused}</span>
      <button class="resume" onclick={() => app.run(api.autopilotResume())}>Reprendre maintenant</button>
    </div>
  {/if}
  <div class="cols">
    {#each COLUMNS as c (c.id)}
      <BoardColumn column={c} tickets={columnTickets(tickets, c.id)} {project} {busyCount} {quota} />
    {/each}
  </div>
</main>

<style>
  .board {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 20px 0 24px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
    container: head / inline-size;
  }
  /*
   * In a narrow window the settings button and the switch keep their size (the button's summary is
   * capped, its label never cut), the places wrap over up to three lines and the project's line is
   * cut with an ellipsis, the full texts in their tooltips.
   */
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .t {
    font-size: 15px;
    font-weight: 700;
  }
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11px;
    color: var(--dim);
  }
  .places {
    flex: none;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
    white-space: normal;
    text-align: right;
    font-size: 12px;
    line-height: 1.3;
    color: var(--muted);
  }
  .places.missing {
    font-weight: 600;
    color: var(--del);
  }
  /*
   * Under this width the longest places (« Claude Code introuvable — … ») no longer fit beside the rest, nor a
   * long summary in the settings button: it is cut, in full in the button's tooltip.
   */
  @container head (max-width: 880px) {
    .places {
      max-width: 108px;
    }
    .places.missing {
      max-width: 132px;
      font-size: 11px;
    }
    .v {
      max-width: 22ch;
    }
    .imp .l {
      display: none;
    }
    .imp {
      padding: 0 10px;
    }
  }
  .imp {
    height: 34px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 13px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .imp:hover {
    border-color: var(--accent);
  }
  .imp .ic {
    color: var(--accent);
  }
  .cfg {
    height: 34px;
    flex: none;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .cfg:hover {
    border-color: var(--accent);
  }
  .gear {
    flex: none;
    font-size: 14px;
    color: var(--muted);
  }
  .k {
    flex: none;
    color: var(--muted);
  }
  .v {
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 11.5px;
    font-weight: 600;
  }
  .auto {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 6px 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--elev);
  }
  .auto .l {
    font-size: 12.5px;
    font-weight: 600;
  }
  .pause {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 20px 8px 24px;
    border-bottom: 1px solid var(--line);
    background: var(--elev);
    font-size: 12.5px;
  }
  .pause .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--wait);
  }
  /* Cut with an ellipsis in a narrow window, in full in its tooltip. */
  .why {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .resume {
    flex: none;
    height: 28px;
    padding: 0 12px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev2);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .resume:hover {
    border-color: var(--accent);
  }
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    padding: 16px 18px 18px;
  }
</style>

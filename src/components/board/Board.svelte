<script lang="ts">
  import { busy, COLUMNS, columnTickets, placesLabel, quotaUntil, settingsSummary } from '../../lib/board';
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

  async function toggleAutopilot() {
    // The project comes back through the backend's event, in order with any newer one: its answer is not written over it.
    await app.run(api.boardSet(project.id, { ...$state.snapshot(s), autopilot: !s.autopilot }));
  }
</script>

<main class="board">
  <header class="head">
    <div class="who">
      <span class="t">Tableau</span>
      <span class="sub mono" title={sub}>{sub}</span>
    </div>
    <div style="flex:1"></div>
    {#if !app.claudeFound}
      <!-- No place is worth showing: nothing starts without Claude Code. -->
      <span class="places missing" title={MISSING}>{MISSING}</span>
    {:else if issue}
      <!-- Nor while the target branch cannot start a ticket. -->
      <span class="places missing" title={issue}>{issue}</span>
    {:else}
      <span class="places" title={places}>{places}</span>
    {/if}
    <!-- Its tooltip holds the summary in full: the summary is cut in a narrow window. -->
    <button
      class="cfg"
      title={`Réglages du tableau — ${summary}`}
      onclick={() => (app.modal = { kind: 'boardSettings', projectId: project.id })}
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
  .cols {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
    padding: 16px 18px 18px;
  }
</style>

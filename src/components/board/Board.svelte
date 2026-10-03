<script lang="ts">
  import { busy, COLUMNS, columnTickets, placesLabel, quotaUntil, settingsSummary } from '../../lib/board';
  import { plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import BoardColumn from './BoardColumn.svelte';

  let { project: given }: { project: Project } = $props();

  // The project as the app holds it now (its board settings change under this view).
  const project = $derived(app.projects.find((p) => p.id === given.id) ?? given);
  const s = $derived(project.board);
  const tickets = $derived(Object.values(app.tickets).filter((t) => t.projectId === project.id));
  const busyCount = $derived(busy(tickets));
  const looping = $derived(tickets.filter((t) => t.column === 'doing').length);
  const quota = $derived(quotaUntil(Object.values(app.agents)));
  const target = $derived(s.target || app.git[project.id]?.branch || 'main');

  async function toggleAutopilot() {
    const saved = await app.run(api.boardSet(project.id, { ...$state.snapshot(s), autopilot: !s.autopilot }));
    if (saved) app.replaceProject(saved);
  }
</script>

<main class="board">
  <header class="head">
    <div class="who">
      <span class="t">Tableau</span>
      <span class="sub mono">{project.name} · {plural(tickets.length, 'ticket', 'tickets')} · {looping} en boucle</span>
    </div>
    <div style="flex:1"></div>
    {#if app.claudeFound}
      <span class="places">{placesLabel(busyCount, s.maxParallel, quota)}</span>
    {:else}
      <!-- No place is worth showing: nothing starts without Claude Code. -->
      <span class="places missing">Claude Code introuvable — aucun ticket ne démarre</span>
    {/if}
    <button class="cfg" title="Réglages du tableau" onclick={() => (app.modal = { kind: 'boardSettings', projectId: project.id })}>
      <span class="gear">⚙</span><span class="k">Après validation :</span><span class="v mono">{settingsSummary(s, target)}</span>
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
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .t {
    font-size: 15px;
    font-weight: 700;
  }
  .sub {
    font-size: 11px;
    color: var(--dim);
  }
  .places {
    font-size: 12px;
    color: var(--muted);
  }
  .places.missing {
    font-weight: 600;
    color: var(--del);
  }
  .cfg {
    height: 34px;
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
    font-size: 14px;
    color: var(--muted);
  }
  .k {
    color: var(--muted);
  }
  .v {
    font-size: 11.5px;
    font-weight: 600;
  }
  .auto {
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

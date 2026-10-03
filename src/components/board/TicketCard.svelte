<script lang="ts">
  import { canStart, waitLabel } from '../../lib/board';
  import { plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project, Ticket } from '../../lib/types';

  let {
    ticket: t,
    project,
    queueIndex,
    busyCount,
    quota,
    onedit,
  }: { ticket: Ticket; project: Project; queueIndex: number; busyCount: number; quota: number | null; onedit: () => void } = $props();

  const s = $derived(project.board);

  function open() {
    if (t.column === 'todo') onedit();
    else if (t.agentId) app.selectAgent(t.agentId);
  }

  async function remove() {
    const gone = await app.run(api.ticketDelete(t.id).then(() => true));
    if (gone) delete app.tickets[t.id];
  }

  function contextMenu(e: MouseEvent) {
    if (t.column !== 'todo') return;
    menu.show(e, [
      { label: 'Modifier', onClick: onedit },
      { label: 'Passer en tête', onClick: () => app.run(api.ticketPrioritize(t.id)) },
      { label: '', separator: true },
      { label: 'Supprimer', danger: true, onClick: remove },
    ]);
  }
</script>

<div
  class="card"
  role="button"
  tabindex="0"
  aria-label={`${t.key} ${t.title}`}
  onclick={open}
  onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && open()}
  oncontextmenu={contextMenu}
>
  <div class="top">
    <span class="key mono">{t.key}</span>
  </div>
  <span class="title">{t.title}</span>
  {#if t.column === 'todo'}
    <span class="meta">{plural(t.criteria.length, 'critère', 'critères')} · max {t.maxLoops} boucles</span>
    <div class="wait">
      <span class="k">{waitLabel(t, queueIndex, s, busyCount)}</span>
      <div style="flex:1"></div>
      {#if canStart(t, s, busyCount, quota)}
        <button
          class="small"
          onclick={(e) => {
            e.stopPropagation();
            app.run(api.ticketStart(t.id));
          }}>Lancer</button
        >
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 11px 12px;
    border-radius: var(--r-sm);
    background: var(--elev);
    border: 1px solid var(--line);
    cursor: pointer;
    outline: none;
  }
  .card:hover,
  .card:focus-visible {
    border-color: var(--line2);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .key {
    font-size: 10.5px;
    color: var(--dim);
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.4;
    text-wrap: pretty;
  }
  .meta {
    font-size: 11.5px;
    color: var(--muted);
  }
  .wait {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .k {
    font-size: 11px;
    color: var(--dim);
  }
  .small {
    height: 24px;
    padding: 0 10px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .small:hover {
    border-color: var(--accent);
  }
</style>

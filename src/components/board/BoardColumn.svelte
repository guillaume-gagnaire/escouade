<script lang="ts">
  import { queueIndices } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Column, Project, Ticket, TicketDraft } from '../../lib/types';
  import TicketCard from './TicketCard.svelte';
  import TicketForm from './TicketForm.svelte';

  let {
    column,
    tickets,
    project,
    busyCount,
    quota,
  }: {
    // Its name and its empty text are in the catalogs, by `id`: the column of `COLUMNS` is only given for its color.
    column: { id: Column; color: string };
    tickets: Ticket[];
    project: Project;
    busyCount: number;
    quota: number | null;
  } = $props();

  let adding = $state(false);
  let editing = $state<string | null>(null);
  /** Each ticket's place in the queue: one that waits for others is passed over, as the autopilot does. */
  const queue = $derived(column.id === 'todo' ? queueIndices(tickets, app.tickets) : {});

  // The command's ticket shows at once, unless the backend's event, which may come before its answer, already brought
  // it, maybe newer (started meanwhile); one coming after brings the same or newer.
  async function add(d: TicketDraft) {
    const made = await app.run(api.ticketCreate(project.id, d));
    if (!made) return;
    app.tickets[made.id] ??= made;
    adding = false;
  }

  async function save(id: string, d: TicketDraft) {
    const saved = await app.run(api.ticketUpdate(id, d));
    if (!saved) return;
    // Only a ticket "À faire" is edited: one gone or under way since is newer than this.
    if (app.tickets[saved.id]?.column === 'todo') app.tickets[saved.id] = saved;
    editing = null;
  }
</script>

<section class="col" aria-label={t(`board.columns.${column.id}`)}>
  <div class="head">
    <span class="dot" style:background={column.color}></span>
    <span class="label">{t(`board.columns.${column.id}`)}</span>
    <span class="count mono">{tickets.length}</span>
    <div style="flex:1"></div>
    {#if column.id === 'todo'}
      <button class="plus" title={t('board.column.newTicket')} aria-label={t('board.column.newTicket')} onclick={() => (adding = true)}
        >+</button
      >
    {/if}
  </div>
  <div class="cards">
    {#if adding}
      <TicketForm projectId={project.id} onsubmit={add} oncancel={() => (adding = false)} />
    {/if}
    {#each tickets as x, i (x.id)}
      {#if editing === x.id}
        <TicketForm ticket={x} projectId={project.id} onsubmit={(d) => save(x.id, d)} oncancel={() => (editing = null)} />
      {:else}
        <TicketCard ticket={x} {project} queueIndex={queue[x.id] ?? i} {busyCount} {quota} onedit={() => (editing = x.id)} />
      {/if}
    {/each}
    {#if !tickets.length && !adding}
      <div class="empty">{t(`board.empty.${column.id}`)}</div>
    {/if}
  </div>
</section>

<style>
  .col {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-radius: var(--r);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 10px 10px 14px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .label {
    font-size: 13px;
    font-weight: 700;
  }
  .count {
    font-size: 11px;
    color: var(--dim);
  }
  .plus {
    width: 26px;
    height: 26px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--accent);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }
  .plus:hover {
    border-color: var(--accent);
  }
  .cards {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 0 8px 10px;
  }
  .empty {
    flex: none;
    padding: 18px 12px;
    border-radius: var(--r-sm);
    border: 1px dashed var(--line2);
    text-align: center;
    font-size: 11.5px;
    color: var(--dim);
    text-wrap: pretty;
  }
</style>

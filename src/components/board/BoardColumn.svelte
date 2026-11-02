<script lang="ts">
  import { queueIndices } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Column, Project, Ticket, TicketDraft } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import TicketCard from './TicketCard.svelte';
  import { tip } from '../../lib/tip';

  let {
    column,
    tickets,
    project,
    busyCount,
    quota,
  }: {
    // Its name and its empty text are in the catalogs, by `id`: the column of `COLUMNS` is only given for its ring.
    column: { id: Column; tone: string; fill: number };
    tickets: Ticket[];
    project: Project;
    busyCount: number;
    quota: number | null;
  } = $props();

  let refreshing = $state(false);
  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    await app.run(api.ticketPrsRefresh(project.id));
    refreshing = false;
  }

  /** Each ticket's place in the queue: one that waits for others is passed over, as the autopilot does. */
  const queue = $derived(column.id === 'todo' ? queueIndices(tickets, app.tickets) : {});

  /**
   * The form of a new ticket, or (`ticket`) of one to do, in its window. The window outlives this board (another project
   * selected meanwhile, by a click on a notification): the ticket is made in the project it was opened from.
   */
  function openForm(ticket?: Ticket) {
    const projectId = project.id;
    app.modal = {
      kind: 'ticket',
      projectId,
      ticket,
      onSubmit: ticket ? (d) => save(ticket.id, d) : (d) => add(projectId, d),
    };
  }

  // The command's ticket shows at once, unless the backend's event, which may come before its answer, already brought
  // it, maybe newer (started meanwhile); one coming after brings the same or newer. True once it is made: the window closes.
  async function add(projectId: string, d: TicketDraft) {
    const made = await app.run(api.ticketCreate(projectId, d));
    if (!made) return false;
    app.tickets[made.id] ??= made;
    return true;
  }

  async function save(id: string, d: TicketDraft) {
    const saved = await app.run(api.ticketUpdate(id, d));
    if (!saved) return false;
    // Only a ticket "À faire" is edited: one gone or under way since is newer than this.
    if (app.tickets[saved.id]?.column === 'todo') app.tickets[saved.id] = saved;
    return true;
  }
</script>

<!-- A column without a box (2.0): its ring, its name and its count, then its cards. -->
<section class="col" aria-label={t(`board.columns.${column.id}`)} style:--tone={column.tone}>
  <div class="head">
    <i
      class="ring"
      class:part={column.fill > 0 && column.fill < 100}
      class:full={column.fill >= 100}
      style:--fill="{column.fill}%"
      aria-hidden="true"
    ></i>
    <span class="label">{t(`board.columns.${column.id}`)}</span>
    <span class="count">{tickets.length}</span>
    {#if column.id === 'pr'}
      <!-- Its pull requests read now, rather than at the next minute (ESC-59). -->
      <button
        class="add"
        class:spin={refreshing}
        use:tip={t('board.column.refreshHint')}
        aria-label={t('board.column.refresh')}
        aria-busy={refreshing}
        onclick={refresh}><Icon name="restart" size={14} /></button
      >
    {/if}
    {#if column.id === 'todo'}
      <button class="add" use:tip={t('board.column.newTicket')} aria-label={t('board.column.newTicket')} onclick={() => openForm()}
        ><Icon name="plus" size={14} /></button
      >
    {/if}
  </div>
  <div class="cards">
    {#each tickets as x, i (x.id)}
      <TicketCard ticket={x} {project} queueIndex={queue[x.id] ?? i} {busyCount} {quota} onedit={() => openForm(x)} />
    {/each}
    {#if !tickets.length}
      <div class="empty">{t(`board.empty.${column.id}`)}</div>
    {/if}
  </div>
</section>

<style>
  .col {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 4px 0 6px;
    font-weight: 600;
  }
  /* The column's state, a ring of its tone filled as far as the tickets in it have come. */
  .ring {
    flex: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 2px var(--tone);
  }
  .ring.part {
    background: conic-gradient(var(--tone) 0 var(--fill), transparent var(--fill));
  }
  .ring.full {
    background: var(--tone);
  }
  .label {
    white-space: nowrap;
  }
  .count {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .add {
    margin-left: auto;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .add:hover {
    background: var(--hover);
    color: var(--text);
  }
  /* « Rafraîchir » while its pull requests are read. */
  .add.spin :global(.icon) {
    animation: turn 0.9s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .add.spin :global(.icon) {
      animation: none;
    }
  }
  /* The cards scroll in their column, the header stays; the room around them keeps their shadow and focus ring. */
  .cards {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 0 -4px;
    padding: 0 4px 4px;
  }
  .empty {
    flex: none;
    display: grid;
    place-items: center;
    min-height: 86px;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px dashed var(--line-2);
    color: var(--dim);
    font-size: 12.5px;
    text-align: center;
    text-wrap: pretty;
  }
</style>

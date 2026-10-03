<script lang="ts">
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
    column: { id: Column; label: string; color: string; empty: string };
    tickets: Ticket[];
    project: Project;
    busyCount: number;
    quota: number | null;
  } = $props();

  let adding = $state(false);
  let editing = $state<string | null>(null);

  async function add(d: TicketDraft) {
    const t = await app.run(api.ticketCreate(project.id, d));
    if (!t) return;
    app.tickets[t.id] = t;
    adding = false;
  }

  async function save(id: string, d: TicketDraft) {
    const t = await app.run(api.ticketUpdate(id, d));
    if (!t) return;
    app.tickets[t.id] = t;
    editing = null;
  }
</script>

<section class="col" aria-label={column.label}>
  <div class="head">
    <span class="dot" style:background={column.color}></span>
    <span class="label">{column.label}</span>
    <span class="count mono">{tickets.length}</span>
    <div style="flex:1"></div>
    {#if column.id === 'todo'}
      <button class="plus" title="Nouveau ticket" aria-label="Nouveau ticket" onclick={() => (adding = true)}>+</button>
    {/if}
  </div>
  <div class="cards">
    {#if adding}
      <TicketForm onsubmit={add} oncancel={() => (adding = false)} />
    {/if}
    {#each tickets as t, i (t.id)}
      {#if editing === t.id}
        <TicketForm ticket={t} onsubmit={(d) => save(t.id, d)} oncancel={() => (editing = null)} />
      {:else}
        <TicketCard ticket={t} {project} queueIndex={i} {busyCount} {quota} onedit={() => (editing = t.id)} />
      {/if}
    {/each}
    {#if !tickets.length && !adding}
      <div class="empty">{column.empty}</div>
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

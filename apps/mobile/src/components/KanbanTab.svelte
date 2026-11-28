<script lang="ts">
  // « Kanban » (MO-7, docs/MOBILE.md § 3.7, direction B): the tickets of the column chosen in the head (KanbanHead),
  // of the project shown; each line with its gestures (TicketLine). Read while the tab is shown, every few seconds.
  // Before a space said its projects, « Choisis un projet »; with none in any space, « Aucun projet ».
  import { onMount } from 'svelte';
  import { t } from '$shared/lib/i18n';
  import { kanban, shownColumn, ticketsIn, watchBoard } from '../lib/board.svelte';
  import { stateOf, phone } from '../lib/phone-spaces.svelte';
  import Empty from './Empty.svelte';
  import TicketLine from './TicketLine.svelte';

  onMount(() => watchBoard());

  const board = $derived(kanban.board);
  const column = $derived(board ? shownColumn(board.counts, kanban.column) : null);
  const tickets = $derived(board && column ? ticketsIn(board, column) : []);
  const older = $derived(board && column === 'done' ? board.counts.done - tickets.length : 0);
  const gestures = $derived(board ? board.counts.review + board.counts.todo > 0 : false);
  // Every space open said it has no project.
  const none = $derived(
    !kanban.choice &&
      phone.spaces.some((s) => stateOf(s) === 'open') &&
      phone.spaces.filter((s) => stateOf(s) === 'open').every((s) => kanban.lists[s.id]?.length === 0),
  );
</script>

{#if kanban.error}
  <p class="err" role="alert">{kanban.error}</p>
{/if}
{#if board && column}
  {#if tickets.length}
    <ul aria-label={t(`board.columns.${column}`)}>
      {#each tickets as ticket (ticket.id)}
        <TicketLine {ticket} />
      {/each}
    </ul>
    {#if older > 0}<p class="note">{t('mobile.kanban.olderDone', { count: older })}</p>{/if}
  {:else}
    <Empty icon="kanban" title={t(`mobile.kanban.empty.${column}`)} text="" />
  {/if}
  {#if gestures}<p class="note">{t('mobile.kanban.hint')}</p>{/if}
{:else if none}
  <Empty icon="kanban" title={t('mobile.kanban.noProject')} text={t('mobile.kanban.noProjectText')} />
{:else}
  <Empty icon="kanban" title={t('mobile.kanban.choose')} text={t('mobile.kanban.chooseText')} />
{/if}

<style>
  ul {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
  }
  .note {
    margin: 14px var(--gut) 0;
    font-size: var(--small);
    line-height: 1.45;
    color: var(--muted);
  }
  .err {
    margin: 10px var(--gut) 4px;
    padding: 10px 12px;
    border-radius: var(--r-m);
    background: oklch(from var(--del) l c h / 0.1);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.4);
    color: var(--text);
    font-size: var(--small);
    overflow-wrap: anywhere;
  }
</style>

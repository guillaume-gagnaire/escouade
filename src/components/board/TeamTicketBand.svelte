<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { hueOf, initialOf } from '../../lib/sharing';
  import { app } from '../../lib/state.svelte';
  import { assigneeLine, conflictLine, holderLine, localOnly, teamBoardApi } from '../../lib/teamboard';
  import type { Project, Ticket } from '../../lib/types';

  // A ticket of a team's Kanban (TS-6): who holds it and since when (« injoignable depuis 12 min » when their machine no
  // longer answers), an interrupted claim and « Reprendre », whom it is assigned to, a move the server refused, and the
  // conflicts this machine lost (« Bob a changé le titre en même temps · Reprendre le mien »).
  let { ticket, project }: { ticket: Ticket; project: Project | null | undefined } = $props();

  const holder = $derived(holderLine(ticket));
  const assignee = $derived(assigneeLine(ticket));
  const conflicts = $derived(ticket.team?.conflicts ?? []);
  const local = $derived(localOnly(ticket, project));

  function act(e: Event, f: () => unknown) {
    e.stopPropagation();
    f();
  }
</script>

{#if local}
  <span class="local" data-team-local><Icon name="lock" size={13} /><span>{t('teamBoard.card.localOnly')}</span></span>
{/if}
{#if holder}
  <div class="holder {holder.tone}" role="status" data-team-holder={holder.tone}>
    {#if holder.tone !== 'here' && ticket.team?.holder}
      {@const h = ticket.team.holder}
      <i class="disc" style:--h={hueOf(h.device || h.name)} aria-hidden="true">{initialOf(h.name || '?')}</i>
    {:else}
      <Icon name="team" size={14} />
    {/if}
    <span class="text"
      >{holder.text}{#if holder.unreachable}<span class="away">{' · '}{holder.unreachable}</span>{/if}</span
    >
    {#if holder.resumable}
      <button class="btn small" onclick={(e) => act(e, () => app.run(teamBoardApi.resume(ticket.id)))}>{t('teamBoard.card.resume')}</button>
    {/if}
  </div>
{/if}
{#if assignee}
  <span class="assignee" data-team-assignee><Icon name="user" size={13} /><span>{assignee}</span></span>
{/if}
{#if ticket.team?.refused}
  <div class="band quiet" role="status" data-team-refused><Icon name="info" size={14} /><span>{ticket.team.refused}</span></div>
{/if}
{#each conflicts as c (c.field + c.stamp)}
  {@const line = conflictLine(c)}
  <div class="band" role="status" data-team-conflict={c.field}>
    <Icon name="team" size={14} /><span>{line.text}</span>
    <div class="row">
      <button class="btn small" onclick={(e) => act(e, () => app.run(teamBoardApi.settle(ticket.id, c.field, true)))}>{line.mine}</button>
      <button class="btn small" onclick={(e) => act(e, () => app.run(teamBoardApi.settle(ticket.id, c.field, false)))}>{line.theirs}</button
      >
    </div>
  </div>
{/each}

<style>
  .holder,
  .band {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px 8px;
    padding: 7px 10px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .holder.interrupted,
  .band:not(.quiet) {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .band {
    grid-template-columns: 14px minmax(0, 1fr);
  }
  .band > :global(.icon),
  .holder.interrupted > :global(.icon) {
    color: var(--wait);
  }
  .band.quiet > :global(.icon) {
    color: var(--muted);
  }
  .band .row {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .away {
    color: var(--wait);
  }
  .disc {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: light-dark(oklch(0.9 0.05 var(--h)), oklch(0.42 0.08 var(--h)));
    color: light-dark(oklch(0.36 0.1 var(--h)), oklch(0.97 0.02 var(--h)));
    font: normal 600 9.5px/1 var(--font);
  }
  .local,
  .assignee {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 12px;
  }
  .btn.small {
    height: 28px;
    font-size: 12.5px;
  }
</style>

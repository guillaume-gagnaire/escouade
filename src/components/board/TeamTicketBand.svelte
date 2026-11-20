<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { hueOf, initialOf } from '../../lib/sharing';
  import { app } from '../../lib/state.svelte';
  import { takeFromCard } from '../../lib/handoff';
  import { assigneeLine, conflictLine, handoffLine, holderLine, localOnly, teamBoardApi } from '../../lib/teamboard';
  import type { Project, Ticket } from '../../lib/types';

  // A ticket of a team's Kanban (TS-6): who holds it and since when (« injoignable depuis 12 min » when their machine no
  // longer answers), an interrupted claim and « Reprendre », whom it is assigned to, a move the server refused, and the
  // conflicts this machine lost (« Bob a changé le titre en même temps · Reprendre le mien »).
  let { ticket, project }: { ticket: Ticket; project: Project | null | undefined } = $props();

  // During a passage its band says who holds it (« Passé à Bob », « Alice te passe ») : the holder line would say the claim the
  // server handed, not who works on it.
  const holder = $derived(ticket.team?.handoff ? null : holderLine(ticket));
  const assignee = $derived(assigneeLine(ticket));
  const conflicts = $derived(ticket.team?.conflicts ?? []);
  const local = $derived(localOnly(ticket, project));
  // A passage of its agent under way (TS-17): « Alice te passe DEM-41 · Prendre », « Passé à Bob · Annuler le passage ».
  const handoff = $derived(handoffLine(ticket));
  let taking = $state(false);

  async function take() {
    if (taking) return;
    taking = true;
    try {
      await takeFromCard(ticket.id);
    } finally {
      taking = false;
    }
  }

  function act(e: Event, f: () => unknown) {
    e.stopPropagation();
    f();
  }
</script>

{#if local}
  <span class="local" data-team-local><Icon name="lock" size={13} /><span>{t('teamBoard.card.localOnly')}</span></span>
{/if}
{#if handoff}
  <div class="band handoff {handoff.tone}" role="status" data-team-handoff={handoff.tone}>
    <Icon name="team" size={14} /><span>{handoff.text}</span>
    {#if handoff.tone !== 'between'}
      <div class="row">
        {#if handoff.tone === 'incoming'}
          <button class="btn small primary" disabled={taking} data-testid="team-handoff-take-card" onclick={(e) => act(e, take)}
            >{taking ? t('teamBoard.handoff.taking') : t('teamBoard.handoff.take')}</button
          >
        {:else}
          <button
            class="btn small"
            data-testid="team-handoff-cancel"
            onclick={(e) => act(e, () => app.run(teamBoardApi.handoffCancel(ticket.id)))}>{t('teamBoard.handoff.cancel')}</button
          >
        {/if}
      </div>
    {/if}
  </div>
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
{#if ticket.formerKeys?.length}
  <!-- The keys it had before it was poured into the team's Kanban (TS-7): it is found by them still. -->
  <span class="assignee" data-team-former
    >{t('teamBoard.card.former', { count: ticket.formerKeys.length, keys: ticket.formerKeys.join(', ') })}</span
  >
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
  .band.handoff.incoming {
    background: var(--p-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 40%, transparent);
  }
  .band.handoff.incoming > :global(.icon) {
    color: var(--p-ink);
  }
  .band.handoff.outgoing,
  .band.handoff.between {
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .band.handoff.outgoing > :global(.icon),
  .band.handoff.between > :global(.icon) {
    color: var(--muted);
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

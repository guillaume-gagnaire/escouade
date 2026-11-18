<script lang="ts">
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { author, teamMemoryApi, type TeamMemoryEntry } from '../../lib/team-memory';
  import type { Project, Ticket } from '../../lib/types';

  // « Ce ticket a laissé 2 souvenirs · Garder · Écarter » (TS-10, C14-B): on a ticket of a team's project to validate,
  // the memories its agent proposed, each kept or set aside by a human (its author, or an administrator); they wait
  // otherwise, in Réglages › Projets › Équipe › Mémoire partagée. Nothing is kept by the validation itself. Read again at each
  // change of the project's memory.
  let { project, ticket }: { project: Project; ticket: Ticket } = $props();

  let left = $state<TeamMemoryEntry[]>([]);
  let open = $state(false);
  let busy = $state(false);

  $effect(() => {
    void app.memoryTicks[project.id];
    void load(project.id, ticket.key);
  });

  async function load(id: string, key: string) {
    if (!project.team || project.private) {
      left = [];
      return;
    }
    try {
      const got = await teamMemoryApi.ofTicket(id, key);
      if (id === project.id && key === ticket.key) left = got;
    } catch {
      left = [];
    }
  }

  async function decide(entries: TeamMemoryEntry[], keep: boolean) {
    busy = true;
    try {
      for (const e of entries) {
        const v = await app.run(teamMemoryApi.decide(project.id, e.id, keep));
        if (!v) break;
      }
      await load(project.id, ticket.key);
    } finally {
      busy = false;
    }
  }

  function stop(e: Event) {
    e.stopPropagation();
  }
</script>

{#if left.length}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="memories" data-testid="ticket-memories" onclick={stop}>
    <div class="line">
      <span class="what">{t('memory.ticket.left', { count: left.length })}</span>
      <button class="btn small ghost" aria-expanded={open} onclick={() => (open = !open)}
        >{open ? t('memory.ticket.hide') : t('memory.ticket.show')}</button
      >
      <button class="btn small" disabled={busy} onclick={() => decide(left, true)} data-testid="ticket-memories-keep"
        >{t('memory.ticket.keep')}</button
      >
      <button class="btn small ghost" disabled={busy} onclick={() => decide(left, false)} data-testid="ticket-memories-reject"
        >{t('memory.ticket.reject')}</button
      >
    </div>
    {#if open}
      <ul>
        {#each left as e (e.id)}
          <li data-testid="ticket-memory" data-name={e.name}>
            <span class="name mono">{e.name}</span>
            <span class="desc">{e.description}</span>
            {#if author(e.provenance).agent}<span class="who">{author(e.provenance).agent}</span>{/if}
            {#if e.provenance.thirdParty}<span class="warn">{t('memory.team.thirdParty')}</span>{/if}
            {#if e.unknown.length}<span class="warn">{t('memory.team.unknown', { hosts: e.unknown.join(', ') })}</span>{/if}
            <span class="acts">
              <button class="btn small ghost" disabled={busy} onclick={() => decide([e], true)}>{t('memory.ticket.keep')}</button>
              <button class="btn small ghost" disabled={busy} onclick={() => decide([e], false)}>{t('memory.ticket.reject')}</button>
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

<style>
  .memories {
    display: grid;
    gap: 6px;
    margin-top: 8px;
    padding: 7px 9px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.04);
    cursor: default;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .what {
    flex: 1 1 auto;
    min-width: 0;
    color: var(--soft);
    font-size: 12px;
  }
  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 2px 8px;
    font-size: 12px;
  }
  .name {
    color: var(--text);
    font-weight: 550;
  }
  .desc,
  .who {
    flex-basis: 100%;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .warn {
    flex-basis: 100%;
    color: var(--warn, var(--del));
  }
  .acts {
    display: flex;
    gap: 4px;
  }
</style>

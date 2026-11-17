<script lang="ts">
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { teamDocsApi, type TeamDocsView } from '../../lib/team-docs';
  import type { Project } from '../../lib/types';

  // « Docs du projet d'équipe » (TS-9, C4-B), in the project's Git page under « Branche des docs »: on the team server
  // (the default for a project of a team) or in the repository. The team's choice, applied at once (not with the
  // draft): it says before what changing does, and loses nothing either way.
  let { project }: { project: Project } = $props();

  let view = $state<TeamDocsView | null>(null);
  let busy = $state(false);
  let journal = $state('');

  const tick = $derived(app.teamDocsTick[project.id] ?? 0);

  $effect(() => {
    void tick;
    void project.team;
    void project.teamProject;
    void load(project.id);
  });

  async function load(id: string) {
    if (!project.team || project.private) {
      view = null;
      return;
    }
    const v = await app.run(teamDocsApi.status(id));
    if (v && id === project.id) {
      view = v;
      app.teamDocsOn[id] = v.onServer;
    }
  }

  async function choose(server: boolean, follow: string | null = null) {
    busy = true;
    try {
      const v = await app.run(teamDocsApi.choose(project.id, server, follow));
      if (v) {
        view = v;
        app.teamDocsOn[project.id] = v.onServer;
        journal = '';
      }
    } finally {
      busy = false;
    }
  }

  const onServer = $derived(!!view?.onServer);
  const followable = $derived(/^[0-9a-f]{32}$/i.test(journal.trim()));
</script>

<div class="team-docs-place" data-team-docs-place={onServer ? 'server' : 'repo'}>
  {#if !project.team}
    <p class="note">{t('teamDocs.place.noTeam')}</p>
  {:else if project.private}
    <p class="note">{t('teamDocs.place.private')}</p>
  {:else if view}
    <div class="segmented chips" role="group" aria-label={t('teamDocs.place.label')}>
      <button class="chip" class:on={onServer} aria-pressed={onServer} disabled={busy} onclick={() => !onServer && choose(true)}
        >{t('teamDocs.place.server')}</button
      >
      <button class="chip" class:on={!onServer} aria-pressed={!onServer} disabled={busy} onclick={() => onServer && choose(false)}
        >{t('teamDocs.place.repo')}</button
      >
    </div>
    <p class="note">{onServer ? t('teamDocs.place.serverNote') : t('teamDocs.place.repoNote')}</p>
    <p class="note change">{onServer ? t('teamDocs.place.toRepo') : t('teamDocs.place.toServer')}</p>
    {#if view.journal}
      <p class="note journal">{t('teamDocs.place.journal')} · <span class="mono" data-journal>{view.journal}</span></p>
    {:else}
      <label class="follow">
        <span>{t('teamDocs.place.follow')}</span>
        <input class="field input mono" spellcheck="false" aria-describedby="team-docs-follow" bind:value={journal} />
        <button class="btn" disabled={busy || !followable} onclick={() => choose(true, journal.trim())}
          >{t('teamDocs.place.followGo')}</button
        >
      </label>
      <p class="note" id="team-docs-follow">{t('teamDocs.place.followHint')}</p>
    {/if}
    {#if busy}<p class="note" role="status">{t('teamDocs.place.busy')}</p>{/if}
  {/if}
</div>

<style>
  .team-docs-place {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .chips {
    align-self: flex-start;
    flex: 0 1 auto;
    flex-wrap: wrap;
    justify-content: flex-start;
    max-width: 100%;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .change {
    color: var(--soft);
  }
  .journal .mono {
    color: var(--soft);
    user-select: all;
    word-break: break-all;
  }
  .follow {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12.5px;
    color: var(--soft);
  }
  .follow input {
    flex: 1;
    min-width: 18ch;
  }
</style>

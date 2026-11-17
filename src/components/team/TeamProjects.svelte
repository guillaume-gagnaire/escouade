<script lang="ts">
  // « Projets de l'équipe » (TS-8): the team's list of projects, encrypted under the team's key. The projects already
  // on this device (known by the address of their repository), attached to the team by the member's gesture, their own
  // name and logo kept; the others with « Cloner », which opens the wizard « Nouveau projet » with the repositories
  // filled in. An administrator adds a project to the list, changes it and takes it out.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { pickLogo } from '../../lib/project-logo.svelte';
  import { teamSettingsApi } from '../../lib/team-settings';
  import {
    draftOf,
    listedOf,
    logoOk,
    split,
    teamProjectsApi,
    type Listed,
    type TeamProjectView,
    type TeamProjectsView,
  } from '../../lib/team-projects';
  import { errorText, teams } from '../../lib/teams.svelte';
  import TileFace from '../TileFace.svelte';
  import { tileText } from '../../lib/project-rail';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  let view = $state<TeamProjectsView | null>(null);
  let busy = $state(false);
  let failure = $state('');
  /** The administrator's form: a project of the list to change (its id), or a new one (null), or closed. */
  let editing = $state<{ id: string | null; draft: Listed } | null>(null);
  let removing = $state<TeamProjectView | null>(null);

  const parts = $derived(view ? split(view) : { here: [], elsewhere: [] });
  const teamName = $derived(view?.name || teams.byId(teamId)?.name || '');

  async function act(f: () => Promise<TeamProjectsView>) {
    busy = true;
    failure = '';
    try {
      view = await f();
      return true;
    } catch (e) {
      failure = errorText(e);
      return false;
    } finally {
      busy = false;
    }
  }

  onMount(() => void act(() => teamProjectsApi.get(teamId)));

  const attach = (p: TeamProjectView) =>
    act(async () => {
      await teamSettingsApi.setProjectTeam(p.here!.project, teamId);
      return teamProjectsApi.get(teamId);
    });

  function openHere(p: TeamProjectView) {
    app.selectProject(p.here!.project);
    app.modal = null;
  }

  function clone(p: TeamProjectView) {
    app.modal = { kind: 'newProject', clone: { team: teamId, teamName, project: $state.snapshot(p) } };
  }

  async function save() {
    if (!editing) return;
    const { id, draft } = editing;
    if (await act(() => teamProjectsApi.put(teamId, id, listedOf($state.snapshot(draft))))) editing = null;
  }

  async function remove() {
    if (!removing) return;
    const id = removing.id;
    removing = null;
    await act(() => teamProjectsApi.remove(teamId, id));
  }

  async function chooseLogo() {
    if (!editing) return;
    const picked = await pickLogo();
    if (!picked) return;
    if (!logoOk(picked.url)) {
      failure = t('teamProjects.admin.logoRefused');
      return;
    }
    editing.draft.logo = picked.url;
  }

  const accessLabel = (p: TeamProjectView) =>
    p.access.kind === 'team' ? t('teamProjects.access.team') : t('teamProjects.access.members', { count: p.access.members.length });

  function toggleMember(id: string, on: boolean) {
    if (!editing) return;
    const a = editing.draft.access;
    const members = a.kind === 'members' ? a.members.filter((m) => m !== id) : [];
    editing.draft.access = { kind: 'members', members: on ? [...members, id] : members };
  }
</script>

{#snippet tile(p: TeamProjectView)}
  <span class="tile" style:--c={p.color || 'var(--muted)'} aria-hidden="true"
    ><TileFace text={tileText({ name: p.name, tileLetters: '' })} logo={p.logo} /></span
  >
{/snippet}

{#snippet repos(p: TeamProjectView)}
  <ul class="repos" aria-label={t('teamProjects.repo.count', { count: p.repos.length })}>
    {#each p.repos as r (r.url)}
      <li>
        <span class="mono url">{r.url}</span>
        {#if p.repos.length > 1}<span class="muted small">{t('teamProjects.repo.folder', { folder: r.folder })}</span>{/if}
      </li>
    {/each}
  </ul>
  <p class="muted small">
    {[p.branch ? t('teamProjects.repo.branch', { branch: p.branch }) : '', accessLabel(p)].filter(Boolean).join(' · ')}
  </p>
  <!-- The Kanban the list names (TS-7): a clone, or a project attached here, joins it without an id to copy. -->
  <p class="muted small" data-testid="team-project-board">
    {p.here?.boardJoined ? t('teamProjects.here.boardJoined') : p.board ? t('teamProjects.board.listed') : t('teamProjects.board.none')}
  </p>
  {#if p.here?.boardOther}<p class="advice small">{t('teamProjects.here.boardOther')}</p>{/if}
  {#if view?.admin && p.proposed}
    <div class="acts" data-testid="team-project-proposed">
      <span class="small">{t('teamProjects.board.proposed', { id: p.proposed.board })}</span>
      <button class="btn" disabled={busy} onclick={() => act(() => teamProjectsApi.acceptBoard(teamId, p.id))}
        >{t('teamProjects.board.accept')}</button
      >
    </div>
  {/if}
{/snippet}

{#snippet adminActs(p: TeamProjectView)}
  {#if view?.admin}
    <button class="btn ghost" disabled={busy} onclick={() => (editing = { id: p.id, draft: draftOf(p) })}
      >{t('teamProjects.admin.edit')}</button
    >
    <button class="btn ghost" disabled={busy} onclick={() => (removing = p)}>{t('teamProjects.admin.remove')}</button>
  {/if}
{/snippet}

<div class="projects">
  <p class="lead">{t('teamProjects.lead')}</p>

  {#if !view}
    <p class="muted">{failure || t('teamProjects.loading')}</p>
  {:else}
    {#if view.alarm}
      <div class="alarm" role="alert">
        <span>{t('teamProjects.alarm')}</span>
        <button class="btn" disabled={busy} onclick={() => act(() => teamProjectsApi.retry(teamId))}>{t('teamProjects.retry')}</button>
      </div>
    {/if}
    {#if view.refused}
      <p class="advice small">{t('teamProjects.refused', { count: view.refused })}</p>
    {/if}
    {#if view.projects.length === 0}
      <p class="muted">{view.admin ? t('teamProjects.emptyAdmin') : t('teamProjects.empty')}</p>
    {/if}

    {#if parts.here.length}
      <section aria-label={t('teamProjects.here.title')}>
        <h4>{t('teamProjects.here.title')}</h4>
        {#each parts.here as p (p.id)}
          <article class="item" aria-label={p.name}>
            <div class="head">
              {@render tile(p)}
              <span class="name">{p.here!.name}</span>
              {#if p.here!.attached}<span class="tag on">{t('teamProjects.here.attached')}</span>{/if}
            </div>
            {@render repos(p)}
            {#if p.here!.nameDiffers}
              <p class="note small">{t('teamProjects.here.nameDiffers', { local: p.here!.name, listed: p.name })}</p>
            {/if}
            {#if p.here!.otherTeam}
              <p class="advice small">{t('teamProjects.here.otherTeam')}</p>
            {:else if p.here!.attached}
              <p class="muted small">{t('teamProjects.here.attachedNote')}</p>
            {:else}
              <p class="muted small">{t('teamProjects.here.attachHint')}</p>
            {/if}
            <div class="acts">
              {#if !p.here!.attached && !p.here!.otherTeam}
                <button class="btn primary" disabled={busy} onclick={() => attach(p)}>{t('teamProjects.here.attach')}</button>
              {/if}
              <button class="btn" onclick={() => openHere(p)}>{t('teamProjects.here.open')}</button>
              {@render adminActs(p)}
            </div>
          </article>
        {/each}
      </section>
    {/if}

    {#if parts.elsewhere.length}
      <section aria-label={t('teamProjects.elsewhere.title')}>
        <h4>{t('teamProjects.elsewhere.title')}</h4>
        {#each parts.elsewhere as p (p.id)}
          <article class="item" aria-label={p.name}>
            <div class="head">
              {@render tile(p)}
              <span class="name">{p.name}</span>
            </div>
            {@render repos(p)}
            <div class="acts">
              {#if p.openToMe}
                <button class="btn primary" disabled={busy} onclick={() => clone(p)}>{t('teamProjects.elsewhere.clone')}</button>
              {:else}
                <span class="muted small">{t('teamProjects.elsewhere.notOpen')}</span>
              {/if}
              {@render adminActs(p)}
            </div>
          </article>
        {/each}
      </section>
    {/if}

    {#if removing}
      <div class="confirm" role="alertdialog" aria-label={t('teamProjects.admin.remove')}>
        <p>{t('teamProjects.admin.removeConfirm', { name: removing.name })}</p>
        <div class="acts">
          <button class="btn danger" disabled={busy} onclick={remove}>{t('teamProjects.admin.remove')}</button>
          <button class="btn ghost" onclick={() => (removing = null)}>{t('teamProjects.admin.cancel')}</button>
        </div>
      </div>
    {/if}

    {#if view.admin}
      {#if editing}
        {@const d = editing.draft}
        <section
          class="form"
          aria-label={editing.id ? t('teamProjects.admin.formEdit', { name: d.name }) : t('teamProjects.admin.formAdd')}
        >
          <h4>{editing.id ? t('teamProjects.admin.formEdit', { name: d.name }) : t('teamProjects.admin.formAdd')}</h4>
          <label class="fld"><span>{t('teamProjects.admin.name')}</span><input class="field" bind:value={d.name} maxlength="80" /></label>
          <div class="fld">
            <span>{t('teamProjects.admin.repos')}</span>
            <span class="muted small">{t('teamProjects.admin.reposHint')}</span>
            {#each d.repos as r, i (i)}
              <div class="repo-row">
                <input
                  class="field mono"
                  aria-label={t('teamProjects.admin.repoUrl', { n: i + 1 })}
                  placeholder={t('teamProjects.admin.repoPlaceholder')}
                  spellcheck="false"
                  bind:value={r.url}
                />
                {#if d.repos.length > 1}
                  <input
                    class="field mono folder"
                    aria-label={t('teamProjects.admin.repoFolder', { n: i + 1 })}
                    spellcheck="false"
                    bind:value={r.folder}
                  />
                  <button class="btn ghost" aria-label={t('teamProjects.admin.repoRemove')} onclick={() => d.repos.splice(i, 1)}>×</button>
                {/if}
              </div>
            {/each}
            <button class="btn ghost add" onclick={() => d.repos.push({ url: '', folder: '' })}>{t('teamProjects.admin.repoAdd')}</button>
          </div>
          <label class="fld"
            ><span>{t('teamProjects.admin.branch')}</span><input
              class="field mono"
              bind:value={d.branch}
              spellcheck="false"
              placeholder={t('teamProjects.admin.branchHint')}
            /></label
          >
          <div class="two">
            <label class="fld"
              ><span>{t('teamProjects.admin.color')}</span>
              <span class="row">
                <input
                  type="color"
                  aria-label={t('teamProjects.admin.color')}
                  value={d.color || '#888888'}
                  oninput={(e) => (d.color = e.currentTarget.value)}
                />
                {#if d.color}<button class="btn ghost" onclick={() => (d.color = '')}>{t('teamProjects.admin.colorNone')}</button>{/if}
              </span>
            </label>
            <div class="fld">
              <span>{t('teamProjects.admin.logo')}</span>
              <span class="row">
                <button class="btn" onclick={chooseLogo}>{t('settings.project.logoChoose')}</button>
                {#if d.logo}<button class="btn ghost" onclick={() => (d.logo = null)}>{t('teamProjects.admin.logoNone')}</button>{/if}
              </span>
            </div>
          </div>
          <fieldset class="fld">
            <legend>{t('teamProjects.admin.access')}</legend>
            <label class="check"
              ><input type="radio" checked={d.access.kind === 'team'} onchange={() => (d.access = { kind: 'team' })} />{t(
                'teamProjects.admin.accessTeam',
              )}</label
            >
            <label class="check"
              ><input
                type="radio"
                checked={d.access.kind === 'members'}
                onchange={() => (d.access = { kind: 'members', members: view!.members.filter((m) => m.me).map((m) => m.id) })}
              />{t('teamProjects.admin.accessMembers')}</label
            >
            {#if d.access.kind === 'members'}
              {@const chosen = d.access.members}
              <div class="members">
                {#each view.members as m (m.id)}
                  <label class="check"
                    ><input type="checkbox" checked={chosen.includes(m.id)} onchange={(e) => toggleMember(m.id, e.currentTarget.checked)} />
                    {m.me ? t('teamProjects.admin.memberMe') : t('teamProjects.admin.member', { id: m.id.slice(0, 8) })}{m.admin
                      ? ` · ${t('teamProjects.admin.memberAdmin')}`
                      : ''}</label
                  >
                {/each}
              </div>
            {/if}
          </fieldset>
          <div class="acts">
            <button class="btn primary" disabled={busy || !d.name.trim()} onclick={save}>{t('teamProjects.admin.save')}</button>
            <button class="btn ghost" onclick={() => (editing = null)}>{t('teamProjects.admin.cancel')}</button>
          </div>
        </section>
      {:else}
        <div class="acts">
          <button class="btn" disabled={busy || view.alarm} onclick={() => (editing = { id: null, draft: draftOf(null) })}
            >{t('teamProjects.admin.add')}</button
          >
        </div>
      {/if}
    {/if}
    {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  {/if}

  <div class="acts end">
    <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
  </div>
</div>

<style>
  .projects {
    display: flex;
    flex-direction: column;
    gap: 14px;
    font-size: 13px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
  .advice {
    color: var(--warn-ink);
  }
  .note {
    color: var(--soft);
    font-style: italic;
  }
  .error {
    color: var(--del);
  }
  .alarm {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--del);
    background: color-mix(in oklch, var(--del) 10%, transparent);
  }
  .item,
  .confirm,
  .form {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .head {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .name {
    font-weight: 600;
    color: var(--text);
  }
  .tile {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    overflow: hidden;
    border-radius: 7px;
    color: #fff;
    font-size: 11px;
    font-weight: 650;
    background: var(--c);
    flex: none;
  }
  .tag {
    padding: 1px 7px;
    border-radius: 999px;
    font-size: 11px;
    color: var(--soft);
    background: var(--hover);
  }
  .tag.on {
    color: var(--run);
    background: color-mix(in oklch, var(--run) 12%, transparent);
  }
  .repos {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .repos li {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: baseline;
  }
  .url {
    color: var(--text);
    font-size: 12px;
    word-break: break-all;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .acts.end {
    justify-content: flex-end;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    border: 0;
  }
  .fld > span:first-child,
  legend {
    color: var(--soft);
    font-size: 12px;
    font-weight: 560;
    padding: 0;
  }
  .repo-row,
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .repo-row .field {
    flex: 1;
    min-width: 0;
  }
  .repo-row .folder {
    flex: 0 0 30%;
  }
  .add {
    align-self: flex-start;
  }
  .two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .check {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .members {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-left: 22px;
  }
</style>

<script lang="ts">
  // « Cloner » in the wizard « Nouveau projet » (TS-8): the repositories of a project of the team's list, each address
  // shown whole before anything runs; the clone by the member's own accounts on the host of code (the team server gives
  // no git access); a repository the member cannot reach said as « Demande l'accès à acme/api sur GitHub »; a project
  // of several repositories clones them all and says which one is missing. Once all are there, the wizard goes on with
  // the folder.
  import { open } from '@tauri-apps/plugin-dialog';
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { folderName, parentOf, teamProjectsApi, type CloneFrom, type CloneView } from '../../lib/team-projects';
  import { errorText } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';

  let { from, oncloned }: { from: CloneFrom; oncloned: (path: string) => void } = $props();

  const p = $derived(from.project);
  let parent = $state(parentOf(app.projects.map((x) => x.path)));
  let folder = $state(untrack(() => folderName(from.project)));
  let busy = $state(false);
  let failure = $state('');
  let result = $state<CloneView | null>(null);

  const missing = $derived(result ? result.repos.filter((r) => !r.done).length : 0);

  async function browse() {
    const dir = await open({ directory: true, multiple: false, title: t('teamProjects.clone.parent') });
    if (typeof dir === 'string') parent = dir;
  }

  async function go() {
    busy = true;
    failure = '';
    try {
      result = await teamProjectsApi.clone(from.team, p.id, parent.trim(), folder.trim());
      if (result.complete) oncloned(result.path);
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const stateOf = (url: string) => result?.repos.find((r) => r.url === url);
</script>

<section class="clone" aria-label={t('teamProjects.clone.title', { name: p.name, team: from.teamName })}>
  <h4>{t('teamProjects.clone.title', { name: p.name, team: from.teamName })}</h4>
  <p class="muted">{t('teamProjects.clone.lead')}</p>
  <ul class="repos" data-testid="clone-repos">
    {#each p.repos as r (r.url)}
      {@const st = stateOf(r.url)}
      <li>
        <span class="mono url">{r.url}</span>
        {#if p.repos.length > 1}<span class="muted small">{t('teamProjects.repo.folder', { folder: r.folder })}</span>{/if}
        {#if st?.done}
          <span class="ok"
            ><Icon name="check" size={12} />{st.already ? t('teamProjects.clone.already') : t('teamProjects.clone.done')}</span
          >
        {:else if st?.noAccess}
          <span class="ask" role="alert">{st.noAccess}</span>
        {:else if st?.error}
          <span class="error small" role="alert">{st.error}</span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if p.branch}<p class="muted small">{t('teamProjects.repo.branch', { branch: p.branch })}</p>{/if}
  <p class="note small"><Icon name="shield" size={12} />{t('teamProjects.clone.accounts')}</p>
  <div class="two">
    <label class="fld">
      <span>{t('teamProjects.clone.parent')}</span>
      <span class="row">
        <input class="field mono" bind:value={parent} spellcheck="false" />
        <button class="btn" onclick={browse}><Icon name="folder" size={14} />{t('shell.newProject.browse')}</button>
      </span>
    </label>
    <label class="fld">
      <span>{t('teamProjects.clone.folder')}</span>
      <input class="field mono" bind:value={folder} spellcheck="false" />
    </label>
  </div>
  {#if missing}<p class="advice small">{t('teamProjects.clone.missing', { count: missing })}</p>{/if}
  {#if failure}<p class="error small" role="alert">{failure}</p>{/if}
  <div class="row">
    <button class="btn primary" disabled={busy || !parent.trim() || !folder.trim()} onclick={go}>
      {busy ? t('teamProjects.clone.going') : result ? t('teamProjects.clone.again') : t('teamProjects.clone.go')}
    </button>
  </div>
</section>

<style>
  .clone {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 13px;
  }
  h4 {
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
  .note {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    color: var(--soft);
  }
  .advice {
    color: var(--warn-ink);
  }
  .error {
    color: var(--del);
  }
  .repos {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .repos li {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: baseline;
  }
  .url {
    color: var(--text);
    font-size: 12px;
    word-break: break-all;
  }
  .ok {
    display: inline-flex;
    gap: 4px;
    align-items: center;
    color: var(--run);
    font-size: 12px;
  }
  .ask {
    color: var(--warn-ink);
    font-size: 12px;
    font-weight: 560;
  }
  .two {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 10px;
  }
  .fld {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fld > span:first-child {
    color: var(--soft);
    font-size: 12px;
    font-weight: 560;
  }
  .row {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .row .field {
    flex: 1;
    min-width: 0;
  }
</style>

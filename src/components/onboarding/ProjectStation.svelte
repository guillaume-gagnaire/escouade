<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onMount, untrack } from 'svelte';
  import { cloneName, cloneParent, hostRepos, ymlList } from '../../lib/first-project';
  import { tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { onboarding, type Primary } from '../../lib/onboarding.svelte';
  import { projectPalette } from '../../lib/plugin-theme.svelte';
  import { app } from '../../lib/state.svelte';
  import { teams } from '../../lib/teams.svelte';
  import type { FolderCheck, FolderInfo, Project } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // Station 4, the first project (ESC-505, docs/ONBOARDING.md § 4.4, direction C): where the first agent works, one
  // question, three answers (four when a team shares its projects): a folder of the machine (recommended, choice O4), a
  // clone, the demonstration project. What a project, an agent, a worktree and a branch are, each in a line, beside a
  // drawing of the branches. Nothing is cloned, written nor opened before the main button; each answer says what it
  // finds first (a folder that is no repository, its .escouade.yml, a folder Escouade cannot write to).
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  type Way = 'team' | 'folder' | 'clone' | 'demo';
  const team = $derived(teams.list.find((x) => x.joined) ?? null);
  const items = $derived<{ id: Way }[]>([...(team ? [{ id: 'team' as const }] : []), { id: 'folder' }, { id: 'clone' }, { id: 'demo' }]);
  let way = $state<string | null>('folder');

  /** The project of the station, once opened here or elsewhere. */
  const opened = $derived((onboarding.project ? app.projects.find((p) => p.id === onboarding.project) : null) ?? app.projects[0] ?? null);
  let busy = $state(false);

  onMount(() => {
    void teams.load().then(() => {
      // An invitation followed (P4): the team's projects first.
      if (untrack(() => team)) way = 'team';
    });
  });

  // === A folder of the machine ===
  let path = $state('');
  let info = $state<FolderInfo | null>(null);
  let check = $state<FolderCheck | null>(null);
  let yml = $state<string[]>([]);
  let commit = $state(true);
  /** The folder `info` and `check` were read for. */
  let readFor = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const p = path.trim();
    clearTimeout(timer);
    if (!p) {
      info = null;
      check = null;
      yml = [];
      readFor = '';
      return;
    }
    timer = setTimeout(async () => {
      const [i, c, shared] = await Promise.all([
        api.inspectFolder(p).catch(() => null),
        api.onboardingFolder(p).catch(() => null),
        api.sharedPreview(p).catch(() => null),
      ]);
      if (path.trim() !== p) return;
      info = i;
      check = c;
      yml = shared?.shared?.file?.keys ?? [];
      readFor = p;
    }, 200);
  });
  const folderReady = $derived(readFor === path.trim() && !!check?.exists && !!check?.writable);

  async function browse() {
    const dir = await open({ directory: true, multiple: false, title: t('shell.newProject.folderTitle') });
    if (typeof dir === 'string') path = dir;
  }

  const used = $derived(new Set(app.projects.map((p) => p.color)));
  const color = $derived(projectPalette().find((c) => !used.has(c)) ?? projectPalette()[0]);

  /** The project made from `dir`, shown in the app's list, kept as the station's. */
  function take(p: Project) {
    if (app.projects.some((x) => x.id === p.id)) app.replaceProject(p);
    else app.projects.push(p);
    onboarding.project = p.id;
  }

  async function openFolder(dir: string, makeCommit: boolean): Promise<boolean> {
    const name =
      dir
        .replace(/[\\/]+$/, '')
        .split(/[\\/]/)
        .pop() ?? '';
    const p = await app.run(api.createProject({ path: dir, name, color, worktreePerAgent: true, firstAgent: null, restore: false }));
    if (!p) return false;
    // A folder with no commit: its first one, as the assistant proposes it (an agent's worktree starts from a commit).
    if (makeCommit) await app.run(api.firstCommit(dir));
    take(p);
    return true;
  }

  // === A clone ===
  let url = $state('');
  let parent = $state(
    untrack(() =>
      cloneParent(
        app.projects.map((p) => p.path),
        onboarding.view?.demo ?? '',
      ),
    ),
  );
  let folder = $state('');
  let folderTouched = false;
  $effect(() => {
    const name = cloneName(url);
    if (!folderTouched) folder = name;
  });
  let failure = $state<{ kind: string; text: string } | null>(null);
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine);
  let hosts = $state<{ host: string; urls: string[] }[]>([]);
  onMount(() => {
    const on = () => (online = navigator.onLine);
    window.addEventListener('online', on);
    window.addEventListener('offline', on);
    void hostRepos().then((h) => (hosts = h));
    return () => {
      window.removeEventListener('online', on);
      window.removeEventListener('offline', on);
    };
  });

  async function browseParent() {
    const dir = await open({ directory: true, multiple: false, title: t('onboarding.project.cloneParent') });
    if (typeof dir === 'string') parent = dir;
  }

  async function clone(): Promise<boolean> {
    failure = null;
    const r = await api.onboardingClone(url.trim(), parent.trim(), folder.trim()).catch((e) => ({
      path: null,
      failure: { kind: 'error' as const, detail: String(e) },
    }));
    if (r.path) return openFolder(r.path, false);
    const f = r.failure;
    if (!f) return false;
    failure = {
      kind: f.kind,
      text:
        f.kind === 'offline'
          ? t('onboarding.project.cloneOffline')
          : f.kind === 'access' || f.kind === 'error'
            ? t('onboarding.project.cloneFailed', { error: f.detail.replace(/[.\s]+$/, '') })
            : f.detail,
    };
    return false;
  }

  // === The demonstration ===
  const demoPath = $derived(onboarding.view?.demo ?? '');
  async function demo(): Promise<boolean> {
    const p = await app.run(api.demoCreate(color));
    if (!p) return false;
    take(p);
    return true;
  }

  // The main button: what the answer chosen does, then the next station.
  $effect(() => {
    if (opened && !redoing()) {
      primary = { label: t('onboarding.continue'), run: async () => true };
      return;
    }
    const run = async (f: () => Promise<boolean>) => {
      busy = true;
      try {
        return await f();
      } finally {
        busy = false;
      }
    };
    // The answer recommended: a folder (O4), or the team's projects on a device that joined one (the funnel, ESC-507).
    const choice = way === (team ? 'team' : 'folder') ? 'recommended' : 'other';
    if (way === 'folder')
      primary = {
        choice,
        label: t('onboarding.project.open'),
        disabled: busy || !folderReady,
        run: () => run(() => openFolder(path.trim(), !info?.isRepo && commit)),
      };
    else if (way === 'clone')
      primary = {
        choice,
        label: busy ? t('onboarding.project.cloning') : failure ? t('onboarding.project.retry') : t('onboarding.project.cloneGo'),
        disabled: busy || !online || !url.trim() || !parent.trim() || !folder.trim(),
        run: () => run(clone),
      };
    else if (way === 'demo')
      primary = {
        choice,
        label: busy ? t('onboarding.project.creating') : t('onboarding.project.demoGo'),
        disabled: busy,
        run: () => run(demo),
      };
    else
      primary = {
        label: t('onboarding.project.teamGo', { team: team?.name ?? '' }),
        run: async () => {
          if (team) app.modal = { kind: 'team', step: 'projects', teamId: team.id };
          return false;
        },
      };
  });
  /** Done again from Aide: the station offers its answers again, the project in place said above them. */
  function redoing() {
    return onboarding.view?.trigger === 'redo';
  }

  const WORDS = ['project', 'agent', 'worktree', 'branch'] as const;
</script>

<div class="q" data-testid="onboarding-project">
  <StationHead step="project" question={t('onboarding.project.question')}>
    {#snippet why()}{t('onboarding.project.why')}{/snippet}
  </StationHead>
  <div class="learn">
    <!-- The branches, as the git graph draws them: decorative, their meaning is in the words beside. -->
    <svg class="schema" viewBox="0 0 300 74" aria-hidden="true">
      <path d="M8 14 H292" style:stroke="var(--lane-main)" />
      <path d="M40 14 C 62 14, 62 42, 86 42 H196" style:stroke="var(--lane-1)" />
      <path d="M120 14 C 142 14, 142 64, 166 64 H226" style:stroke="var(--lane-2)" />
      <circle cx="24" cy="14" r="4" style:fill="var(--lane-main)" />
      <circle cx="196" cy="42" r="4" style:fill="var(--lane-1)" />
      <circle cx="226" cy="64" r="4" style:fill="var(--lane-2)" />
      <text x="30" y="8">{t('onboarding.project.schema.main')}</text>
      <text x="204" y="45">{t('onboarding.project.schema.agent')}</text>
      <text x="234" y="67">{t('onboarding.project.schema.agent2')}</text>
    </svg>
    <dl class="words" aria-label={t('onboarding.project.words.label')}>
      {#each WORDS as w (w)}
        <div>
          <dt>{t(`onboarding.project.words.${w}.term`)}{t('onboarding.project.words.sep')}</dt>
          <dd>{t(`onboarding.project.words.${w}.text`)}</dd>
        </div>
      {/each}
    </dl>
  </div>
  {#if opened && !redoing()}
    <p class="done" data-testid="onboarding-project-open">
      <Icon name="check" size={14} />{t('onboarding.project.opened', { name: opened.name })}
    </p>
  {:else}
    {#if opened}<p class="done"><Icon name="check" size={14} />{t('onboarding.project.opened', { name: opened.name })}</p>{/if}
    <Choices {items} bind:value={way} label={t('onboarding.project.question')}>
      {#snippet head(it: { id: Way })}
        <span class="t">
          <Icon name={it.id === 'folder' ? 'folder' : it.id === 'clone' ? 'download' : it.id === 'demo' ? 'sparkle' : 'team'} size={14} />
          <b>{t(`onboarding.project.${it.id}`)}</b>
          {#if it.id === 'folder' && !team}<span class="rec">{t('settings.permissions.recommended')}</span>{/if}
        </span>
        <span class="d"
          >{it.id === 'team' ? t('onboarding.project.teamHint', { team: team?.name ?? '' }) : t(`onboarding.project.${it.id}Hint`)}</span
        >
      {/snippet}
      {#snippet extra(it: { id: Way }, on: boolean)}
        {#if on && it.id === 'folder'}
          <div class="x">
            <span class="field-row">
              <input
                class="field mono"
                bind:value={path}
                spellcheck="false"
                aria-label={t('onboarding.project.folderLabel')}
                placeholder={t('onboarding.project.folderPlaceholder')}
              />
              <button class="btn" onclick={browse}>{t('onboarding.project.browse')}</button>
            </span>
            {#if readFor && readFor === path.trim()}
              {#if !check?.exists}
                <p class="bad" role="alert">{t('onboarding.project.missing')}</p>
              {:else if !check.writable}
                <p class="bad" role="alert">{t('onboarding.project.notWritable', { path: tildify(readFor) })}</p>
              {:else}
                {#if info && !info.isRepo && !info.repos?.length}
                  <p class="note">{t('onboarding.project.noGit')}</p>
                  <label class="tick"><input type="checkbox" bind:checked={commit} />{t('onboarding.project.firstCommit')}</label>
                {/if}
                {#if yml.length}
                  <p class="note" data-testid="onboarding-yml">{t('onboarding.project.yml', { list: ymlList(yml) })}</p>
                {/if}
              {/if}
            {/if}
          </div>
        {:else if on && it.id === 'clone'}
          <div class="x">
            {#if !online}<p class="bad" role="alert">{t('onboarding.project.cloneOffline')}</p>{/if}
            <input
              class="field mono"
              bind:value={url}
              spellcheck="false"
              list="onb-repos"
              aria-label={t('onboarding.project.cloneUrl')}
              placeholder={t('onboarding.project.clonePlaceholder')}
            />
            <!-- Always there, so the field is the same combobox with or without a code host connected. -->
            <datalist id="onb-repos">
              {#each hosts as h (h.host)}
                {#each h.urls as u (u)}<option value={u}>{t('onboarding.project.cloneHosts', { host: h.host })}</option>{/each}
              {/each}
            </datalist>
            <span class="two">
              <label class="fld">
                <span>{t('onboarding.project.cloneParent')}</span>
                <span class="field-row">
                  <input class="field mono" bind:value={parent} spellcheck="false" aria-label={t('onboarding.project.cloneParent')} />
                  <button class="btn" onclick={browseParent}>{t('onboarding.project.browse')}</button>
                </span>
              </label>
              <label class="fld">
                <span>{t('onboarding.project.cloneFolder')}</span>
                <input class="field mono" bind:value={folder} oninput={() => (folderTouched = true)} spellcheck="false" />
              </label>
            </span>
            {#if failure}<p class="bad" role="alert" data-testid="onboarding-clone-failed">{failure.text}</p>{/if}
          </div>
        {:else if on && it.id === 'demo'}
          <div class="x">
            <p class="note">{t('onboarding.project.demoWhere', { path: tildify(demoPath) })}</p>
          </div>
        {/if}
      {/snippet}
    </Choices>
  {/if}
</div>

<style>
  .q {
    display: grid;
    gap: 18px;
    width: min(680px, 100%);
  }
  .learn {
    display: grid;
    grid-template-columns: minmax(0, 300px) minmax(0, 1fr);
    gap: 18px;
    align-items: center;
  }
  .schema {
    width: 100%;
    height: auto;
    overflow: visible;
  }
  .schema path {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .schema text {
    fill: var(--muted);
    font-size: 9.5px;
  }
  .words {
    display: grid;
    gap: 5px;
    margin: 0;
    font-size: 12.5px;
    line-height: 1.4;
  }
  .words div {
    display: block;
  }
  .words dt {
    display: inline;
    color: var(--text);
    font-weight: 600;
  }
  .words dd {
    display: inline;
    margin: 0;
    color: var(--muted);
  }
  .t {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    color: var(--soft);
  }
  .t b {
    color: var(--text);
    font-weight: 620;
  }
  .rec {
    padding: 1px 8px;
    border-radius: 999px;
    color: var(--teal-ink);
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 45%, transparent);
    background: color-mix(in oklch, var(--brand) 8%, transparent);
  }
  .d,
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .bad {
    margin: 0;
    color: var(--del);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .x {
    display: grid;
    gap: 8px;
    padding: 0 14px 12px 42px;
  }
  .field-row {
    display: flex;
    gap: 8px;
    min-width: 0;
  }
  .field-row .field {
    flex: 1;
    min-width: 0;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
    gap: 8px;
  }
  .fld {
    display: grid;
    gap: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .tick {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
    font-size: 12.5px;
  }
  .done {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--soft);
  }
  .done :global(.icon) {
    color: var(--run);
  }
  @media (max-width: 1100px) {
    .q {
      gap: 14px;
    }
    .learn {
      grid-template-columns: minmax(0, 220px) minmax(0, 1fr);
      gap: 14px;
    }
    .words {
      font-size: 12px;
    }
  }
</style>

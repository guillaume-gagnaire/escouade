<script lang="ts">
  import { askCommit, canCommit, integrateBase, mergeAgent } from '../lib/agent-actions';
  import { basename, dirname, fInt } from '../lib/format';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { app } from '../lib/state.svelte';
  import type { Agent, FileChange, Project } from '../lib/types';
  import FileDiff from './FileDiff.svelte';
  import Icon from './Icon.svelte';
  import { tip } from '../lib/tip';
  import { worktreeLabel } from '../lib/worktrees';
  import { agentRepos, groupByRepo, inRepo, isMulti, quietRepos, repoLabel } from '../lib/repos';
  import { SHARED_DIR, isTicketPath, ticketOfPath } from '../lib/sharing';

  // The side panel's uncommitted files. `docked`: right half of the split layout, showing the
  // picked file's diff under the list. `count`: number of files listed, for the panel's tab.
  let {
    project,
    agent,
    docked = false,
    count = $bindable(0),
  }: { project: Project; agent: Agent | null; docked?: boolean; count?: number } = $props();

  let files = $state<FileChange[]>([]);
  // Rows drawn: a build folder or a mass rename changes thousands of files, and a row is a few DOM
  // nodes. The count, the diff and the commit still cover them all.
  const SHOWN = 500;
  const listed = $derived(files.slice(0, SHOWN));
  // Once a list is read: shown again (its tab chosen anew), the panel does not say 0 while it reads it.
  $effect(() => {
    if (listFor !== null) count = files.length;
  });
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const scope = $derived(app.filesScope);
  // The agent object changes on every backend update: refetch on id / scope / git changes only.
  const agentId = $derived(agent?.id ?? null);
  const projectId = $derived(project.id);

  // Which list `files` holds: until the list of a newly selected agent / scope arrives, the old one
  // must not be paired with the new owner to load a diff.
  const listKey = (pid: string, aid: string | null, agentScope: boolean) => `${pid}|${agentScope ? aid : '*'}`;
  let listFor = $state<string | null>(null);
  const wanted = $derived(listKey(projectId, agentId, scope === 'agent'));

  $effect(() => {
    void app.gitTick;
    const pid = projectId;
    const aid = scope === 'agent' ? agentId : null;
    const agentScope = scope === 'agent';
    clearTimeout(timer);
    timer = setTimeout(() => load(pid, aid, agentScope), 120);
    return () => clearTimeout(timer);
  });

  async function load(pid: string, aid: string | null, agentScope: boolean) {
    const mine = ++seq;
    if (agentScope && !aid) {
      setFiles([], listKey(pid, aid, agentScope));
      return;
    }
    // No loading flag: a re-read keeps what is shown until its answer, so nothing blinks.
    try {
      const result = await api.gitFiles(pid, aid);
      if (mine !== seq) return; // superseded by a newer request
      setFiles(result, listKey(pid, aid, agentScope));
      error = null;
    } catch (e) {
      if (mine !== seq) return;
      error = String(e);
      setFiles([], listKey(pid, aid, agentScope));
    }
  }

  function setFiles(list: FileChange[], key: string) {
    // The same list again: leave `files` alone so nothing downstream re-renders.
    if (JSON.stringify(list) !== JSON.stringify(files)) files = list;
    listFor = key;
    // Pin the file shown (the first one by default): a file sorting before it must not take its place.
    if (!list.some((f) => keyOf(f) === picked)) picked = list[0] ? keyOf(list[0]) : null;
  }

  /** Where the agent's worktree is, said around its path (in mono). */
  const worktree = $derived(scope === 'agent' && agent?.worktree ? worktreeLabel(project.path, agent.worktree.path) : null);
  const around = $derived(worktree === null ? null : t('git.files.hintWorktree', { path: '\u0000' }).split('\u0000'));
  const hint = $derived(scope === 'agent' ? t('git.files.hintAgent') : t('git.files.hintProject'));

  /** « Merger <b>branch</b> → <b>base</b>… », its two branches apart from the words around them. */
  function mergeParts(branch: string, base: string) {
    const [a, b, c] = t('git.files.merge', { branch: '\u0000', base: '\u0001' }).split(/[\u0000\u0001]/);
    return [
      { text: a, ref: false },
      { text: branch, ref: true },
      { text: b, ref: false },
      { text: base, ref: true },
      { text: c, ref: false },
    ];
  }

  function agentName(id: string | null) {
    return id ? (app.agents[id]?.name ?? '?') : '';
  }

  function openDiff(paths: string[], agentId: string | null, title: string) {
    app.modal = { kind: 'diff', projectId: project.id, agentId, paths, title };
  }

  /** « Voir le diff » of the footer: the listed files, which for the whole project are in several
   *  checkouts (the project's, and each agent's worktree), so the backend reads them all. */
  function openListedDiff() {
    if (scope === 'project') {
      app.modal = {
        kind: 'diff',
        projectId: project.id,
        agentId: null,
        paths: [],
        title: t('git.files.changesOf', { name: project.name }),
        wholeProject: true,
      };
    } else {
      openDiff(
        agent && !agent.worktree ? files.map((f) => f.path) : [],
        agent?.id ?? null,
        t('git.files.changesOf', { name: agent?.name ?? '' }),
      );
    }
  }

  const keyOf = (f: FileChange) => f.agentId + ':' + f.path;

  /** The agent whose worktree holds `f` (null: the project checkout), as listed: a row still
   *  shown while another agent's list loads keeps pointing at its own checkout. */
  function diffOwner(f: FileChange) {
    return f.inWorktree ? f.agentId : null;
  }

  /** Shows `f` in the editor, on the checkout it comes from. */
  function openInEditor(f: FileChange) {
    app.openEditor({ projectId: project.id, source: diffOwner(f) ?? 'project', path: f.path });
  }

  function fileMenu(e: MouseEvent, f: FileChange) {
    menu.show(e, [
      // A deleted file has nothing to open.
      { label: t('common.openInEditor'), onClick: () => openInEditor(f), disabled: f.status === 'D' },
      { label: '', separator: true },
      discardItem(f, diffOwner(f)),
    ]);
  }

  function discardItem(f: FileChange, owner: string | null): MenuItem {
    const discard = () => app.run(api.gitDiscard(project.id, owner, f.path));
    if (f.status === 'D') return { label: t('git.files.restore'), onClick: discard };
    const isNew = f.status === 'A';
    const name = basename(f.path);
    return {
      label: isNew ? t('git.files.deleteFile') : t('git.files.discardChanges'),
      danger: true,
      onClick: () => {
        app.modal = {
          kind: 'confirm',
          title: isNew ? t('git.files.deleteTitle', { name }) : t('git.files.discardTitle', { name }),
          body: isNew ? t('git.files.deleteBody', { path: f.path }) : t('git.files.discardBody', { path: f.path }),
          confirm: isNew ? t('common.delete') : t('git.files.discardConfirm'),
          danger: true,
          onConfirm: discard,
        };
      },
    };
  }

  // Of several repositories (ESC-169): the list grouped by repository, those without a change on one line.
  const multi = $derived(isMulti(project));
  const repos = $derived(scope === 'agent' ? agentRepos(project, agent) : (project.repos ?? []));
  const groups = $derived(multi ? groupByRepo(listed, repos) : []);
  /**
   * The shared tickets' files Escouade wrote in the project's folder (ESC-190, choice 120), at the head of the list: each by
   * its ticket's key and title. Written by Escouade, committed by the user, never by Escouade.
   */
  const ticketRows = $derived(multi ? [] : listed.filter((x) => isTicketPath(x.path)));
  const otherRows = $derived(multi ? listed : listed.filter((x) => !isTicketPath(x.path)));
  const projectTickets = $derived(Object.values(app.tickets).filter((x) => x.projectId === project.id));
  let ticketsFolded = $state(false);
  const quiet = $derived(multi && listFor !== null ? quietRepos(repos, groupByRepo(files, repos)) : []);
  let folded = $state<Record<string, boolean>>({});
  /** The branch a repository's group is on: the agent's, else the repository's own. */
  function branchOf(repo: string): string {
    if (scope === 'agent' && agent?.worktree) return agent.worktree.branch;
    return app.git[project.id]?.repos?.find((r) => r.repo === repo)?.branch ?? '';
  }
  /** « Merger dans api et front… »: the repositories its branch changed, as the list shows them. */
  const mergeInto = $derived(
    groups.length ? t('repos.files.merge', { repos: groups.map((g) => repoLabel(g.repo)).join(', ') }) : t('repos.files.mergeNone'),
  );
  const multiCaption = $derived(
    !multi
      ? ''
      : scope === 'agent' && agent?.worktree
        ? t('repos.files.folder', { path: worktreeLabel(project.path, agent.worktree.path), count: repos.length })
        : t('repos.files.project', { count: repos.length }),
  );

  let picked = $state<string | null>(null);
  // The picked file (kept listed by setFiles), once the list matches the selected agent and scope.
  const current = $derived(docked && listFor === wanted ? (files.find((f) => keyOf(f) === picked) ?? null) : null);
  // Primitives, so that a list refresh returning the same file does not reload its diff twice.
  const currentPath = $derived(current?.path ?? null);
  const currentOwner = $derived(current ? diffOwner(current) : null);
</script>

<div class="files" class:docked>
  <div class="side-top">
    <div class="segmented small scope" role="group" aria-label={t('git.files.scope')}>
      <button aria-pressed={scope === 'agent'} onclick={() => (app.filesScope = 'agent')}>{t('git.files.scopeAgent')}</button>
      <button aria-pressed={scope === 'project'} onclick={() => (app.filesScope = 'project')}>{t('git.files.scopeProject')}</button>
    </div>
    {#if multi}
      <div class="caption" use:tip={{ text: multiCaption, cut: true }}>{multiCaption}</div>
    {:else}
      <div class="caption" use:tip={{ text: around ? around.join(worktree ?? '') : hint, cut: true }}>
        {#if around}{around[0]}<span class="mono">{worktree}</span>{around[1]}{:else}{hint}{/if}
      </div>
    {/if}
  </div>
  <div class="list">
    {#if error}
      <div class="empty">{error}</div>
    {:else if !files.length && listFor === wanted}
      <div class="empty">{scope === 'agent' ? t('git.files.emptyAgent') : t('git.files.emptyProject')}</div>
    {/if}
    {#if multi}
      {#each groups as g (g.repo)}
        {@const open = !folded[g.repo]}
        <button
          class="repo-h"
          aria-expanded={open}
          aria-label={t(open ? 'repos.files.fold' : 'repos.files.unfold', { repo: repoLabel(g.repo) })}
          onclick={() => (folded[g.repo] = open)}
          ><Icon name="down" size={14} /><span class="rn">{repoLabel(g.repo)}</span><span class="rb">{branchOf(g.repo)}</span><span
            class="cnt"><span class="add">+{g.add}</span><span class="del">−{g.del}</span></span
          ></button
        >
        {#if open}
          {#each g.files as f (keyOf(f))}{@render row(f, g.repo)}{/each}
        {/if}
      {/each}
      {#if quiet.length}<p class="quiet">{t('repos.files.quiet', { repos: quiet.map(repoLabel).join(', ') })}</p>{/if}
    {:else}
      {#if ticketRows.length}
        <button class="repo-h tickets-h" aria-expanded={!ticketsFolded} onclick={() => (ticketsFolded = !ticketsFolded)}
          ><Icon name="down" size={14} /><span class="rn sans">{t('share.git.group')}</span><span class="rb">{SHARED_DIR}</span><span
            class="cnt">{ticketRows.length}</span
          ></button
        >
        {#if !ticketsFolded}
          {#each ticketRows as f (keyOf(f))}{@render row(f, null)}{/each}
          <p class="tnote">{t('share.git.note')}</p>
        {/if}
      {/if}
      {#each otherRows as f (keyOf(f))}{@render row(f, null)}{/each}
    {/if}
    {#snippet row(f: FileChange, repo: string | null)}
      {@const on = current !== null && keyOf(current) === keyOf(f)}
      {@const shown = repo === null ? f.path : inRepo(f.path, repo)}
      {@const tk = repo === null && f.path.endsWith('.yml') && isTicketPath(f.path) ? ticketOfPath(f.path, projectTickets) : undefined}
      <div class="filerow" class:on class:in={repo !== null}>
        <button
          class="file"
          aria-current={on ? 'true' : undefined}
          onclick={() => (docked ? (picked = keyOf(f)) : openDiff([f.path], diffOwner(f), f.path))}
          oncontextmenu={(e) => fileMenu(e, f)}
        >
          <span class="letter {f.status.toLowerCase()}">{f.status}</span>
          {#if tk}
            <span class="fn" use:tip={f.path}><b><span class="tkey">{tk.key.split('@')[0]}</span>{tk.title}</b></span>
          {:else}
            <span class="fn"
              ><b>{basename(shown)}</b>{#if dirname(shown)}<span class="dir">{dirname(shown)}</span>{/if}</span
            >
          {/if}
          {#if scope === 'project' && f.agentId}<span class="tag">{agentName(f.agentId)}</span>{/if}
          <span class="cnt"><span class="add">+{f.add}</span><span class="del">−{f.del}</span></span>
        </button>
        <!-- A sibling of the row's button, not inside it. -->
        {#if f.status !== 'D'}
          <button
            class="edit icon-btn"
            aria-label={t('git.files.openNamed', { name: basename(f.path) })}
            use:tip={t('common.openInEditor')}
            onclick={() => openInEditor(f)}
            oncontextmenu={(e) => fileMenu(e, f)}><Icon name="code" size={14} /></button
          >
        {/if}
      </div>
    {/snippet}
    {#if files.length > SHOWN}<p class="more">
        {t('git.files.more', { count: files.length - SHOWN, n: fInt(files.length - SHOWN) })}
      </p>{/if}
  </div>
  {#if docked}
    {#if currentPath !== null}
      {#key currentOwner + ':' + currentPath}
        <FileDiff projectId={project.id} agentId={currentOwner} path={currentPath} />
      {/key}
    {:else}
      <div class="fill"></div>
    {/if}
  {/if}
  <div class="foot">
    <div class="rowb">
      <button class="btn" disabled={!files.length} onclick={openListedDiff}>{t('git.files.showDiff')}</button>
      <button
        class="btn primary"
        disabled={!canCommit(project, agent, scope) || !files.length}
        onclick={() => askCommit(project, agent, scope)}
      >
        {scope === 'agent' ? t('git.files.commitAgent') : t('git.files.commitAll')}
      </button>
    </div>
    {#if scope === 'agent' && agent?.worktree}
      {@const wt = agent.worktree}
      {@const merge = multi ? mergeInto : t('git.files.merge', { branch: wt.branch, base: wt.baseBranch })}
      <div class="rowb">
        <button class="btn wide" aria-label={merge} use:tip={{ text: merge, cut: true }} onclick={() => agent && mergeAgent(agent)}
          ><Icon name="merge" size={14} /><span class="ell"
            >{#if multi}{merge}{:else}{#each mergeParts(wt.branch, wt.baseBranch) as p, i (i)}{#if p.ref}<b>{p.text}</b
                  >{:else}{p.text}{/if}{/each}{/if}</span
          ></button
        >
        <!-- Not for an archived agent, as in the sidebar's menu: nothing goes into the worktree of one that is not running. -->
        {#if !agent.archived}
          <button
            class="btn icon-only"
            aria-label={t('branches.integrate.menu', { base: wt.baseBranch })}
            use:tip={t('branches.integrate.menu', { base: wt.baseBranch })}
            onclick={() => agent && integrateBase(agent)}><Icon name="pull" size={14} /></button
          >
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .files {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .docked .list {
    flex: 0 1 auto;
    max-height: 35%;
  }
  .fill {
    flex: 1;
  }
  .side-top {
    display: grid;
    gap: 8px;
    padding: 12px 14px 8px;
  }
  /* The scope in two equal segments, as the preview draws them. */
  .scope {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0;
    padding: 3px;
    border-radius: 9px;
  }
  .scope button {
    padding: 0;
    border-radius: 7px;
  }
  .caption {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .caption .mono {
    font-size: 11px;
  }
  .list {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-auto-rows: max-content;
    align-content: start;
    gap: 1px;
    padding: 0 8px;
  }
  .empty {
    padding: 30px 10px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  .more {
    margin: 0;
    padding: 8px;
    font-size: 11.5px;
    color: var(--dim);
  }
  .filerow {
    position: relative;
    display: flex;
    min-width: 0;
  }
  /* A repository's group (ESC-169): its name, its branch, its counts; its files under it. */
  .repo-h {
    display: grid;
    grid-template-columns: 14px auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin-top: 4px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  /* The shared tickets' group (ESC-190): named in the interface's font, its keys in the project's ink. */
  .repo-h .rn.sans {
    font-family: var(--font);
  }
  .tickets-h .cnt {
    color: var(--muted);
  }
  .tkey {
    margin-right: 6px;
    color: var(--p-ink);
    font: 600 11px var(--mono);
  }
  .tnote {
    margin: 2px 12px 8px 34px;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .repo-h:hover {
    background: var(--hover);
  }
  .repo-h :global(svg) {
    color: var(--muted);
    transition: transform 0.2s var(--ease);
  }
  .repo-h[aria-expanded='false'] :global(svg) {
    transform: rotate(-90deg);
  }
  .repo-h .rn {
    color: var(--text);
    font: 600 12px var(--mono);
  }
  .repo-h .rb {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font: 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .filerow.in .file {
    padding-left: 30px;
  }
  .quiet {
    margin: 0;
    padding: 10px 8px 4px;
    color: var(--muted);
    font-size: 11.5px;
  }
  /* A file: its letter, its name and its folder, its counts. */
  .file {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 8px;
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .file:hover {
    background: var(--hover);
  }
  .file[aria-current='true'] {
    background: rgba(var(--fg-rgb), 0.06);
  }
  .letter {
    font: 600 11.5px var(--mono);
    text-align: center;
    color: var(--git-m);
  }
  .letter.a {
    color: var(--add);
  }
  .letter.d {
    color: var(--del);
  }
  .fn {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fn b {
    color: var(--text);
    font-weight: 550;
  }
  .fn .dir {
    margin-left: 6px;
    color: var(--muted);
  }
  /* In the whole project: the agent whose worktree has the file. */
  .tag {
    max-width: 96px;
    overflow: hidden;
    padding: 0 6px;
    border-radius: 5px;
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 11px;
    line-height: 18px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cnt {
    display: inline-flex;
    gap: 6px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  /* In place of the row's counts, once the row is pointed at or reached with the keyboard. */
  .edit {
    position: absolute;
    right: 4px;
    top: 50%;
    width: 24px;
    height: 24px;
    transform: translateY(-50%);
    opacity: 0;
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--p-ink);
  }
  .filerow:hover .edit,
  .filerow:has(:focus-visible) .edit {
    opacity: 1;
  }
  .filerow:hover .cnt,
  .filerow:has(:focus-visible) .cnt {
    visibility: hidden;
  }
  .foot {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    padding: 12px 14px 14px;
    border-top: 1px solid var(--line);
  }
  .rowb {
    display: flex;
    gap: 8px;
    min-width: 0;
  }
  .rowb .btn.primary {
    margin-left: auto;
  }
  .btn.wide {
    flex: 1;
    min-width: 0;
  }
  .btn.wide .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .btn.wide b {
    font-family: var(--mono);
    font-size: 12px;
    font-weight: 500;
  }
</style>

<script lang="ts">
  import { askCommit, canCommit, mergeAgent } from '../lib/agent-actions';
  import { basename, dirname, plural } from '../lib/format';
  import { api } from '../lib/ipc';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { app } from '../lib/state.svelte';
  import type { Agent, FileChange, Project } from '../lib/types';
  import FileDiff from './FileDiff.svelte';

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
  $effect(() => {
    count = files.length;
  });
  let loading = $state(false);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let seq = 0;

  const scope = $derived(app.filesScope);
  // The agent object changes on every backend update: refetch on id / scope / git changes only.
  const agentId = $derived(agent?.id ?? null);
  const projectId = $derived(project.id);
  const SC: Record<string, string> = { A: 'var(--add)', M: 'var(--wait)', D: 'var(--del)' };

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
    loading = true;
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
    loading = false;
  }

  function setFiles(list: FileChange[], key: string) {
    files = list;
    listFor = key;
    // Pin the file shown (the first one by default): a file sorting before it must not take its place.
    if (!list.some((f) => keyOf(f) === picked)) picked = list[0] ? keyOf(list[0]) : null;
  }

  const hint = $derived(
    scope === 'agent'
      ? agent?.worktree
        ? `worktree ${agent.worktree.path.replace(/\\/g, '/').split('/.claude/')[1] ? '.claude/' + agent.worktree.path.replace(/\\/g, '/').split('/.claude/')[1] : agent.worktree.path} · isolé des autres agents`
        : 'Fichiers modifiés par cet agent (d’après ses éditions)'
      : 'Tous les agents du projet · attribution par worktree',
  );

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
        title: `Modifications de ${project.name}`,
        wholeProject: true,
      };
    } else {
      openDiff(agent && !agent.worktree ? files.map((f) => f.path) : [], agent?.id ?? null, `Modifications de ${agent?.name}`);
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
      { label: 'Ouvrir dans l’éditeur', onClick: () => openInEditor(f), disabled: f.status === 'D' },
      { label: '', separator: true },
      discardItem(f, diffOwner(f)),
    ]);
  }

  function discardItem(f: FileChange, owner: string | null): MenuItem {
    const discard = () => app.run(api.gitDiscard(project.id, owner, f.path));
    if (f.status === 'D') return { label: 'Restaurer le fichier', onClick: discard };
    const isNew = f.status === 'A';
    const name = basename(f.path);
    return {
      label: isNew ? 'Supprimer le fichier…' : 'Abandonner les modifications…',
      danger: true,
      onClick: () => {
        app.modal = {
          kind: 'confirm',
          title: isNew ? `Supprimer « ${name} » ?` : `Abandonner les modifications de « ${name} » ?`,
          body: isNew
            ? `${f.path} n’a jamais été commité : il est supprimé du disque, sans retour possible.`
            : `${f.path} revient à son état du dernier commit : ses modifications non commitées sont perdues.`,
          confirm: isNew ? 'Supprimer' : 'Abandonner les modifications',
          danger: true,
          onConfirm: discard,
        };
      },
    };
  }

  let picked = $state<string | null>(null);
  // The picked file (kept listed by setFiles), once the list matches the selected agent and scope.
  const current = $derived(docked && listFor === wanted ? (files.find((f) => keyOf(f) === picked) ?? null) : null);
  // Primitives, so that a list refresh returning the same file does not reload its diff twice.
  const currentPath = $derived(current?.path ?? null);
  const currentOwner = $derived(current ? diffOwner(current) : null);
</script>

<div class="files" class:docked>
  <div class="scope">
    <div class="segmented" style="width:100%">
      <button style="flex:1;font-family:var(--ui);font-size:12px" class:on={scope === 'agent'} onclick={() => (app.filesScope = 'agent')}
        >Cet agent</button
      >
      <button
        style="flex:1;font-family:var(--ui);font-size:12px"
        class:on={scope === 'project'}
        onclick={() => (app.filesScope = 'project')}>Tout le projet</button
      >
    </div>
  </div>
  <div class="hint mono">{hint}</div>
  <div class="list">
    {#if error}
      <div class="empty">{error}</div>
    {:else if !files.length && !loading}
      <div class="empty">{scope === 'agent' ? 'Aucun fichier modifié par cet agent.' : 'Aucune modification non commitée.'}</div>
    {/if}
    {#each listed as f (keyOf(f))}
      {@const on = current !== null && keyOf(current) === keyOf(f)}
      <div class="filerow" class:on>
        <button
          class="file"
          class:on
          aria-current={on ? 'true' : undefined}
          onclick={() => (docked ? (picked = keyOf(f)) : openDiff([f.path], diffOwner(f), f.path))}
          oncontextmenu={(e) => fileMenu(e, f)}
        >
          <span class="st" style:color={SC[f.status]}>{f.status}</span>
          <span class="names">
            <span class="fname mono">{basename(f.path)}</span>
            <span class="dir mono">{dirname(f.path)}</span>
          </span>
          {#if scope === 'project' && f.agentId}<span class="tag mono">{agentName(f.agentId)}</span>{/if}
          <span class="add mono">+{f.add}</span>
          <span class="del mono">−{f.del}</span>
        </button>
        <!-- A sibling of the row's button, not inside it. -->
        {#if f.status !== 'D'}
          <button
            class="edit"
            aria-label={`Ouvrir ${basename(f.path)} dans l’éditeur`}
            title="Ouvrir dans l’éditeur"
            onclick={() => openInEditor(f)}
            oncontextmenu={(e) => fileMenu(e, f)}>&lt;/&gt;</button
          >
        {/if}
      </div>
    {/each}
    {#if files.length > SHOWN}<p class="more mono">… et {plural(files.length - SHOWN, 'autre fichier', 'autres fichiers')}</p>{/if}
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
    <button class="btn" style="flex:1" disabled={!files.length} onclick={openListedDiff}>Voir le diff</button>
    <button
      class="btn primary"
      style="flex:1"
      disabled={!canCommit(project, agent, scope) || !files.length}
      onclick={() => askCommit(project, agent, scope)}
    >
      {scope === 'agent' ? 'Commit…' : 'Commit tout…'}
    </button>
  </div>
  {#if scope === 'agent' && agent?.worktree}
    <div class="foot merge">
      <button class="btn" style="flex:1" onclick={() => agent && mergeAgent(agent)}
        >Merger {agent.worktree.branch} → {agent.worktree.baseBranch}…</button
      >
    </div>
  {/if}
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
  .scope {
    padding: 0 14px 12px 18px;
  }
  .hint {
    padding: 0 18px 10px;
    font-size: 11px;
    color: var(--dim);
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .list {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 0 8px 12px;
  }
  .empty {
    padding: 30px 10px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  .more {
    margin: 0;
    padding: 8px 10px;
    font-size: 11px;
    color: var(--dim);
  }
  .filerow {
    position: relative;
    display: flex;
  }
  .filerow .file {
    flex: 1;
    min-width: 0;
  }
  /* Over the row's counters, once the row is pointed at, picked or reached with the keyboard. */
  .filerow .edit {
    position: absolute;
    right: 6px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    height: 22px;
    padding: 0 7px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--accent);
    font-family: var(--mono);
    font-size: 11px;
    cursor: pointer;
  }
  .filerow:hover .edit,
  .filerow.on .edit,
  .filerow .edit:focus-visible {
    opacity: 1;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .file:hover,
  .file.on {
    background: var(--elev);
  }
  .st {
    width: 16px;
    flex: none;
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
  }
  .names {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .fname {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dir {
    font-size: 10.5px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--muted);
    white-space: nowrap;
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .add {
    font-size: 11px;
    color: var(--add);
  }
  .del {
    font-size: 11px;
    color: var(--del);
  }
  .foot {
    display: flex;
    gap: 8px;
    padding: 12px 14px 14px;
    border-top: 1px solid var(--line);
  }
  .foot.merge {
    border-top: none;
    padding-top: 0;
  }
</style>

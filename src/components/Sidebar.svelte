<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { api } from '../lib/ipc';
  import { fDur, fTok, fUsd, fWhen, tildify } from '../lib/format';
  import { copyRemoteLink, openRemote, toggleRemote } from '../lib/agent-actions';
  import { ticketTag } from '../lib/board';
  import { shortBranch } from '../lib/branches';
  import { buffers, lossNotice } from '../lib/editor/buffers.svelte';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { modelLabel } from '../lib/models';
  import { ESTIMATE_HINT, fSpentUsd, spent } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import { closeTerminal, newTerminal, SHELL_GLYPH, terminalIn } from '../lib/term-actions';
  import { canPrepare, prepareLaunch, stopTests } from '../lib/test-launch.svelte';
  import type { Agent, Project } from '../lib/types';
  import RunsSection from './RunsSection.svelte';
  import StatusDot from './StatusDot.svelte';

  let { project }: { project: Project } = $props();

  const SL: Record<string, string> = { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' };
  const SC: Record<string, string> = {
    running: 'var(--ok)',
    waiting: 'var(--wait)',
    idle: 'var(--dim)',
    done: 'var(--ok)',
    error: 'var(--del)',
  };

  let termMenuBtn = $state<HTMLButtonElement>();
  let renaming = $state<string | null>(null);
  let renameValue = $state('');

  const git = $derived(app.git[project.id]);
  const terms = $derived(app.terminals.filter((t) => t.projectId === project.id));
  const selectedAgentId = $derived(app.agent?.id);
  const termSelected = $derived(app.selectedTerm[project.id] ?? null);
  /** Another view (terminal, launch log, board) fills the main area: no agent is highlighted. */
  const otherView = $derived(!!termSelected || !!app.runCommand || app.boardOn);
  const review = $derived(app.reviewCount(project.id));

  function duration(a: Agent) {
    return fDur(a.activeMs + (a.activeSince ? app.now - a.activeSince : 0));
  }

  function startRename(a: Agent) {
    renaming = a.id;
    renameValue = a.name;
  }

  async function commitRename(a: Agent) {
    // Enter then the blur of the removed input: rename once.
    if (renaming !== a.id) return;
    const v = renameValue.trim();
    renaming = null;
    if (v && v !== a.name) await app.run(api.renameAgent(a.id, v));
  }

  function agentMenu(e: MouseEvent, a: Agent) {
    // A copy is made from a conversation at rest: during a turn, the original's session moves on.
    const turn = a.status === 'running' || a.status === 'waiting';
    menu.show(e, [
      { label: 'Renommer', onClick: () => startRename(a) },
      ...(a.archived
        ? []
        : [
            {
              label: 'Dupliquer la conversation',
              disabled: turn,
              title: turn ? 'Attends la fin de son tour.' : undefined,
              onClick: () => app.duplicateAgent(a.id),
            },
          ]),
      a.archived
        ? { label: 'Restaurer', onClick: () => app.run(api.archiveAgent(a.id, false)) }
        : { label: 'Archiver', hint: 'garde la conversation', onClick: () => confirmArchive(a) },
      ...(canPrepare(a) ? [{ label: 'Préparer le lancement', onClick: () => prepareLaunch(a) }] : []),
      ...(a.archived
        ? []
        : [
            {
              label: 'Ouvrir dans l’éditeur',
              onClick: () => app.openEditor({ projectId: a.projectId, source: a.worktree ? a.id : 'project' }),
            },
            // In its worktree, else in the project's folder (the backend knows which).
            { label: 'Ouvrir un terminal', onClick: () => terminalIn(a.projectId, { agentId: a.id }, a.name) },
          ]),
      ...remoteItems(a),
      { label: '', separator: true },
      { label: 'Supprimer…', danger: true, onClick: () => confirmDelete(a) },
    ]);
  }

  function remoteItems(a: Agent): MenuItem[] {
    if (a.archived) return [];
    const items: MenuItem[] = [
      { label: '', separator: true },
      a.remoteControl
        ? { label: 'Désactiver le remote control', onClick: () => toggleRemote(a) }
        : { label: 'Activer le remote control', hint: 'claude.ai, mobile', onClick: () => toggleRemote(a) },
    ];
    if (a.remoteControl && a.remoteUrl) {
      items.push(
        { label: 'Ouvrir sur claude.ai', onClick: () => openRemote(a) },
        { label: 'Copier le lien claude.ai', onClick: () => copyRemoteLink(a) },
      );
    }
    return items;
  }

  function remoteTitle(a: Agent) {
    const state = a.remoteState === 'connected' ? 'connecté' : a.remoteState === 'ready' ? 'connexion…' : 'en attente de connexion';
    return `Remote control : ${state} (accessible depuis claude.ai et l’app Claude)`;
  }

  /** Archives the agent; one whose ticket is under way or to test asks first: its ticket starts over from "À faire". */
  function confirmArchive(a: Agent) {
    const archive = () => {
      // Archiving stops its test launches: stopped first, their ends are no crashes.
      stopTests(a.id);
      return app.run(api.archiveAgent(a.id, true));
    };
    const t = app.ticketOf(a.id);
    if (t?.column !== 'doing' && t?.column !== 'review') {
      archive();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: `Archiver ${a.name} ?`,
      body: `Son ticket ${t.key} repartira « À faire ».`,
      confirm: 'Archiver',
      onConfirm: archive,
    };
  }

  function confirmDelete(a: Agent) {
    app.modal = {
      kind: 'confirm',
      title: `Supprimer l'agent « ${a.name} » ?`,
      body:
        'Le processus Claude est arrêté et la conversation est retirée de l’application (la session Claude Code reste sur le disque).' +
        // The files of its worktree, open in the editor: an agent without one edits the project's, which stay.
        lossNotice(buffers.unsavedIn(a.projectId, a.id)),
      confirm: 'Supprimer',
      danger: true,
      option: a.worktree ? { label: `Supprimer aussi le worktree et la branche ${a.worktree.branch}`, value: true } : undefined,
      onConfirm: async (removeWorktree) => {
        stopTests(a.id);
        const warning = await app.run(api.deleteAgent(a.id, removeWorktree));
        if (warning) app.toast(warning, 'error');
      },
    };
  }

  function shellMenu() {
    if (!termMenuBtn) return;
    if (!app.shells.length) {
      app.toast('Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages.', 'error');
      return;
    }
    menu.showAt(
      termMenuBtn,
      app.shells.map((s) => ({ label: s.label, hint: SHELL_GLYPH[s.id]?.glyph, onClick: () => newTerminal(project.id, s.id) })),
    );
  }
</script>

{#snippet browse()}
  <button class="browse" onclick={() => app.openEditor({ projectId: project.id, source: 'project' })}>Parcourir</button>
{/snippet}

<aside class="side">
  <div class="views">
    <div class="switcher" role="group" aria-label="Vue du projet">
      <button class:on={!app.boardOn} aria-pressed={!app.boardOn} onclick={() => app.closeBoard(project.id)}>Agents</button>
      <button class:on={app.boardOn} aria-pressed={app.boardOn} onclick={() => app.openBoard(project.id)}
        >Kanban{#if review}<span class="badge" title={`${review} à tester`}>{review}</span>{/if}</button
      >
    </div>
  </div>

  <div class="head">
    <span class="section-label">Agents</span>
    <span class="count">{app.projectAgents.length}</span>
    <div style="flex:1"></div>
    <button class="new" onclick={() => app.newAgent(project.id)} title={`Nouvel agent (${keyLabel('Ctrl+N')})`}>
      <span class="plus">+</span> Nouvel agent
    </button>
  </div>

  <div class="list">
    {#each app.projectAgents as a (a.id)}
      {@const sel = a.id === selectedAgentId && !otherView}
      {@const s = spent(a)}
      {@const tag = ticketTag(app.ticketOf(a.id))}
      <div
        class="card"
        class:sel
        class:alert={app.attention[a.id]}
        style:--alert={a.status === 'error' ? 'var(--del)' : a.status === 'waiting' ? 'var(--wait)' : 'var(--ok)'}
        role="button"
        tabindex="0"
        onclick={() => app.selectAgent(a.id)}
        onkeydown={(e) => e.key === 'Enter' && app.selectAgent(a.id)}
        oncontextmenu={(e) => agentMenu(e, a)}
      >
        <div class="row1">
          <StatusDot status={a.status} size={8} />
          {#if renaming === a.id}
            <!-- svelte-ignore a11y_autofocus -->
            <input
              class="rename"
              bind:value={renameValue}
              autofocus
              onclick={(e) => e.stopPropagation()}
              onkeydown={(e) => {
                e.stopPropagation();
                if (e.key === 'Enter') commitRename(a);
                if (e.key === 'Escape') renaming = null;
              }}
              onblur={() => commitRename(a)}
            />
          {:else}
            <span class="name" ondblclick={() => startRename(a)} role="presentation">{a.name}</span>
          {/if}
          {#if a.remoteControl}
            <span class="rc" class:on={a.remoteState === 'connected'} title={remoteTitle(a)}>
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                aria-hidden="true"
                ><path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0" /><circle cx="12" cy="19.5" r="1" fill="currentColor" /></svg
              >
            </span>
          {/if}
          {#if a.status === 'waiting'}
            <span class="pill">Question</span>
          {:else if a.resumeAt}
            <span
              class="status"
              style:color="var(--wait)"
              title="Arrêté par la limite d’usage : reprise automatique {fWhen(a.resumeAt, app.now)}"
              >Reprise {fWhen(a.resumeAt, app.now)}</span
            >
          {:else if a.setup}
            <span class="status" style:color="var(--wait)" title="Préparation du worktree : {a.setup}">Préparation…</span>
          {:else}
            <span class="status" style:color={SC[a.status]}>{SL[a.status]}</span>
          {/if}
        </div>
        <div class="meta">
          <span>{modelLabel(a.model, app.models)}</span><span class="sep">·</span><span>{duration(a)}</span>
          {#if a.worktree}<span class="sep">·</span><span class="wt" title={a.worktree.branch}>⎇ {shortBranch(a.worktree.branch)}</span
            >{/if}
        </div>
        <div class="meta dim" title={s.estimated ? ESTIMATE_HINT : undefined}>
          <span>{fTok(s.tokens)} tok</span><span>{fSpentUsd(s)}</span><span>{git?.agents[a.id] ?? 0} fich.</span>
        </div>
        {#if tag}
          <div class="ticket-tag mono">▸ {tag}</div>
        {/if}
      </div>
    {:else}
      <div class="empty">
        Aucun agent.<br />
        <button class="btn" style="margin-top:10px" onclick={() => app.newAgent(project.id)}>Créer un agent</button>
      </div>
    {/each}

    {#if app.archivedAgents.length}
      <button class="archived-toggle" onclick={() => (app.showArchived = !app.showArchived)}>
        {app.showArchived ? '▾' : '▸'} Archivés ({app.archivedAgents.length})
      </button>
      {#if app.showArchived}
        {#each app.archivedAgents as a (a.id)}
          <div
            class="card archived"
            class:sel={a.id === selectedAgentId && !otherView}
            role="button"
            tabindex="0"
            onclick={() => app.selectAgent(a.id)}
            onkeydown={(e) => e.key === 'Enter' && app.selectAgent(a.id)}
            oncontextmenu={(e) => agentMenu(e, a)}
          >
            <div class="row1">
              <span class="name">{a.name}</span>
              <span class="status">{fUsd(a.cost)}</span>
            </div>
          </div>
        {/each}
      {/if}
    {/if}
  </div>

  <RunsSection {project} />

  <div class="terms">
    <div class="head small">
      <span class="section-label">Terminaux</span>
      <span class="count">{terms.length}</span>
      <div style="flex:1"></div>
      <button class="plus-btn" bind:this={termMenuBtn} title="Nouveau terminal" onclick={shellMenu}>+</button>
    </div>
    <div class="term-list">
      {#each terms as t (t.id)}
        {@const g = SHELL_GLYPH[t.shell] ?? { glyph: '>_', c: 'var(--muted)' }}
        <div
          class="term"
          class:sel={termSelected === t.id}
          role="button"
          tabindex="0"
          onclick={() => app.selectTerm(t.id)}
          onkeydown={(e) => e.key === 'Enter' && app.selectTerm(t.id)}
        >
          <span class="glyph" style:color={g.c}>{g.glyph}</span>
          <span class="tname">{t.name}</span>
          {#if app.exitedTerms[t.id] !== undefined}<span class="shell">terminé</span>{/if}
          <button
            class="x"
            title="Fermer"
            onclick={(e) => {
              e.stopPropagation();
              closeTerminal(t.id);
            }}>×</button
          >
        </div>
      {:else}
        <span class="none">Aucun terminal ouvert</span>
      {/each}
    </div>
  </div>

  <div class="foot">
    <div class="path mono" title={project.path}>{tildify(project.path)}</div>
    {#if git?.isRepo}
      <div class="branch mono">
        <span class="ring"></span>{git.branch || '—'}
        <div style="flex:1"></div>
        {@render browse()}
      </div>
      <div class="gitc mono">
        <span style="color:var(--wait)">~{git.modified} modifiés</span>
        <span style="color:var(--add)">+{git.added} ajoutés</span>
        <span style="color:var(--del)">−{git.deleted} supprimés</span>
      </div>
    {:else}
      <!-- No git information yet, or no repository: the project's folder can be browsed all the same. -->
      <div class="branch mono">
        {#if git}<span style="color:var(--dim)">Pas de dépôt git</span>{/if}
        <div style="flex:1"></div>
        {@render browse()}
      </div>
    {/if}
  </div>
</aside>

<style>
  .side {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-right: 1px solid var(--line);
    min-height: 0;
  }
  .views {
    padding: 12px 10px 0;
  }
  .switcher {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  .switcher button {
    flex: 1;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .switcher button.on {
    background: var(--elev2);
    color: var(--text);
  }
  .badge {
    min-width: 17px;
    height: 17px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--wait);
    color: #2a1f05;
    font-family: var(--mono);
    font-size: 10px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 14px 10px 18px;
  }
  .head.small {
    padding: 12px 14px 8px 18px;
  }
  .count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
  }
  .new {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .new:hover,
  .plus-btn:hover {
    border-color: var(--accent);
  }
  .plus {
    font-size: 15px;
    line-height: 1;
    color: var(--accent);
  }
  .list {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 10px 12px;
    min-height: 80px;
  }
  .card {
    text-align: left;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 11px 12px;
    border-radius: var(--r);
    border: 1px solid transparent;
    cursor: pointer;
    outline: none;
  }
  .card:hover:not(.sel) {
    background: color-mix(in oklch, var(--elev) 55%, transparent);
  }
  .card.sel {
    background: var(--elev);
    border-color: var(--line2);
  }
  /* Asked, finished or failed out of sight: blinks until seen. */
  .card.alert {
    border-color: var(--alert);
    animation: cardAlert 1.1s ease-in-out infinite alternate;
  }
  @keyframes cardAlert {
    from {
      background: transparent;
    }
    to {
      background: color-mix(in oklch, var(--alert) 18%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card.alert {
      animation: none;
      background: color-mix(in oklch, var(--alert) 14%, transparent);
    }
  }
  .card:focus-visible {
    border-color: var(--accent);
  }
  .card.archived {
    padding: 8px 12px;
    opacity: 0.7;
  }
  .row1 {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
  }
  .name {
    font-weight: 600;
    font-size: 13.5px;
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rename {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    border: 1px solid var(--accent);
    border-radius: 4px;
    background: var(--bg);
    font-size: 13px;
    font-weight: 600;
    outline: none;
  }
  .status {
    font-size: 11px;
    color: var(--muted);
    flex: none;
  }
  .rc {
    flex: none;
    display: inline-flex;
    color: var(--dim);
  }
  .rc.on {
    color: var(--info);
  }
  .meta {
    display: flex;
    gap: 10px;
    padding-left: 17px;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .meta.dim {
    color: var(--dim);
  }
  .ticket-tag {
    margin-left: 17px;
    align-self: flex-start;
    font-size: 10.5px;
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--accent);
  }
  .sep {
    color: var(--dim);
  }
  .wt {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    padding: 24px 8px;
    text-align: center;
    color: var(--dim);
    font-size: 12.5px;
  }
  .archived-toggle {
    margin-top: 8px;
    padding: 6px 8px;
    border: none;
    background: transparent;
    color: var(--dim);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .archived-toggle:hover {
    color: var(--text);
  }
  .terms {
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    padding-bottom: 10px;
    max-height: 40%;
    min-height: 0;
  }
  .plus-btn {
    width: 26px;
    height: 26px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--accent);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }
  .term-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 10px;
    overflow: auto;
  }
  .term {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 6px 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid transparent;
    cursor: pointer;
    flex: none;
  }
  .term:hover:not(.sel) {
    background: color-mix(in oklch, var(--elev) 55%, transparent);
  }
  .term.sel {
    background: var(--elev);
    border-color: var(--line2);
  }
  .glyph {
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 700;
    padding: 2px 5px;
    border-radius: 3px;
    background: var(--bg);
    min-width: 24px;
    text-align: center;
  }
  .tname {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .shell {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
  .x {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--dim);
    font-size: 13px;
    cursor: pointer;
  }
  .x:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .none {
    padding: 4px 8px 6px;
    font-size: 12px;
    color: var(--dim);
  }
  .foot {
    border-top: 1px solid var(--line);
    padding: 14px 18px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .browse {
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-family: var(--ui);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .browse:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .path {
    font-size: 11.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
  }
  .ring {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    border: 1.5px solid var(--muted);
  }
  .gitc {
    display: flex;
    gap: 12px;
    font-size: 11px;
  }
</style>

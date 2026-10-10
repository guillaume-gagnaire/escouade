<script lang="ts">
  import { tick } from 'svelte';
  import { keyLabel } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { fDur, fTok, fUsd, fWhen, tildify } from '../lib/format';
  import { copyRemoteLink, integrateBase, openRemote, toggleRemote } from '../lib/agent-actions';
  import { startAgentOn } from '../lib/branch-actions';
  import { ticketTag } from '../lib/board';
  import { shortBranch } from '../lib/branches';
  import { buffers, lossNotice } from '../lib/editor/buffers.svelte';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { modelLabel } from '../lib/models';
  import { estimateHint, fSpentUsd, spent } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import { closeTerminal, newTerminal, SHELL_GLYPH, terminalIn } from '../lib/term-actions';
  import { canPrepare, prepareLaunch, stopTests } from '../lib/test-launch.svelte';
  import type { Agent, AgentStatus, Project } from '../lib/types';
  import RunsSection from './RunsSection.svelte';
  import StatusDot from './StatusDot.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';

  let { project }: { project: Project } = $props();

  /** What an agent is doing, read when shown (the language of the interface can change). */
  const statusLabel = (status: AgentStatus) => t(`nav.sidebar.status.${status}`);
  const SC: Record<string, string> = {
    running: 'var(--ok)',
    waiting: 'var(--wait)',
    idle: 'var(--dim)',
    done: 'var(--ok)',
    error: 'var(--del)',
  };

  let termMenuBtn = $state<HTMLButtonElement>();
  let moreBtn = $state<HTMLButtonElement>();
  /** The branch picker is open, to choose the branch of a new agent. */
  let pickingBranch = $state(false);
  let renaming = $state<string | null>(null);
  let renameValue = $state('');

  const git = $derived(app.git[project.id]);
  const terms = $derived(app.terminals.filter((x) => x.projectId === project.id));
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
      { label: t('common.rename'), onClick: () => startRename(a) },
      ...(a.archived
        ? []
        : [
            {
              label: t('nav.sidebar.menu.duplicate'),
              disabled: turn,
              title: turn ? t('nav.sidebar.menu.waitTurn') : undefined,
              onClick: () => app.duplicateAgent(a.id),
            },
          ]),
      a.archived
        ? { label: t('nav.sidebar.menu.restore'), onClick: () => app.run(api.archiveAgent(a.id, false)) }
        : { label: t('nav.sidebar.menu.archive'), hint: t('nav.sidebar.menu.archiveHint'), onClick: () => confirmArchive(a) },
      ...(canPrepare(a) ? [{ label: t('nav.sidebar.menu.prepareLaunch'), onClick: () => prepareLaunch(a) }] : []),
      // Its base branch goes into its own, as the project's strategy says.
      ...(a.worktree && !a.archived
        ? [{ label: t('branches.integrate.menu', { base: a.worktree.baseBranch }), onClick: () => integrateBase(a) }]
        : []),
      ...(a.archived
        ? []
        : [
            {
              label: t('common.openInEditor'),
              onClick: () => app.openEditor({ projectId: a.projectId, source: a.worktree ? a.id : 'project' }),
            },
            // In its worktree, else in the project's folder (the backend knows which).
            { label: t('nav.sidebar.menu.openTerminal'), onClick: () => terminalIn(a.projectId, { agentId: a.id }, a.name) },
          ]),
      ...remoteItems(a),
      { label: '', separator: true },
      { label: t('nav.sidebar.menu.delete'), danger: true, onClick: () => confirmDelete(a) },
    ]);
  }

  function remoteItems(a: Agent): MenuItem[] {
    if (a.archived) return [];
    const items: MenuItem[] = [
      { label: '', separator: true },
      a.remoteControl
        ? { label: t('nav.sidebar.menu.remoteOff'), onClick: () => toggleRemote(a) }
        : { label: t('nav.sidebar.menu.remoteOn'), hint: t('nav.sidebar.menu.remoteOnHint'), onClick: () => toggleRemote(a) },
    ];
    if (a.remoteControl && a.remoteUrl) {
      items.push(
        { label: t('nav.sidebar.menu.openRemote'), onClick: () => openRemote(a) },
        { label: t('nav.sidebar.menu.copyRemoteLink'), onClick: () => copyRemoteLink(a) },
      );
    }
    return items;
  }

  function remoteTitle(a: Agent) {
    return t(
      a.remoteState === 'connected'
        ? 'nav.sidebar.remote.connected'
        : a.remoteState === 'ready'
          ? 'nav.sidebar.remote.connecting'
          : 'nav.sidebar.remote.waiting',
    );
  }

  /** Archives the agent; one whose ticket is under way or to test asks first: its ticket starts over from "À faire". */
  function confirmArchive(a: Agent) {
    const archive = () => {
      // Archiving stops its test launches: stopped first, their ends are no crashes.
      stopTests(a.id);
      return app.run(api.archiveAgent(a.id, true));
    };
    const ticket = app.ticketOf(a.id);
    if (ticket?.column !== 'doing' && ticket?.column !== 'review') {
      archive();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('nav.sidebar.archiveTitle', { name: a.name }),
      body: t('nav.sidebar.archiveBody', { key: ticket.key }),
      confirm: t('nav.sidebar.archiveConfirm'),
      onConfirm: archive,
    };
  }

  function confirmDelete(a: Agent) {
    app.modal = {
      kind: 'confirm',
      title: t('nav.sidebar.deleteTitle', { name: a.name }),
      body:
        t('nav.sidebar.deleteBody') +
        // The files of its worktree, open in the editor: an agent without one edits the project's, which stay.
        lossNotice(buffers.unsavedIn(a.projectId, a.id)),
      confirm: t('common.delete'),
      danger: true,
      option: a.worktree
        ? {
            // A branch that was there before the agent stays, whatever becomes of its worktree.
            label: a.worktree.existing
              ? t('branches.agent.deleteWorktree', { branch: a.worktree.branch })
              : t('nav.sidebar.deleteWorktree', { branch: a.worktree.branch }),
            value: true,
          }
        : undefined,
      onConfirm: async (removeWorktree) => {
        stopTests(a.id);
        const warning = await app.run(api.deleteAgent(a.id, removeWorktree));
        if (warning) app.toast(warning, 'error');
      },
    };
  }

  function newAgentMenu() {
    if (!moreBtn) return;
    menu.showAt(moreBtn, [
      {
        label: t('branches.agent.onBranch'),
        // Once the menu is gone and has given the focus back to its button: the picker gives it back there in turn.
        onClick: async () => {
          await tick();
          pickingBranch = true;
        },
      },
    ]);
  }

  function shellMenu() {
    if (!termMenuBtn) return;
    if (!app.shells.length) {
      app.toast(t('nav.sidebar.noShell'), 'error');
      return;
    }
    menu.showAt(
      termMenuBtn,
      app.shells.map((s) => ({ label: s.label, hint: SHELL_GLYPH[s.id]?.glyph, onClick: () => newTerminal(project.id, s.id) })),
    );
  }
</script>

{#snippet browse()}
  <button class="browse" onclick={() => app.openEditor({ projectId: project.id, source: 'project' })}>{t('common.browse')}</button>
{/snippet}

<aside class="side">
  <div class="views">
    <div class="switcher" role="group" aria-label={t('nav.sidebar.viewLabel')}>
      <button class:on={!app.boardOn} aria-pressed={!app.boardOn} onclick={() => app.closeBoard(project.id)}>{t('common.agents')}</button>
      <button class:on={app.boardOn} aria-pressed={app.boardOn} onclick={() => app.openBoard(project.id)}
        >Kanban{#if review}<span class="badge" title={t('nav.sidebar.toReview', { count: review })}>{review}</span>{/if}</button
      >
    </div>
  </div>

  <div class="head">
    <span class="section-label">{t('common.agents')}</span>
    <span class="count">{app.projectAgents.length}</span>
    <div style="flex:1"></div>
    <div class="newgrp">
      <button class="new" onclick={() => app.newAgent(project.id)} title={t('nav.sidebar.newAgentTitle', { shortcut: keyLabel('Ctrl+N') })}>
        <span class="plus">+</span>
        {t('nav.sidebar.newAgent')}
      </button>
      <button
        class="more"
        bind:this={moreBtn}
        aria-haspopup="menu"
        aria-label={t('branches.agent.moreWays')}
        title={t('branches.agent.moreWays')}
        onclick={newAgentMenu}>▾</button
      >
    </div>
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
            <span class="pill">{statusLabel('waiting')}</span>
          {:else if a.resumeAt}
            <span class="status" style:color="var(--wait)" title={t('nav.sidebar.resumeTitle', { when: fWhen(a.resumeAt, app.now) })}
              >{t('nav.sidebar.resumes', { when: fWhen(a.resumeAt, app.now) })}</span
            >
          {:else if a.setup}
            <span class="status" style:color="var(--wait)" title={t('nav.sidebar.settingUpTitle', { step: a.setup })}
              >{t('nav.sidebar.settingUp')}</span
            >
          {:else}
            <span class="status" style:color={SC[a.status]}>{statusLabel(a.status)}</span>
          {/if}
        </div>
        {#if a.progressLine}
          <!-- What the agent reported through Escouade's MCP server, until its next report. -->
          <div class="said" title={a.progressLine}><span class="sr">{t('mcp.progress.label')}{' '}</span>{a.progressLine}</div>
        {/if}
        <div class="meta">
          <span>{modelLabel(a.model, app.models)}</span><span class="sep">·</span><span>{duration(a)}</span>
          {#if a.worktree}<span class="sep">·</span><span class="wt" title={a.worktree.branch}>⎇ {shortBranch(a.worktree.branch)}</span
            >{/if}
        </div>
        <div class="meta dim" title={s.estimated ? estimateHint() : undefined}>
          <span>{t('nav.sidebar.tokens', { tokens: fTok(s.tokens) })}</span><span>{fSpentUsd(s)}</span><span
            >{t('nav.sidebar.files', { count: git?.agents[a.id] ?? 0 })}</span
          >
        </div>
        {#if tag}
          <div class="ticket-tag mono">▸ {tag}</div>
        {/if}
      </div>
    {:else}
      <div class="empty">
        {t('nav.sidebar.empty')}<br />
        <button class="btn" style="margin-top:10px" onclick={() => app.newAgent(project.id)}>{t('nav.sidebar.createAgent')}</button>
      </div>
    {/each}

    {#if app.archivedAgents.length}
      <button class="archived-toggle" onclick={() => (app.showArchived = !app.showArchived)}>
        {app.showArchived ? '▾' : '▸'}
        {t('nav.sidebar.archived', { n: app.archivedAgents.length })}
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
      <span class="section-label">{t('nav.sidebar.terminals')}</span>
      <span class="count">{terms.length}</span>
      <div style="flex:1"></div>
      <button class="plus-btn" bind:this={termMenuBtn} title={t('nav.sidebar.newTerminal')} onclick={shellMenu}>+</button>
    </div>
    <div class="term-list">
      {#each terms as term (term.id)}
        {@const g = SHELL_GLYPH[term.shell] ?? { glyph: '>_', c: 'var(--muted)' }}
        <div
          class="term"
          class:sel={termSelected === term.id}
          role="button"
          tabindex="0"
          onclick={() => app.selectTerm(term.id)}
          onkeydown={(e) => e.key === 'Enter' && app.selectTerm(term.id)}
        >
          <span class="glyph" style:color={g.c}>{g.glyph}</span>
          <span class="tname">{term.name}</span>
          {#if app.exitedTerms[term.id] !== undefined}<span class="shell">{t('nav.sidebar.exited')}</span>{/if}
          <button
            class="x"
            title={t('common.close')}
            onclick={(e) => {
              e.stopPropagation();
              closeTerminal(term.id);
            }}>×</button
          >
        </div>
      {:else}
        <span class="none">{t('nav.sidebar.noTerminals')}</span>
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
        <span style="color:var(--wait)">{t('nav.sidebar.modified', { n: git.modified })}</span>
        <span style="color:var(--add)">{t('nav.sidebar.added', { n: git.added })}</span>
        <span style="color:var(--del)">{t('nav.sidebar.deleted', { n: git.deleted })}</span>
      </div>
    {:else}
      <!-- No git information yet, or no repository: the project's folder can be browsed all the same. -->
      <div class="branch mono">
        {#if git}<span style="color:var(--dim)">{t('nav.sidebar.noRepo')}</span>{/if}
        <div style="flex:1"></div>
        {@render browse()}
      </div>
    {/if}
  </div>
</aside>

{#if pickingBranch}
  <BranchPicker
    projectId={project.id}
    anchor={moreBtn}
    mode="pick"
    onclose={() => (pickingBranch = false)}
    onpick={(b) => startAgentOn(project.id, b.name)}
  />
{/if}

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
  .more:hover,
  .plus-btn:hover {
    border-color: var(--accent);
  }
  /* « Nouvel agent » and the menu of the other ways to make one: one control in two parts. */
  .newgrp {
    display: flex;
  }
  .newgrp .new {
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .more {
    width: 24px;
    height: 28px;
    margin-left: -1px;
    padding: 0;
    border: 1px solid var(--line2);
    border-radius: 0 var(--r-sm) var(--r-sm) 0;
    background: var(--elev);
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }
  .more:hover {
    color: var(--text);
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
  .said {
    padding-left: 17px;
    font-size: 11.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
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

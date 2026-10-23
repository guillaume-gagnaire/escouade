<script lang="ts">
  import { tick } from 'svelte';
  import { keyLabel } from '../lib/platform';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { fDur, fTok, fUsd, fWhen, tildify } from '../lib/format';
  import { canSwitchView, copyRemoteLink, inHarnessView, integrateBase, openRemote, switchView, toggleRemote } from '../lib/agent-actions';
  import { can, canFork, choosesEngine, engineName, engineOptions, showsCost, speakerOf } from '../lib/engines';
  import { startAgentOn } from '../lib/branch-actions';
  import { canFixChecks, canFixReviews, cardPrs, fixPr, openPr, prGlyph, prMenuItem, prTip, prTone, prWord, stateWord } from '../lib/prs';
  import { runIn } from '../lib/launch-actions';
  import { ticketTag } from '../lib/board';
  import { shortBranch } from '../lib/branches';
  import { buffers, lossNotice } from '../lib/editor/buffers.svelte';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { modelLabel } from '../lib/models';
  import { planView } from '../lib/plan';
  import { sidebarSubs, type SubRow } from '../lib/subagents';
  import { BG_FINISHED_SHOWN, BG_RUNNING_SHOWN, bgCount, bgDuration, rowLine, rowTitle, sidebarBg, type BgRow } from '../lib/bgtasks';
  import { estimateHint, fSpentUsd, spent } from '../lib/spend';
  import { drivenAgents, pendingFor, pendingLabel, pendingText } from '../lib/later';
  import { app } from '../lib/state.svelte';
  import { closeTerminal, newTerminal, SHELL_GLYPH, terminalIn } from '../lib/term-actions';
  import { canPrepare, prepareLaunch, stopTests } from '../lib/test-launch.svelte';
  import { backTarget, forwardTargets } from '../lib/handover';
  import type { Agent, AgentStatus, BranchInfo, EngineId, Project } from '../lib/types';
  import RunsSection from './RunsSection.svelte';
  import EngineMark from './EngineMark.svelte';
  import StatusDot from './StatusDot.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';
  import { tip } from '../lib/tip';

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
  /**
   * The branch picker is open to choose where a new agent's worktree starts (ESC-160): `branch`, the new branch typed in
   * « Nouvel agent sur une branche… » that starts there; null, an agent of a worktree of its own.
   */
  let pickingBase = $state<{ branch: string | null } | null>(null);
  /** The new branch the picker of the starting branch was opened for: read once it is closed, as it gives its pick then. */
  let baseFor: string | null = null;
  function pickBase(branch: string | null) {
    baseFor = branch;
    pickingBase = { branch };
  }
  function startFrom(b: BranchInfo) {
    if (baseFor) void startAgentOn(project.id, baseFor, true, b.name);
    else void app.newAgent(project.id, null, true, b.name);
  }
  let renaming = $state<string | null>(null);
  /** The finished subagents of an agent, unfolded by the reader (folded by default: there can be many). */
  let finishedOpen = $state<Record<string, boolean>>({});
  /** The subagents of an agent beyond the first ones of a list, unrolled by the reader: the running, the finished. */
  let runningAll = $state<Record<string, boolean>>({});
  let finishedAll = $state<Record<string, boolean>>({});
  /** How many subagents a list shows before it asks to unroll the others: it never scrolls by itself, inside the list of the agents. */
  const RUNNING_SHOWN = 5;
  const FINISHED_SHOWN = 10;
  /** The same for the background tasks of an agent (ESC-14): their finished ones unfolded, and their lists unrolled. */
  let bgFinishedOpen = $state<Record<string, boolean>>({});
  let bgRunningAll = $state<Record<string, boolean>>({});
  /** The list of the agents an agent drives (ESC-138), folded by the reader. */
  let drivenOpen = $state<Record<string, boolean>>({});
  let bgFinishedAll = $state<Record<string, boolean>>({});
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
    if (!v || v === a.name) return;
    // Its branch follows the name, unless it must keep its own: said, the agent renamed all the same.
    const kept = await app.run(api.renameAgent(a.id, v));
    if (kept) app.toast(kept, 'info');
  }

  function agentMenu(e: MouseEvent, a: Agent) {
    // A copy is made from a conversation at rest: during a turn, the original's session moves on.
    const turn = a.status === 'running' || a.status === 'waiting';
    menu.show(e, [
      { label: t('common.rename'), onClick: () => startRename(a) },
      // Only for a harness that copies a conversation.
      ...(a.archived || !canFork(a)
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
      ...(a.worktree && !a.archived && project.runCommands.length
        ? [{ label: t('nav.sidebar.menu.launch'), onClick: () => launchMenu(e, a) }]
        : []),
      ...(canPrepare(a) ? [{ label: t('nav.sidebar.menu.prepareLaunch'), onClick: () => prepareLaunch(a) }] : []),
      // Its base branch goes into its own, as the project's strategy says.
      ...(a.worktree && !a.archived
        ? [{ label: t('branches.integrate.menu', { base: a.worktree.baseBranch }), onClick: () => integrateBase(a) }]
        : []),
      // « Ouvrir une PR… » on its branch (ESC-64), « Voir la PR #n » once one is open.
      ...[prMenuItem(a, tick)].filter((x): x is MenuItem => x !== null),
      ...(a.archived
        ? []
        : [
            // Its worktree waits for its first message (ESC-156): neither opens in the project's folder meanwhile.
            {
              label: t('common.openInEditor'),
              disabled: !!a.worktreePending,
              title: a.worktreePending ? t('nav.sidebar.worktreeLaterTitle') : undefined,
              onClick: () => app.openEditor({ projectId: a.projectId, source: a.worktree ? a.id : 'project' }),
            },
            // In its worktree, else in the project's folder (the backend knows which).
            {
              label: t('nav.sidebar.menu.openTerminal'),
              disabled: !!a.worktreePending,
              title: a.worktreePending ? t('nav.sidebar.worktreeLaterTitle') : undefined,
              onClick: () => terminalIn(a.projectId, { agentId: a.id }, a.name),
            },
          ]),
      // As the button of its conversation's header.
      ...(canSwitchView(a)
        ? [
            {
              label: t(inHarnessView(a) ? 'nav.sidebar.menu.toRich' : 'nav.sidebar.menu.toTerminal', { engine: engineName(a.engine) }),
              onClick: () => switchView(a),
            },
          ]
        : []),
      ...handoverItems(e, a),
      ...remoteItems(a),
      { label: '', separator: true },
      { label: t('nav.sidebar.menu.delete'), danger: true, onClick: () => confirmDelete(a) },
    ]);
  }

  /** « Lancer… »: the project's launch commands, each to run in the agent's worktree, where its menu was. */
  async function launchMenu(e: MouseEvent, a: Agent) {
    const at = { x: e.clientX, y: e.clientY };
    // Once the card's menu is gone.
    await tick();
    menu.open = {
      ...at,
      items: project.runCommands.map((c) => ({ label: c.name, hint: c.cwd || undefined, onClick: () => runIn(project, c, a.id) })),
    };
  }

  /** « Continuer sur un autre harness… » at any time, and « Revenir sur <harness> » once the quota of the one it left came back (ESC-88). */
  function handoverItems(e: MouseEvent, a: Agent): MenuItem[] {
    if (a.archived) return [];
    const forward = forwardTargets(a);
    const back = backTarget(a);
    // Its turn ends first; a question or a permission waiting goes in the handover file.
    const waits = a.status === 'running';
    const busy = { disabled: waits, title: waits ? t('handover.waitTurn') : undefined };
    const items: MenuItem[] = [];
    if (forward.length) items.push({ label: t('handover.menu'), ...busy, onClick: () => engineMenu(e, a, forward) });
    if (back)
      items.push({
        label: t('handover.back', { harness: engineName(back) }),
        ...busy,
        onClick: () => app.run(api.continueOnEngine(a.id, back)),
      });
    return items;
  }

  /** The harnesses the agent may go on on, where its menu was. */
  async function engineMenu(e: MouseEvent, a: Agent, engines: EngineId[]) {
    const at = { x: e.clientX, y: e.clientY };
    await tick();
    menu.open = {
      ...at,
      items: engines.map((x) => ({
        label: t('handover.continueOn', { harness: engineName(x) }),
        onClick: () => app.run(api.continueOnEngine(a.id, x)),
      })),
    };
  }

  function remoteItems(a: Agent): MenuItem[] {
    if (a.archived || !can(a, 'remoteControl')) return [];
    const items: MenuItem[] = [
      { label: '', separator: true },
      a.remoteControl
        ? { label: t('nav.sidebar.menu.remoteOff'), onClick: () => toggleRemote(a) }
        : { label: t('nav.sidebar.menu.remoteOn'), hint: t('nav.sidebar.menu.remoteOnHint'), onClick: () => toggleRemote(a) },
    ];
    // In Claude Code's own interface, the session is the interface's: its link is offered once the interface says it is up.
    if (a.remoteControl && a.remoteUrl && (!a.terminal || a.remoteState === 'connected')) {
      items.push(
        { label: t('nav.sidebar.menu.openRemote'), onClick: () => openRemote(a) },
        { label: t('nav.sidebar.menu.copyRemoteLink'), onClick: () => copyRemoteLink(a) },
      );
    }
    return items;
  }

  type RemoteState = 'connected' | 'connecting' | 'waiting' | 'noInterface' | 'nextLaunch' | 'unavailable' | 'stillOn';

  /** The state of the remote control icon of an agent's card, null for none: what its link is, or what keeps it. */
  function remoteStateOf(a: Agent): RemoteState | null {
    if (!can(a, 'remoteControl')) return null;
    if (!a.terminal) {
      if (!a.remoteControl) return null;
      return a.remoteState === 'connected' ? 'connected' : a.remoteState === 'ready' ? 'connecting' : 'waiting';
    }
    // In Claude Code's view the link is the interface's, which Escouade does not drive: what it reports is the state.
    if (!a.remoteControl) {
      // Off for Escouade, but the open interface stays linked to claude.ai until its next launch: only if it really is linked.
      return a.interfaceRemote === true && a.remoteState === 'connected' ? 'stillOn' : null;
    }
    // One started before the remote control came on has it only from its next launch.
    if (a.interfaceRemote === null) return 'noInterface';
    if (a.interfaceRemote === false) return 'nextLaunch';
    // Started with it: linked, or not opened by Claude Code (no subscription, network, proxy or policy), or not told yet.
    return a.remoteState === 'unavailable' ? 'unavailable' : a.remoteState === 'connected' ? 'connected' : 'connecting';
  }

  /** The state of its link, then what turned it on. */
  function remoteTitle(a: Agent, state: RemoteState) {
    const text = t(`nav.sidebar.remote.${state}`);
    return a.remoteSource
      ? t('nav.sidebar.remote.withSource', { state: text, source: t(`nav.sidebar.remote.source.${a.remoteSource}`) })
      : text;
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
        t('nav.sidebar.deleteBody', { engine: speakerOf(a.engine), harness: engineName(a.engine) }) +
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
    // From two harnesses found on, one entry each, the project's first: « Nouvel agent » alone goes on the project's.
    const own = project.engine ?? 'claude';
    const engines = choosesEngine() ? engineOptions().sort((a, b) => Number(b.value === own) - Number(a.value === own)) : [];
    menu.showAt(moreBtn, [
      ...engines.map((e) => ({
        label: t('nav.sidebar.newAgentOn', { engine: e.label }),
        onClick: () => app.newAgent(project.id, e.value),
      })),
      ...(engines.length ? [{ label: '', separator: true }] : []),
      {
        label: t('branches.agent.onBranch'),
        // Once the menu is gone and has given the focus back to its button: the picker gives it back there in turn.
        onClick: async () => {
          await tick();
          pickingBranch = true;
        },
      },
      // The other way round from « Un worktree par agent »: without one in a project that gives one, in one otherwise.
      {
        label: project.worktreePerAgent ? t('nav.sidebar.newAgentNoWorktree') : t('nav.sidebar.newAgentInWorktree'),
        onClick: () => app.newAgent(project.id, null, !project.worktreePerAgent),
      },
      // In a worktree that starts from a branch chosen next, whatever the project's starting branch (ESC-160).
      {
        label: t('nav.sidebar.newAgentFrom'),
        onClick: async () => {
          await tick();
          pickBase(null);
        },
      },
    ]);
  }

  /** The agents of the project that have a worktree to open a terminal in (not the archived ones), as the list shows them. */
  const worktreeAgents = () => app.projectAgents.filter((a) => a.worktree);

  /** The agents whose worktree can hold a terminal, one entry each. */
  function worktreeMenu() {
    if (!termMenuBtn || !worktreeAgents().length) return;
    menu.showAt(
      termMenuBtn,
      worktreeAgents().map((a) => ({ label: a.name, onClick: () => terminalIn(project.id, { agentId: a.id }, a.name) })),
    );
  }

  function shellMenu() {
    if (!termMenuBtn) return;
    if (!app.shells.length) {
      app.toast(t('nav.sidebar.noShell'), 'error');
      return;
    }
    menu.showAt(termMenuBtn, [
      ...app.shells.map((s) => ({ label: s.label, hint: SHELL_GLYPH[s.id]?.glyph, onClick: () => newTerminal(project.id, s.id) })),
      ...(worktreeAgents().length
        ? [
            { label: '', separator: true },
            {
              label: t('nav.sidebar.inWorktree'),
              // Once this menu is gone and has given the focus back to the button: the next one takes it from there.
              onClick: async () => {
                await tick();
                worktreeMenu();
              },
            },
          ]
        : []),
    ]);
  }
</script>

<!-- A subagent of the agent `a` under its card: a click opens its conversation in place of the agent's. -->
{#snippet subRow(a: Agent, s: SubRow)}
  {@const on = app.subView[a.id] === s.id && a.id === selectedAgentId && !otherView}
  <li>
    <button
      class="sub"
      class:on
      aria-current={on ? 'page' : undefined}
      style:--depth={s.depth}
      use:tip={{ text: s.title, cut: true }}
      onclick={() => app.openSubagent(a.id, s.id)}
    >
      <span class="sdot {s.status}" aria-hidden="true"></span>
      <span class="stext">
        <span class="sline">
          <span class="stitle">{s.title}</span>
          {#if s.model}<span class="smodel mono">{modelLabel(s.model, app.models)}</span>{/if}
          {#if s.status !== 'running'}<span class="sstate">{t(`subagents.status.${s.status}`)}</span>{/if}
        </span>
        {#if s.status === 'running' && s.doing}<span class="sdoing">{s.doing}</span>{/if}
      </span>
    </button>
  </li>
{/snippet}

<!-- The button of a list that shows only its first rows: it unrolls the others, then folds them back (the focus stays). -->
{#snippet moreRows(all: boolean, label: string, onclick: () => void, less: string = t('subagents.lessList'))}
  <li>
    <button class="more-toggle" aria-expanded={all} {onclick}>
      <span class="chev" aria-hidden="true">{all ? '▾' : '▸'}</span>{all ? less : label}
    </button>
  </li>
{/snippet}

<!-- A background task (or a workflow) of the agent `a` under its card: a click opens its view in place of the agent's conversation (ESC-14). -->
{#snippet bgItem(a: Agent, r: BgRow)}
  {@const on = app.taskView[a.id] === r.id && a.id === selectedAgentId && !otherView}
  {@const ms = bgDuration(r, app.now)}
  <li>
    <button
      class="sub"
      class:on
      aria-current={on ? 'page' : undefined}
      data-bg-row={r.id}
      data-kind={r.kind}
      data-status={r.status}
      use:tip={{ text: rowTitle(r), cut: true }}
      onclick={() => app.openTask(a.id, r.id)}
    >
      <span class="sdot {r.status}" aria-hidden="true"></span>
      <span class="stext">
        <span class="sline">
          <span class="stitle">{rowTitle(r)}</span>
          {#if ms !== null}<span class="sdur mono">{fDur(ms)}</span>{/if}
        </span>
        <span class="sdoing">{rowLine(r)}</span>
      </span>
    </button>
  </li>
{/snippet}

{#snippet browse()}
  <button class="browse" onclick={() => app.openEditor({ projectId: project.id, source: 'project' })}>{t('common.browse')}</button>
{/snippet}

<aside class="side">
  <div class="views">
    <div class="switcher" role="group" aria-label={t('nav.sidebar.viewLabel')}>
      <button class:on={!app.boardOn} aria-pressed={!app.boardOn} onclick={() => app.closeBoard(project.id)}>{t('common.agents')}</button>
      <button class:on={app.boardOn} aria-pressed={app.boardOn} onclick={() => app.openBoard(project.id)}
        >Kanban{#if review}<span class="badge" use:tip={t('nav.sidebar.toReview', { count: review })}>{review}</span>{/if}</button
      >
    </div>
  </div>

  <div class="head">
    <span class="section-label">{t('common.agents')}</span>
    <span class="count">{app.projectAgents.length}</span>
    <div style="flex:1"></div>
    <div class="newgrp">
      <button
        class="new"
        onclick={() => app.newAgent(project.id)}
        use:tip={t('nav.sidebar.newAgentTitle', { shortcut: keyLabel('Ctrl+N') })}
      >
        <span class="plus">+</span>
        {t('nav.sidebar.newAgent')}
      </button>
      <button
        class="more"
        bind:this={moreBtn}
        aria-haspopup="menu"
        aria-label={t('branches.agent.moreWays')}
        use:tip={t('branches.agent.moreWays')}
        onclick={newAgentMenu}>▾</button
      >
    </div>
  </div>

  <div class="list">
    {#each app.projectAgents as a (a.id)}
      {@const sel = a.id === selectedAgentId && !otherView}
      {@const s = spent(a)}
      {@const tag = ticketTag(app.ticketOf(a.id))}
      {@const plan = planView(a)?.mini}
      {@const inBg = can(a, 'backgroundTasks') ? bgCount(a.plan) : 0}
      {@const remote = remoteStateOf(a)}
      {@const pending = pendingFor(a, app.agents, app.tickets, app.now, can(a, 'backgroundTasks'))}
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
          <EngineMark engine={a.engine} />
          {#if inBg > 0}
            <!-- What runs in the background for the agent: the list is under the card. -->
            <span
              class="bgchip"
              role="img"
              aria-label={t('bgtasks.chipLabel', { count: inBg })}
              use:tip={{ text: t('bgtasks.chipLabel', { count: inBg }), aria: false }}
              data-bg-chip
              data-count={inBg}>{t('bgtasks.chip', { count: inBg })}</span
            >
          {/if}
          {#if remote}
            <span class="rc" class:on={remote === 'connected'} class:warn={remote === 'stillOn'} use:tip={remoteTitle(a, remote)}>
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.2"
                stroke-linecap="round"
                aria-hidden="true"
                ><path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0" /><circle
                  cx="12"
                  cy="19.5"
                  r="1"
                  fill="currentColor"
                />{#if remote === 'stillOn'}<path class="bar" d="M4 4l16 16" />{/if}</svg
              >
            </span>
          {/if}
          {#if a.status === 'waiting'}
            <span class="pill">{statusLabel('waiting')}</span>
          {:else if a.resumeAt}
            <span class="status" style:color="var(--wait)" use:tip={t('nav.sidebar.resumeTitle', { when: fWhen(a.resumeAt, app.now) })}
              >{t('nav.sidebar.resumes', { when: fWhen(a.resumeAt, app.now) })}</span
            >
          {:else if a.setup}
            <span class="status" style:color="var(--wait)" use:tip={t('nav.sidebar.settingUpTitle', { step: a.setup })}
              >{t('nav.sidebar.settingUp')}</span
            >
          {:else if pending}
            <!-- Its turn is over, but what it left goes on (ESC-138): not « Terminé » yet. -->
            <span class="status" style:color={SC.running} use:tip={pendingText(pending, app.now)} data-pending
              >{pendingLabel(pending, app.now)}</span
            >
          {:else}
            <span class="status" style:color={SC[a.status]}>{statusLabel(a.status)}</span>
          {/if}
        </div>
        {#if a.progressLine}
          <!-- What the agent reported through Escouade's MCP server, until its next report. -->
          <div class="said" use:tip={{ text: a.progressLine, cut: true }}>
            <span class="sr">{t('mcp.progress.label')}{' '}</span>{a.progressLine}
          </div>
        {/if}
        <div class="meta">
          <span>{modelLabel(a.model, app.models)}</span><span class="sep">·</span><span>{duration(a)}</span>
          {#if a.worktree}<span class="sep">·</span><span class="wt" use:tip={a.worktree.branch}
              >⎇ {shortBranch(a.worktree.branch, a.worktree.existing ? '' : project.agentBranchPrefix)}</span
            >{:else if a.worktreePending}<span class="sep">·</span><span class="wt" use:tip={t('nav.sidebar.worktreeLaterTitle')}
              >{t('nav.sidebar.worktreeLater')}</span
            >{/if}
        </div>
        <div class="meta dim" use:tip={s.estimated ? estimateHint(s) : undefined}>
          <span>{t('nav.sidebar.tokens', { tokens: fTok(s.tokens) })}</span>{#if showsCost(a)}<span>{fSpentUsd(s)}</span>{/if}<span
            >{t('nav.sidebar.files', { count: git?.agents[a.id] ?? 0 })}</span
          >
        </div>
        {#if plan}
          <!-- How far the agent is through its plan: the bar says it at a glance, the words are read. -->
          <div class="plan-mini">
            <span class="plan-bar" aria-hidden="true"><span style:width="{plan.percent}%"></span></span>
            <span class="plan-text mono"
              >{plan.subs
                ? t('plan.miniSubs', { done: plan.done, total: plan.total, count: plan.subs })
                : t('plan.mini', { done: plan.done, total: plan.total })}</span
            >
          </div>
        {/if}
        {#if tag}
          <div class="ticket-tag mono">▸ {tag}</div>
        {/if}
      </div>
      {@const prs = cardPrs(a)}
      {#if prs.length}
        <!-- Under the card, not in it (the card is a button): its pull requests (ESC-64), the ones to merge first; a number
             opens it, the corrections go to the agent on a click only. -->
        {@const word = prWord(a)}
        <div class="prs" role="group" aria-label={t('prs.group', { name: a.name })} data-prs={a.id}>
          {#each prs as p (p.number)}
            <div class="pr" data-pr={p.number}>
              <button
                class="prnum mono {prTone(p)}"
                aria-label={t('prs.chipLabel', { word, number: p.number, state: stateWord(p) })}
                use:tip={{ text: prTip(word, p), aria: false }}
                onclick={() => openPr(p)}><span class="prglyph" aria-hidden="true">{prGlyph(p)}</span>#{p.number}</button
              >
              {#if !a.archived && canFixChecks(p)}
                <button
                  class="prfix"
                  use:tip={t('prs.fixChecksHint', { agent: a.name, word, number: p.number })}
                  onclick={() => fixPr(a, p, 'checks')}>{t('prs.fixChecks')}</button
                >
              {/if}
              {#if !a.archived && canFixReviews(p)}
                <button
                  class="prfix"
                  use:tip={t('prs.fixReviewsHint', { agent: a.name, word, number: p.number })}
                  onclick={() => fixPr(a, p, 'reviews')}>{t('prs.fixReviews')}</button
                >
              {/if}
            </div>
          {/each}
        </div>
      {/if}
      {@const subs = sidebarSubs(a)}
      {#if subs}
        <!-- Under the card, not in it (the card is a button): its subagents, unfolded while one runs. -->
        {@const open = app.subsOpen[a.id] ?? subs.running.length > 0}
        {@const doneOpen = finishedOpen[a.id] ?? false}
        <div class="subs" class:sel role="group" aria-label={t('subagents.toggle', { name: a.name })}>
          <button class="subs-toggle" aria-expanded={open} onclick={() => (app.subsOpen[a.id] = !open)}>
            <span class="chev" aria-hidden="true">{open ? '▾' : '▸'}</span>
            <span class="lbl">{t('subagents.title')}</span>
            {#if subs.running.length}<span class="cnt run">{t('subagents.runningCount', { count: subs.running.length })}</span>{/if}
            {#if subs.finished.length}<span class="cnt">{t('subagents.finishedCount', { count: subs.finished.length })}</span>{/if}
          </button>
          {#if open}
            <!-- The role is said again: a list with no bullets is a list no more for the web views of macOS. -->
            <!-- svelte-ignore a11y_no_redundant_roles -->
            <ul class="sub-list" role="list">
              {#each runningAll[a.id] ? subs.running : subs.running.slice(0, RUNNING_SHOWN) as s (s.id)}
                {@render subRow(a, s)}
              {/each}
              {#if subs.running.length > RUNNING_SHOWN}
                {@render moreRows(
                  runningAll[a.id] ?? false,
                  t('subagents.moreRunning', { count: subs.running.length - RUNNING_SHOWN }),
                  () => (runningAll[a.id] = !runningAll[a.id]),
                )}
              {/if}
              {#if subs.finished.length}
                <li class="finished">
                  <button class="done-toggle" aria-expanded={doneOpen} onclick={() => (finishedOpen[a.id] = !doneOpen)}>
                    <span class="chev" aria-hidden="true">{doneOpen ? '▾' : '▸'}</span>{t('subagents.finishedToggle', {
                      count: subs.finished.length,
                    })}
                  </button>
                  {#if doneOpen}
                    <!-- svelte-ignore a11y_no_redundant_roles -->
                    <ul class="sub-list done" role="list">
                      {#each finishedAll[a.id] ? subs.finished : subs.finished.slice(0, FINISHED_SHOWN) as s (s.id)}
                        {@render subRow(a, s)}
                      {/each}
                      {#if subs.finished.length > FINISHED_SHOWN}
                        {@render moreRows(
                          finishedAll[a.id] ?? false,
                          t('subagents.moreFinished', { count: subs.finished.length - FINISHED_SHOWN }),
                          () => (finishedAll[a.id] = !finishedAll[a.id]),
                        )}
                      {/if}
                    </ul>
                  {/if}
                </li>
              {/if}
            </ul>
          {/if}
        </div>
      {/if}
      {@const bg = can(a, 'backgroundTasks') ? sidebarBg(a.plan, { running: bgRunningAll[a.id], finished: bgFinishedAll[a.id] }) : null}
      {#if bg}
        <!-- Under the card too, after the subagents: the tasks Claude Code runs in the background, unfolded while one runs. -->
        {@const open = app.bgOpen[a.id] ?? bg.runningTotal > 0}
        {@const doneOpen = bgFinishedOpen[a.id] ?? false}
        <div class="subs bgs" class:sel role="group" aria-label={t('bgtasks.toggle', { name: a.name })} data-bg-group={a.id}>
          <button class="subs-toggle" aria-expanded={open} onclick={() => (app.bgOpen[a.id] = !open)}>
            <span class="chev" aria-hidden="true">{open ? '▾' : '▸'}</span>
            <span class="lbl">{t('bgtasks.title')}</span>
            {#if bg.runningTotal}<span class="cnt run">{t('bgtasks.runningCount', { count: bg.runningTotal })}</span>{/if}
            {#if bg.finishedTotal}<span class="cnt">{t('bgtasks.finishedCount', { count: bg.finishedTotal })}</span>{/if}
          </button>
          {#if open}
            <!-- svelte-ignore a11y_no_redundant_roles -->
            <ul class="sub-list" role="list">
              {#each bg.running as r (r.id)}
                {@render bgItem(a, r)}
              {/each}
              {#if bg.runningTotal > BG_RUNNING_SHOWN}
                {@render moreRows(
                  bgRunningAll[a.id] ?? false,
                  t('bgtasks.moreRunning', { count: bg.runningTotal - BG_RUNNING_SHOWN }),
                  () => (bgRunningAll[a.id] = !bgRunningAll[a.id]),
                  t('bgtasks.lessList'),
                )}
              {/if}
              {#if bg.finishedTotal}
                <li class="finished">
                  <button class="done-toggle" aria-expanded={doneOpen} onclick={() => (bgFinishedOpen[a.id] = !doneOpen)}>
                    <span class="chev" aria-hidden="true">{doneOpen ? '▾' : '▸'}</span>{t('bgtasks.finishedToggle', {
                      count: bg.finishedTotal,
                    })}
                  </button>
                  {#if doneOpen}
                    <!-- svelte-ignore a11y_no_redundant_roles -->
                    <ul class="sub-list done" role="list">
                      {#each bg.finished as r (r.id)}
                        {@render bgItem(a, r)}
                      {/each}
                      {#if bg.finishedTotal > BG_FINISHED_SHOWN}
                        {@render moreRows(
                          bgFinishedAll[a.id] ?? false,
                          t('bgtasks.moreFinished', { count: bg.finishedTotal - BG_FINISHED_SHOWN }),
                          () => (bgFinishedAll[a.id] = !bgFinishedAll[a.id]),
                          t('bgtasks.lessList'),
                        )}
                      {/if}
                    </ul>
                  {/if}
                </li>
              {/if}
            </ul>
          {/if}
        </div>
      {/if}
      {@const driven = drivenAgents(a.plan, app.agents, app.tickets, a.id)}
      {#if driven.length}
        <!-- Under the card too (ESC-138): the agents of Escouade it set to work through the MCP server, while they work. -->
        {@const open = drivenOpen[a.id] ?? true}
        <div class="subs" class:sel role="group" aria-label={t('bgtasks.later.drivenToggle', { name: a.name })} data-driven-group={a.id}>
          <button class="subs-toggle" aria-expanded={open} onclick={() => (drivenOpen[a.id] = !open)}>
            <span class="chev" aria-hidden="true">{open ? '▾' : '▸'}</span>
            <span class="lbl">{t('bgtasks.later.drivenTitle')}</span>
            <span class="cnt run">{t('bgtasks.later.drivenCount', { count: driven.length })}</span>
          </button>
          {#if open}
            <!-- svelte-ignore a11y_no_redundant_roles -->
            <ul class="sub-list" role="list">
              {#each driven as d (d.id)}
                <li>
                  <button class="sub" aria-label={t('bgtasks.later.drivenOpen', { name: d.name })} onclick={() => app.selectAgent(d.id)}>
                    <span class="sdot running" aria-hidden="true"></span>
                    <span class="stext"
                      ><span class="sline"><span class="stitle">{d.name}</span><span class="sstate">{statusLabel(d.status)}</span></span
                      ></span
                    >
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
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
            {#if a.worktreeKept}
              <!-- Its ticket over, its worktree stayed: what it holds would be lost (ESC-143). -->
              <div class="kept">{a.worktreeKept}</div>
            {/if}
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
      <button class="plus-btn" bind:this={termMenuBtn} use:tip={t('nav.sidebar.newTerminal')} onclick={shellMenu}>+</button>
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
            use:tip={t('common.close')}
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
    <div class="path mono" use:tip={project.path}>{tildify(project.path)}</div>
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
    oncreate={(name) => startAgentOn(project.id, name, true)}
    oncreatefrom={(name) => pickBase(name)}
  />
{/if}

{#if pickingBase}
  <BranchPicker
    projectId={project.id}
    anchor={moreBtn}
    mode="base"
    forBranch={pickingBase.branch ?? undefined}
    onclose={() => (pickingBase = null)}
    onpick={startFrom}
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
    .plan-bar span {
      transition: none;
    }
  }
  .card:focus-visible {
    border-color: var(--accent);
  }
  .card.archived .kept {
    margin-top: 3px;
    font-size: 11px;
    line-height: 1.35;
    color: var(--wait);
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
  /* Off for Escouade, still linked in the interface left open: barred, in the colour of the other warnings. */
  .rc.warn {
    color: var(--wait);
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
  .plan-mini {
    margin-left: 17px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .plan-bar {
    flex: 1;
    height: 3px;
    border-radius: 2px;
    background: var(--elev2);
    overflow: hidden;
  }
  .plan-bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 1s linear;
  }
  .plan-text {
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
  }
  /* The pull requests under a card (ESC-64): their numbers, colored by what to look at first, and the corrections. */
  .prs {
    margin: -2px 0 4px 29px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .pr {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .prnum,
  .prfix {
    height: 22px;
    padding: 0 7px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: transparent;
    font: inherit;
    font-size: 11px;
    cursor: pointer;
  }
  .prnum {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-family: var(--mono);
    color: var(--muted);
  }
  .prnum.ok .prglyph {
    color: var(--ok);
  }
  .prnum.fail .prglyph {
    color: var(--del);
  }
  .prnum.wait .prglyph {
    color: var(--wait);
  }
  .prnum.merged .prglyph {
    color: var(--info);
  }
  .prnum.dim {
    color: var(--dim);
  }
  .prfix {
    color: var(--text);
  }
  .prnum:hover,
  .prfix:hover {
    background: color-mix(in oklch, var(--elev) 55%, transparent);
    color: var(--text);
  }
  /* The subagents under a card: a line that folds them, then their rows, indented like the card's lines. A list shows its
     first rows and a button unrolls the others: it never scrolls inside the list of the agents, which scrolls already. */
  .subs {
    margin: -2px 0 4px 29px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .subs-toggle,
  .done-toggle,
  .more-toggle {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 24px;
    padding: 0 6px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 11.5px;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .subs-toggle:hover,
  .done-toggle:hover,
  .more-toggle:hover {
    color: var(--text);
    background: color-mix(in oklch, var(--elev) 55%, transparent);
  }
  .subs-toggle .lbl {
    font-weight: 600;
  }
  .chev {
    width: 9px;
    font-size: 10px;
    color: var(--dim);
  }
  .cnt {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
  .cnt.run {
    color: var(--ok);
  }
  .sub-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .sub-list.done {
    margin-left: 10px;
  }
  .finished {
    display: flex;
    flex-direction: column;
  }
  .sub {
    width: 100%;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 5px 6px 5px calc(6px + var(--depth, 0) * 12px);
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .sub:hover:not(.on) {
    background: color-mix(in oklch, var(--elev) 55%, transparent);
  }
  .sub.on {
    background: var(--elev);
    border-color: var(--line2);
  }
  .sub:focus-visible,
  .subs-toggle:focus-visible,
  .done-toggle:focus-visible,
  .more-toggle:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .sdot {
    flex: none;
    width: 6px;
    height: 6px;
    margin-top: 5px;
    border-radius: 50%;
    background: var(--dim);
  }
  .sdot.running {
    background: var(--ok);
  }
  .sdot.failed {
    background: var(--del);
  }
  .sdot.stopped,
  .sdot.interrupted {
    background: var(--wait);
  }
  .stext {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    flex: 1;
  }
  .sline {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .stitle {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .smodel,
  .sstate {
    flex: none;
    font-size: 10.5px;
    color: var(--dim);
  }
  .sdoing {
    font-size: 11px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A background task's time, live while it runs: beside its title. */
  .sdur {
    flex: none;
    font-size: 10.5px;
    color: var(--dim);
  }
  /* What runs in the background for an agent, on its card: it gives way to the name when the room is short. */
  .bgchip {
    flex: none;
    height: 16px;
    padding: 0 6px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--ok) 14%, transparent);
    color: var(--ok);
    font-family: var(--mono);
    font-size: 10.5px;
    line-height: 16px;
    white-space: nowrap;
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

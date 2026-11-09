<script module lang="ts">
  /** The last ask of Ctrl+J for a card (`app.cardFocus.n`) one sidebar took: the one of a project just shown takes it too. */
  let cardFocusTaken = 0;
</script>

<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { itemKey, leaves, type PaneItem } from '../lib/panes';
  import { keyLabel } from '../lib/platform';
  import { SIDE_MIN, sideWidth } from '../lib/side-width.svelte';
  import Splitter from './Splitter.svelte';
  import DocsSidebar from './docs/DocsSidebar.svelte';
  import { t } from '../lib/i18n';
  import { branchSummary, deltaDetail, repoLabel } from '../lib/repos';
  import { api } from '../lib/ipc';
  import { dropPlace, landing, stepPlace, type Place } from '../lib/agent-folders';
  import { fDur, fUsd, fWhen, tildify } from '../lib/format';
  import { accountName, defaultEngine, newAgentAccount, newAgentDefaults } from '../lib/accounts';
  import {
    cardActivity,
    cardSpend,
    listChoices,
    listDigit,
    listRequest,
    listStep,
    listTarget,
    foldedCards,
    shownAgents,
    type SidebarItem,
    type ListDecision,
    type ListRequest,
  } from '../lib/sidebar';
  import { HOLD_MS } from '../lib/answer-keys.svelte';
  import { answerOf, draftIfAny, draftOf, dropDraft } from '../lib/question-answers.svelte';
  import { revealHidden } from '../lib/recipe';
  import { toolLabel } from '../lib/tools';
  import { canSwitchView, copyRemoteLink, inHarnessView, integrateBase, openRemote, switchView, toggleRemote } from '../lib/agent-actions';
  import { can, canFork, engineName, offered, speakerOf } from '../lib/engines';
  import { startAgentOn } from '../lib/branch-actions';
  import {
    canFixChecks,
    canFixReviews,
    cardPrs,
    fixPr,
    openPr,
    prIcon,
    prKey,
    prMenuItem,
    prTip,
    prTone,
    prWord,
    stateWord,
  } from '../lib/prs';
  import { runIn } from '../lib/launch-actions';
  import { shortBranch } from '../lib/branches';
  import { buffers, lossNotice } from '../lib/editor/buffers.svelte';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { modeLabel, modelLabel, MODES, planResume } from '../lib/models';
  import HeatFigure from './HeatFigure.svelte';
  import { planView } from '../lib/plan';
  import { cardNext, cardStep } from '../lib/workflow-view';
  import { workflows } from '../lib/workflows.svelte';
  import { sidebarSubs, type SubRow } from '../lib/subagents';
  import { BG_FINISHED_SHOWN, BG_RUNNING_SHOWN, bgCount, bgDuration, rowLine, rowTitle, sidebarBg, type BgRow } from '../lib/bgtasks';
  import { estimateHint, spent } from '../lib/spend';
  import { drivenAgents, pendingFor, pendingLabel, pendingText } from '../lib/later';
  import { app, type HeaderFolded } from '../lib/state.svelte';
  import { hasTab } from '../lib/code-host';
  import { shortWindow } from '../lib/short-window.svelte';
  import { closeTerminal, newTerminal, terminalIn } from '../lib/term-actions';
  import { canPrepare, prepareLaunch, stopTests } from '../lib/test-launch.svelte';
  import { returnHeld } from '../lib/outbox.svelte';
  import { backTarget, forwardTargets } from '../lib/handover';
  import type { Agent, AgentFolder, AgentStatus, BranchInfo, EngineId, Project } from '../lib/types';
  import RunsSection from './RunsSection.svelte';
  import EngineMark from './EngineMark.svelte';
  import StatusDot from './StatusDot.svelte';
  import FoldTitle from './FoldTitle.svelte';
  import { ciShown } from '../lib/branch-ci';
  import CiPill from './CiPill.svelte';
  import Icon from './Icon.svelte';
  import BranchPicker from './branches/BranchPicker.svelte';
  import { tip } from '../lib/tip';

  let { project }: { project: Project } = $props();

  // The workflows of the project and where its agents are in them, for the step on their cards (ESC-197): read again
  // when its corrections change, as the Workflow tab does.
  const workflowFixes = $derived(JSON.stringify(project.workflowFixes ?? []));
  $effect(() => {
    const id = project.id;
    void workflowFixes;
    untrack(() => void workflows.follow(id));
  });

  /** What an agent is doing, read when shown (the language of the interface can change). */
  const statusLabel = (status: AgentStatus) => t(`nav.sidebar.status.${status}`);
  /** A shortcut as the keys of the preview write it: « Ctrl N », « Ctrl Maj N ». */
  const kbd = (s: string) => keyLabel(s).split('+').join(' ');

  let termMenuBtn = $state<HTMLButtonElement>();
  // Terminaux, folded to its header in a short window (choice 9) until its title unfolds it.
  let termsUnfolded = $state(false);
  const termsFolded = $derived(shortWindow.now && !termsUnfolded);
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
  /** The agent's pull request in the side panel's « PR et CI » tab (ESC-305): its CI, its log, its reviews. */
  function showCi(id: string) {
    app.selectAgent(id);
    app.panelTab = 'hosts';
    if (!app.split) app.filesOpen = true;
  }

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
  const otherView = $derived(!!termSelected || !!app.runCommand || app.boardOn || app.docsOn || app.prsOn);
  /** The contents of the split screen's panes (ESC-392), when it is on screen: each is highlighted, the active one more. */
  const paneKeys = $derived(app.panesOn && app.panes ? new Set(leaves(app.panes.root).map(itemKey)) : null);
  const inPane = (key: string) => !!paneKeys?.has(key) && app.panes?.active !== key;

  /** « Ouvrir à côté », « Ouvrir en dessous » (ESC-392): what a card's or a row's menu offers. */
  function besideItems(item: PaneItem): MenuItem[] {
    return [
      {
        label: t('shell.panes.openBeside'),
        hint: `${keyLabel('Ctrl')}+${t('shell.panes.click')}`,
        onClick: () => app.openBeside(item, 'right'),
      },
      { label: t('shell.panes.openBelow'), onClick: () => app.openBeside(item, 'bottom') },
    ];
  }

  /** A click on a card or a row: with Ctrl (Cmd), beside the content on screen. */
  function open(e: MouseEvent, item: PaneItem, select: () => void) {
    if (e.ctrlKey || e.metaKey) app.openBeside(item, 'right');
    else select();
  }
  const review = $derived(app.reviewCount(project.id));
  /** The word of the project's code host (« PR », « MR » on GitLab): read by its view, else by one of its agents (ESC-65). */
  const prsWord = $derived(app.prWords[project.id] ?? app.projectAgents.find((a) => a.prWord)?.prWord ?? 'PR');
  /** The « PR » tab only for a project whose remote is on a code host whose plugin is on (ESC-390). */
  const prsTab = $derived(hasTab(app.codeHosts[project.id], 'pullRequests'));
  /** The project's pull requests come since its PR tab was last opened (ESC-403). */
  const prsNew = $derived(app.prsNew(project.id));
  // Its code host read when the project shows, and again when the window comes back (its remote may have changed).
  $effect(() => {
    const id = project.id;
    untrack(() => void app.readCodeHost(id));
  });
  /** Who waits for an answer goes first (2.0 design). */
  const agents = $derived(app.projectAgents);
  /** What the list shows: folders and loose agents (`agents`, flat, is the same in shown order: the keys' and Ctrl+J's). */
  const items = $derived(app.projectItems);
  type FolderItem = Extract<SidebarItem, { kind: 'folder' }>;
  /** The requests answered from the list, until the agent's request is gone: one click answers once. */
  let answering = $state<Record<string, string>>({});
  /** The cards that flash green a moment, their agent sent off again from the list. */
  let flashing = $state<Record<string, boolean>>({});
  /**
   * When each request that can be answered from the list was first shown: its keys wait `HOLD_MS` (as the cards of the
   * conversation do), so that a digit pressed as it comes does not answer it unread.
   */
  const shownAt = new Map<string, number>();
  /** When a question of several moved on to the next one: its buttons wait `HOLD_MS`, for a double-click not to answer it too. */
  const stepAt = new Map<string, number>();
  $effect.pre(() => {
    // A request answered from here and gone (the agent went on): its card answers the next one, even under the same id.
    for (const [id, req] of Object.entries(answering)) {
      const a = app.agents[id];
      if (a?.status === 'waiting' && a.requests?.[0]?.id === req) continue;
      delete answering[id];
      dropDraft(req);
      shownAt.delete(req);
      stepAt.delete(req);
    }
    for (const a of agents) {
      const r = listRequest(a);
      if (r && !shownAt.has(r.requestId)) shownAt.set(r.requestId, performance.now());
    }
  });

  // Ctrl+J (`app.nextWaiting`): the card of the agent that waits gets the focus, the conversation open staying.
  $effect(() => {
    const { id, n } = app.cardFocus;
    if (n <= cardFocusTaken) return;
    cardFocusTaken = n;
    tick().then(() => document.querySelector<HTMLElement>(`.agent[data-agent="${CSS.escape(id)}"]`)?.focus());
  });

  // Ctrl+Shift+N (lib/shortcuts.ts): the menu « Harness » opens beside « Nouvel agent ».
  let menuAsked = app.newAgentMenu;
  $effect(() => {
    if (app.newAgentMenu === menuAsked) return;
    menuAsked = app.newAgentMenu;
    if (!app.boardOn && !app.docsOn && !app.prsOn) newAgentMenu();
  });

  // « … » of an agent's header (Conversation): its menu, the one of its card, where the button is; the keyboard goes in.
  let agentMenuSeq = app.agentMenuAsk?.seq ?? 0;
  $effect(() => {
    const ask = app.agentMenuAsk;
    if (!ask || ask.seq === agentMenuSeq) return;
    agentMenuSeq = ask.seq;
    const a = app.agents[ask.id];
    if (!a) return;
    const at = { clientX: ask.x, clientY: ask.y, preventDefault() {}, stopPropagation() {} } as MouseEvent;
    untrack(() => {
      agentMenu(at, a, ask.folded);
      if (menu.open) menu.open.keyboard = true;
    });
  });

  /** A permission's decisions, as its card in the conversation names them. */
  const DECISIONS = {
    allow: 'composer.permission.allow',
    always: 'composer.permission.always',
    session: 'composer.permission.forSession',
    agent: 'composer.permission.forAgent',
    deny: 'composer.permission.deny',
  } as const;

  /** The question of `r` the card shows: the one to answer, the last one once all have their choice (on its way). */
  function shownStep(r: ListRequest & { kind: 'question' }): number {
    const step = listStep(r, draftIfAny(r.requestId)?.picks ?? {});
    return step < 0 ? r.questions.length - 1 : step;
  }

  /** The choices of the card, numbered from 1: the options of the question it shows, or the permission's decisions. */
  function choiceLabels(r: ListRequest): string[] {
    if (r.kind === 'question') return r.questions[shownStep(r)].options;
    return (listChoices(r, {}) as ListDecision[]).map((d) => t(DECISIONS[d]));
  }

  /**
   * Answers the request `a` waits on with its choice `i`, from its card: its conversation, open or not, stays where it
   * is. A question of several is answered one after the other, the choices kept with the conversation's (its card shows
   * them), and sent with the last one. Once sent, the card flashes green and a toast says what was answered.
   */
  async function pick(a: Agent, r: ListRequest, i: number, click = false) {
    // A click just after the next question came: the second of a double-click. Keys held down never come (`listDigit`).
    if (answering[a.id] === r.requestId || (click && performance.now() - (stepAt.get(r.requestId) ?? -Infinity) < HOLD_MS)) return;
    if (r.kind === 'permission') {
      const d = (listChoices(r, {}) as ListDecision[])[i];
      if (d) await send(a, r, api.answerPermission(a.id, r.requestId, d === 'session' || d === 'agent' ? 'always' : d), t(DECISIONS[d]));
      return;
    }
    const step = listStep(r, draftIfAny(r.requestId)?.picks ?? {});
    const q = r.questions[step];
    const choice = q?.options[i];
    if (choice === undefined) return;
    // Read again once made: the reactive draft, which the conversation's card shows too.
    draftOf(r.requestId);
    const draft = draftIfAny(r.requestId)!;
    draft.picks[q.question] = [choice];
    delete draft.texts[q.question];
    if (listStep(r, draft.picks) >= 0) {
      stepAt.set(r.requestId, performance.now());
      return;
    }
    const answers = Object.fromEntries(r.questions.map((x) => [x.question, answerOf(draft, x.question)]));
    await send(a, r, api.answerQuestion(a.id, r.requestId, answers), Object.values(answers).join(' · '));
  }

  async function send(a: Agent, r: ListRequest, answer: Promise<unknown>, said: string) {
    answering[a.id] = r.requestId;
    if (!(await app.run(answer.then(() => true)))) {
      delete answering[a.id];
      return;
    }
    flashing[a.id] = true;
    setTimeout(() => delete flashing[a.id], 900);
    app.toast(t('nav.sidebar.answered', { name: a.name, choice: said }), 'ok');
  }

  /** Enter opens a card, as a click does; Alt+↑ / Alt+↓ move it one step (into a folder, out of it at its edges). */
  function cardKey(e: KeyboardEvent, a: Agent) {
    if (e.target !== e.currentTarget) return;
    if (e.key === 'Enter') app.selectAgent(a.id);
    else stepKey(e, a.id, `.agent[data-agent="${CSS.escape(a.id)}"]`);
  }

  /** The folder an agent is in, null when it is loose. */
  const folderOf = (id: string) =>
    items.find((it): it is FolderItem => it.kind === 'folder' && (it.principal?.id === id || it.agents.some((a) => a.id === id)));

  /** The ids of a level as it is drawn (null: the project's): what a drop line can sit before. */
  function rowIds(level: string | null): string[] {
    if (!level) return items.map((it) => (it.kind === 'agent' ? it.agent.id : it.folder.id));
    const f = items.find((it): it is FolderItem => it.kind === 'folder' && it.folder.id === level);
    if (!f) return [];
    return (f.folder.folded ? foldedCards(f, selectedAgentId) : [...(f.principal ? [f.principal] : []), ...f.agents]).map((a) => a.id);
  }

  const isFolder = (id: string) => items.some((it) => it.kind === 'folder' && it.folder.id === id);

  const move = (id: string, p: Place) => app.run(api.agentFolderMove(project.id, id, p.folderId, p.index ?? undefined));

  /** Alt+↑ / Alt+↓ on a card or a folder's chevron: one step; the focus stays on what moved, wherever it is drawn next. */
  async function stepKey(e: KeyboardEvent, id: string, focus: string) {
    if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
    e.preventDefault();
    const p = stepPlace(project, items, id, e.key === 'ArrowUp' ? -1 : 1);
    if (!p) return;
    await move(id, p);
    await tick();
    // In a folded folder the card is not drawn: the focus goes to the folder's chevron.
    const chev = p.folderId ? `[data-folder="${CSS.escape(p.folderId)}"] .chev` : focus;
    (document.querySelector<HTMLElement>(focus) ?? document.querySelector<HTMLElement>(chev))?.focus();
  }

  // Dragging, by the pointer gesture of the split screen (paneDrag): over the list a line shows where it lands, a folder's
  // header lights up when an agent hovers it (it goes in at the bottom); released on the work area, a card opens in a pane.
  /** The drop line: sits before the drawn row `before` of level `level` (a folder's id; null: the project's), null at its end. */
  let drop = $state<{ level: string | null; before: string | null } | null>(null);
  let overFolder = $state<string | null>(null);
  /** The agent or folder being dragged (wherever the pointer is). */
  const dragging = $derived(paneDrag.on ? paneDrag.sidebarId : null);
  /** Where the dragged item would land, null when it would not move or may not go there: no line then. */
  const place = $derived(dragging && drop ? dropPlace(project, items, dragging, drop.level, drop.before) : null);
  /** The drawn row the line sits before: where the item lands (`landing`), not the row hovered when it is one that moved up. */
  const lineBefore = $derived.by(() => {
    if (!dragging || !drop) return null;
    const next = landing(items, dragging, drop.level, drop.before);
    return next && rowIds(drop.level).includes(next) ? next : null;
  });

  /** Over a drawn row (an agent's card or a folder's header at the project level): the line goes above it or under it. */
  function aimRow(id: string, r: DOMRect, y: number) {
    const level = folderOf(id)?.folder.id ?? null;
    overFolder = null;
    const row = rowIds(level);
    drop = { level, before: y < r.top + r.height / 2 ? id : (row[row.indexOf(id) + 1] ?? null) };
  }

  /**
   * Between the rows (the gap, or the free space of a list): the line goes before the first drawn row of `level` whose
   * middle is under the pointer, at the end of the level when none is.
   */
  function aimGap(level: string | null, y: number) {
    const rows = level
      ? document.querySelectorAll<HTMLElement>(`[data-folder="${CSS.escape(level)}"] .folder-b > [data-row]`)
      : document.querySelectorAll<HTMLElement>('.agents > [data-row], .agents > section.folder');
    const below = [...rows].find((r) => {
      const b = r.getBoundingClientRect();
      return y < b.top + b.height / 2;
    });
    drop = { level, before: below ? (below.dataset.row ?? below.dataset.folder!) : null };
  }

  /** What the pointer at (x, y) would put where, from the element under it: the rules of the list, kept as they were. */
  function aim(id: string, x: number, y: number) {
    const dragsFolder = isFolder(id);
    drop = overFolder = null;
    for (const n of document.elementsFromPoint(x, y)) {
      const el = n.closest<HTMLElement>('[data-row], .folder-h, .folder-b');
      if (!el) continue;
      const folder = el.closest<HTMLElement>('[data-folder]')?.dataset.folder;
      if (el.matches('.folder-b')) {
        // A folder has no place inside another one.
        if (!dragsFolder && folder) aimGap(folder, y);
      } else if (el.matches('.folder-h')) {
        if (dragsFolder) aimRow(folder!, el.getBoundingClientRect(), y);
        else overFolder = folder ?? null;
      } else {
        const agentId = el.dataset.row!;
        // A folder goes among the project's rows only: inside another folder it has no place.
        if (!(dragsFolder && folderOf(agentId))) aimRow(agentId, el.getBoundingClientRect(), y);
      }
      return;
    }
    // The free space of the list, or a gap between its rows.
    aimGap(null, y);
  }

  $effect(() => {
    if (paneDrag.on && paneDrag.inSidebar && paneDrag.sidebarId) aim(paneDrag.sidebarId, paneDrag.x, paneDrag.y);
    else drop = overFolder = null;
  });

  /** Released over the list: on a folder's header an agent goes in at the bottom; elsewhere where the line is. */
  function dropHere(id: string) {
    const p =
      overFolder && !isFolder(id)
        ? dropPlace(project, items, id, overFolder, null)
        : drop && dropPlace(project, items, id, drop.level, drop.before);
    drop = overFolder = null;
    if (p) void move(id, p);
  }

  /**
   * The keys 1, 2… answer from the list wherever the focus is but a field (a terminal, the editor, a menu, a dialog keep
   * them): the agent whose card has the focus, else the first one of the list that can be answered there. Not over the
   * Kanban, and not a request just shown (`HOLD_MS`).
   */
  function listKey(e: KeyboardEvent) {
    if (e.defaultPrevented || app.modal || menu.open || app.boardOn || app.docsOn || app.prsOn || app.ui.view !== 'project') return;
    const target = e.target instanceof Element ? e.target : null;
    if (
      target?.closest(
        'input, textarea, select, [contenteditable]:not([contenteditable="false"]), .xterm, .cm-editor, [role="menu"], [role="dialog"]',
      )
    )
      return;
    const n = listDigit(e);
    if (n === null) return;
    const focused = document.activeElement?.closest('.agent[data-agent]')?.getAttribute('data-agent') ?? null;
    const a = listTarget(agents, focused);
    const r = a && listRequest(a);
    if (!a || !r || n >= choiceLabels(r).length) return;
    e.preventDefault();
    if (performance.now() - (shownAt.get(r.requestId) ?? 0) < HOLD_MS) return;
    pick(a, r, n);
  }

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

  function startFolderRename(f: AgentFolder) {
    renaming = f.id;
    renameValue = f.name;
  }

  async function commitFolderRename(f: AgentFolder) {
    // Enter then the blur of the removed input: rename once.
    if (renaming !== f.id) return;
    const v = renameValue.trim();
    renaming = null;
    if (v && v !== f.name) await app.run(api.agentFolderRename(project.id, f.id, v));
  }

  /** A new folder (holding `agentId`, if any), its name asked at once, once it is drawn. */
  async function newFolder(agentId?: string) {
    const f = await app.run(api.agentFolderCreate(project.id, agentId));
    if (f) startFolderRename(f);
  }

  /** « 2 en cours, 1 en attente »: what the pills of a folder's header say, for the screen readers. */
  const folderCounts = (it: FolderItem) =>
    [
      it.running && t('nav.sidebar.folders.running', { count: it.running }),
      it.waiting && t('nav.sidebar.folders.waiting', { count: it.waiting }),
    ]
      .filter(Boolean)
      .join(', ');

  /** Monter / Descendre, greyed at the ends of what `id` can step through. */
  function stepItems(id: string): MenuItem[] {
    return [
      { label: t('nav.rail.up'), hint: t('nav.rail.upKey'), disabled: !stepPlace(project, items, id, -1), onClick: () => stepMove(id, -1) },
      {
        label: t('nav.rail.down'),
        hint: t('nav.rail.downKey'),
        disabled: !stepPlace(project, items, id, 1),
        onClick: () => stepMove(id, 1),
      },
    ];
  }
  function stepMove(id: string, by: -1 | 1) {
    const p = stepPlace(project, items, id, by);
    if (p) void move(id, p);
  }

  /** What an agent's menu offers about folders: where to move it, a folder made of it, its star, a step up or down. */
  function folderItems(e: MouseEvent, a: Agent): MenuItem[] {
    if (a.archived) return [];
    const mine = folderOf(a.id);
    return [
      { label: t('nav.sidebar.folders.moveTo'), onClick: () => moveMenu(e, a) },
      ...(mine
        ? [
            {
              label: t(mine.principal?.id === a.id ? 'nav.sidebar.folders.removePrincipal' : 'nav.sidebar.folders.makePrincipal'),
              onClick: () => app.run(api.agentFolderPrincipal(project.id, mine.folder.id, mine.principal?.id === a.id ? null : a.id)),
            },
          ]
        : [{ label: t('nav.sidebar.folders.createWith'), onClick: () => newFolder(a.id) }]),
      ...stepItems(a.id),
    ];
  }

  /** « Déplacer vers… »: each other folder, a new one, out of any, where the agent's menu was (no submenus: a second menu). */
  async function moveMenu(e: MouseEvent, a: Agent) {
    const at = { x: e.clientX, y: e.clientY };
    const mine = folderOf(a.id)?.folder.id ?? null;
    await tick();
    menu.open = {
      ...at,
      items: [
        ...items
          .filter((it): it is FolderItem => it.kind === 'folder' && it.folder.id !== mine)
          .map((it) => ({ label: it.folder.name, onClick: () => move(a.id, { folderId: it.folder.id, index: null }) })),
        { label: t('nav.sidebar.folders.create'), onClick: () => newFolder(a.id) },
        ...(mine ? [{ label: t('nav.sidebar.folders.outOf'), onClick: () => move(a.id, { folderId: null, index: null }) }] : []),
      ],
    };
  }

  /** The menu of a folder's header. */
  function folderMenu(e: MouseEvent, f: AgentFolder) {
    menu.show(e, [
      { label: t('common.rename'), onClick: () => startFolderRename(f) },
      ...stepItems(f.id),
      { label: '', separator: true },
      { label: t('nav.sidebar.folders.delete'), danger: true, onClick: () => app.run(api.agentFolderDelete(project.id, f.id)) },
    ]);
  }

  /** `folded`: what the header of its conversation, too narrow, no longer shows (ESC-166), offered here instead. */
  function agentMenu(e: MouseEvent, a: Agent, folded?: HeaderFolded) {
    // A copy is made from a conversation at rest: during a turn, the original's session moves on.
    const turn = a.status === 'running' || a.status === 'waiting';
    menu.show(e, [
      { label: t('common.rename'), onClick: () => startRename(a) },
      ...folderItems(e, a),
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
      ...(a.archived ? [] : besideItems({ kind: 'agent', id: a.id })),
      ...foldedItems(folded),
      ...handoverItems(e, a),
      ...remoteItems(a),
      { label: '', separator: true },
      { label: t('nav.sidebar.menu.delete'), danger: true, onClick: () => confirmDelete(a) },
    ]);
  }

  /** The files beside the conversation and its layout, when its header folded them away. */
  function foldedItems(folded?: HeaderFolded): MenuItem[] {
    const items: MenuItem[] = [];
    if (folded?.files)
      items.push({
        label: t(app.filesOpen ? 'conv.header.filesClose' : 'conv.header.filesHint'),
        hint: keyLabel('Ctrl+Shift+B'),
        onClick: () => (app.filesOpen = !app.filesOpen),
      });
    if (folded?.layout)
      items.push({
        label: t(app.split ? 'conv.header.layoutClassic' : 'conv.header.layoutSplit'),
        hint: keyLabel('Ctrl+Shift+L'),
        onClick: () => app.toggleLayout(),
      });
    return items;
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
      // What it holds goes back to its draft, unsent.
      returnHeld(a.id);
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

  /**
   * The menu « Harness » beside « Nouvel agent » (choices 1 and 16 of the 2.0 design): each harness set up (found, with an
   * account), in its order, numbered, with the account, model and mode its new agent would take; none set up, no heading.
   * Then the agent the other way round from « Un worktree par agent », the one on a branch, and the project's default harness.
   */
  async function newAgentMenu() {
    if (!moreBtn) return;
    // A harness installed since is offered (ESC-186).
    await app.refreshEngines();
    if (!moreBtn) return;
    const own = project.engine ?? defaultEngine();
    // Those that can be used (ESC-391): found, their plugin on, with an account; not one turned off for the project (ESC-404).
    const setUp = offered(project.id);
    const harnesses: MenuItem[] = setUp.map((e, i) => {
      const account = newAgentAccount(project, e.id);
      const d = newAgentDefaults(project, e.id);
      const detail = [
        e.id === own ? t('nav.sidebar.projectHarness') : '',
        account ? accountName(account) : '',
        d.model ? modelLabel(d.model, app.models) : '',
        MODES.find((m) => m.value === d.mode)?.label ?? '',
      ];
      return {
        label: e.name,
        engine: e.id,
        detail: detail.filter(Boolean).join(' · '),
        key: String(i + 1),
        checked: e.id === own,
        onClick: () => app.newAgent(project.id, e.id),
      };
    });
    menu.showAt(moreBtn, [
      ...(harnesses.length
        ? [{ label: t('nav.sidebar.harness'), heading: true, hint: kbd('Ctrl+Shift+N') }, ...harnesses, { label: '', separator: true }]
        : []),
      { label: t('nav.sidebar.worktree'), heading: true },
      // The other way round from « Un worktree par agent »: without one in a project that gives one, in one otherwise.
      project.worktreePerAgent
        ? {
            label: t('nav.sidebar.newAgentNoWorktree'),
            icon: 'folder',
            // Of several repositories (ESC-169): it sees them all, from the project's root.
            detail: project.repos?.length
              ? t('repos.agent.bare', { count: project.repos.length })
              : t('nav.sidebar.newAgentNoWorktreeDetail'),
            onClick: () => app.newAgent(project.id, null, false),
          }
        : {
            label: t('nav.sidebar.newAgentInWorktree'),
            icon: 'branch',
            detail: project.repos?.length
              ? t('repos.agent.note', { count: project.repos.length })
              : t('nav.sidebar.newAgentInWorktreeDetail'),
            onClick: () => app.newAgent(project.id, null, true),
          },
      {
        label: t('branches.agent.onBranch'),
        icon: 'branch',
        // Once the menu is gone and has given the focus back to its button: the picker gives it back there in turn.
        onClick: async () => {
          await tick();
          pickingBranch = true;
        },
      },
      // In a worktree that starts from a branch chosen next, whatever the project's starting branch (ESC-160).
      {
        label: t('nav.sidebar.newAgentFrom'),
        icon: 'merge',
        onClick: async () => {
          await tick();
          pickBase(null);
        },
      },
      { label: '', separator: true },
      { label: t('nav.sidebar.folders.create'), icon: 'folder', onClick: () => newFolder() },
      { label: '', separator: true },
      {
        label: t('nav.sidebar.defaultHarness'),
        icon: 'sliders',
        onClick: () => (app.modal = { kind: 'settings', page: 'project', projectId: project.id }),
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
      ...app.shells.map((s) => ({ label: s.label, icon: 'term' as const, onClick: () => newTerminal(project.id, s.id) })),
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
          {#if s.model}<span class="smodel">{modelLabel(s.model, app.models)}</span>{/if}
          {#if s.status !== 'running'}<span class="sstate">{t(`subagents.status.${s.status}`)}</span>{/if}
        </span>
        {#if s.status === 'running' && s.doing}<span class="sdoing">{s.doing}</span>{/if}
      </span>
    </button>
  </li>
{/snippet}

<!-- The chevron of a list that folds: down when open. -->
{#snippet chev(open: boolean)}
  <span class="chev" aria-hidden="true"><Icon name={open ? 'down' : 'right'} size={12} /></span>
{/snippet}

<!-- The line where a dragged agent or folder lands: before the row `before` of its level, null at the end of it. -->
{#snippet line(level: string | null, before: string | null)}
  {#if place && drop && drop.level === level && lineBefore === before}<div class="drop" aria-hidden="true"></div>{/if}
{/snippet}

<!-- The button of a list that shows only its first rows: it unrolls the others, then folds them back (the focus stays). -->
{#snippet moreRows(all: boolean, label: string, onclick: () => void, less: string = t('subagents.lessList'))}
  <li>
    <button class="more-toggle" aria-expanded={all} {onclick}>
      {@render chev(all)}{all ? less : label}
    </button>
  </li>
{/snippet}

<!-- The choices of the request `r` the agent `a` waits on, numbered as their keys 1, 2…: a permission's « Autoriser » first. -->
{#snippet choices(a: Agent, r: ListRequest)}
  {@const labels = choiceLabels(r)}
  <!-- A permission's three decisions, too wide for one line in French, one under the other. -->
  <div
    class="opts"
    class:stack={r.kind === 'permission' && labels.length > 2}
    role="group"
    aria-label={t('nav.sidebar.answers', { name: a.name })}
  >
    {#each labels as o, i (i)}
      <button
        class="opt"
        class:primary={r.kind === 'permission' && i === 0}
        disabled={answering[a.id] === r.requestId}
        onclick={(e) => {
          e.stopPropagation();
          pick(a, r, i, true);
        }}><kbd>{i + 1}</kbd>{o}</button
      >
    {/each}
  </div>
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
          <span class="stitle mono">{rowTitle(r)}</span>
          {#if ms !== null}<span class="sdur">{fDur(ms)}</span>{/if}
        </span>
        <span class="sdoing">{rowLine(r)}</span>
      </span>
    </button>
  </li>
{/snippet}

<!-- The keys 1, 2… of the list, wherever the focus is but a field. -->
<svelte:window onkeydown={listKey} onfocus={() => void app.readCodeHost(project.id)} />

<aside class="side">
  <!-- Agents · Kanban · PR · Docs (direction A of design/kanban-envs-2-2): words alone, each tab its own width. -->
  <div class="seg" role="group" aria-label={t('nav.sidebar.viewLabel')}>
    <button
      class:on={!app.boardOn && !app.docsOn && !app.prsOn}
      aria-pressed={!app.boardOn && !app.docsOn && !app.prsOn}
      onclick={() => {
        app.closeBoard(project.id);
        app.closeDocs(project.id);
        app.closePrs(project.id);
      }}>{t('common.agents')}<span class="count" aria-hidden="true">{app.projectAgents.length}</span></button
    >
    <button class:on={app.boardOn} aria-pressed={app.boardOn} onclick={() => app.openBoard(project.id)}
      >Kanban{#if review}<span class="badge" use:tip={t('nav.sidebar.toReview', { count: review })}>{review}</span>{/if}</button
    >
    {#if prsTab}
      <!-- The project's open pull requests (ESC-65), in the host's word: « PR », « MR » on GitLab; only with a code host. -->
      <button
        class:on={app.prsOn}
        aria-pressed={app.prsOn}
        onclick={() => app.openPrs(project.id)}
        aria-label={prsNew ? t('prs.list.tabNew', { word: prsWord, count: prsNew }) : undefined}
        use:tip={t('prs.list.tabHint', { word: prsWord })}
        >{t('prs.list.tab', { word: prsWord })}{#if prsNew}<span class="badge" data-testid="prs-new" aria-hidden="true">{prsNew}</span
          >{/if}</button
      >
    {/if}
    <button class:on={app.docsOn} aria-pressed={app.docsOn} onclick={() => app.openDocs(project.id)}>{t('docs.tab')}</button>
  </div>

  {#if app.docsOn}
    <DocsSidebar {project} />
  {:else}
    <!-- « Nouvel agent »: Ctrl+N on the project's harness; the arrow opens the menu « Harness » (Ctrl+Shift+N). -->
    <div class="new-split">
      <button
        class="new-agent"
        aria-label={t('nav.sidebar.newAgent')}
        aria-keyshortcuts="Control+N"
        onclick={() => app.newAgent(project.id)}
        use:tip={t('nav.sidebar.newAgentTitle', { shortcut: keyLabel('Ctrl+N') })}
      >
        <span class="plus"><Icon name="plus" size={14} /></span><span class="nl">{t('nav.sidebar.newAgent')}</span><kbd>{kbd('Ctrl+N')}</kbd
        >
      </button>
      <button
        class="new-more"
        bind:this={moreBtn}
        aria-haspopup="menu"
        aria-label={t('branches.agent.moreWays')}
        use:tip={t('branches.agent.moreWays')}
        onclick={newAgentMenu}><Icon name="down" size={14} /></button
      >
    </div>

    {#snippet card(a: Agent, principal = false)}
      {@const sel = paneKeys ? app.panes?.active === `agent:${a.id}` : a.id === selectedAgentId && !otherView}
      {@const s = spent(a)}
      {@const ticket = app.ticketOf(a.id)}
      {@const plan = planView(a)?.mini}
      {@const wfStep = cardStep(workflows.scans[project.id]?.workflows ?? [], workflows.tracks[project.id]?.[a.id])}
      {@const wfNext = cardNext(workflows.scans[project.id]?.workflows ?? [], workflows.tracks[project.id]?.[a.id])}
      {@const inBg = can(a, 'backgroundTasks') ? bgCount(a.plan) : 0}
      {@const subsRunning = (a.plan?.agents ?? []).filter((x) => x.status === 'running').length}
      {@const remote = remoteStateOf(a)}
      {@const pending = pendingFor(a, app.agents, app.tickets, app.now, can(a, 'backgroundTasks'))}
      {@const ask = listRequest(a)}
      {@const act = cardActivity(a, git?.agents[a.id] ?? 0, pending)}
      {@const prs = cardPrs(a)}
      {@const subs = a.id === selectedAgentId ? sidebarSubs(a) : null}
      {@const bg =
        a.id === selectedAgentId && can(a, 'backgroundTasks')
          ? sidebarBg(a.plan, { running: bgRunningAll[a.id], finished: bgFinishedAll[a.id] })
          : null}
      {@const driven = drivenAgents(a.plan, app.agents, app.tickets, a.id)}
      <!-- The card and the rows under it (pull requests, sub-agents…) are one row: a drop on any of them is a drop on the card. -->
      <div class="row" data-row={a.id}>
        <div
          class="agent card"
          class:waiting={a.status === 'waiting'}
          class:done={a.status === 'done' && !pending}
          class:idle={a.status === 'idle' && !pending}
          class:error={a.status === 'error'}
          class:awaits={!!pending}
          class:sel
          class:inpane={inPane(`agent:${a.id}`)}
          class:alert={app.attention[a.id]}
          class:flash={flashing[a.id]}
          class:dragging={dragging === a.id}
          style:--alert={a.status === 'error' ? 'var(--del)' : a.status === 'waiting' ? 'var(--wait)' : 'var(--run)'}
          role="button"
          tabindex="0"
          aria-current={sel ? 'true' : undefined}
          data-agent={a.id}
          onclick={(e) => open(e, { kind: 'agent', id: a.id }, () => app.selectAgent(a.id))}
          onpointerdown={(e) => {
            if (renaming !== a.id)
              paneDrag.press(e, { kind: 'agent', id: a.id }, a.name, engineName(a.engine), { id: a.id, drop: () => dropHere(a.id) });
          }}
          onkeydown={(e) => cardKey(e, a)}
          oncontextmenu={(e) => agentMenu(e, a)}
        >
          <div class="a-top">
            <StatusDot status={a.status} awaits={!!pending} />
            {#if principal}
              <span
                class="star"
                role="img"
                aria-label={t('nav.sidebar.folders.principal')}
                use:tip={{ text: t('nav.sidebar.folders.principal'), aria: false }}><Icon name="star" size={12} /></span
              >
            {/if}
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
            {#if subsRunning + inBg > 0}
              <!-- Its subagents and its background tasks at work (choice 10): their lists unfold under the card of the agent open. -->
              <span class="bg-chip">
                {#if subsRunning}
                  {@const label = t('subagents.chipLabel', { count: subsRunning })}
                  <span
                    class="part"
                    role="img"
                    aria-label={label}
                    use:tip={{ text: label, aria: false }}
                    data-sub-chip
                    data-count={subsRunning}><Icon name="sub" size={12} />{subsRunning}</span
                  >
                {/if}
                {#if inBg}
                  {@const label = t('bgtasks.chipLabel', { count: inBg })}
                  <span class="part" role="img" aria-label={label} use:tip={{ text: label, aria: false }} data-bg-chip data-count={inBg}
                    ><Icon name="clock" size={12} />{inBg}</span
                  >
                {/if}
              </span>
            {/if}
            {#if remote}
              <span class="rc" class:on={remote === 'connected'} class:warn={remote === 'stillOn'} use:tip={remoteTitle(a, remote)}>
                <Icon name="remote" size={13} />{#if remote === 'stillOn'}<span class="bar" aria-hidden="true"></span>{/if}
              </span>
            {/if}
            {#if a.status === 'waiting'}
              <span class="a-state status">{statusLabel('waiting')}</span>
            {:else if a.resumeAt}
              <span class="a-state status wk" use:tip={t('nav.sidebar.resumeTitle', { when: fWhen(a.resumeAt, app.now) })}
                ><Icon name="clock" size={12} />{t('nav.sidebar.resumes', { when: fWhen(a.resumeAt, app.now) })}</span
              >
            {:else if a.setup}
              <span class="a-state status wk" use:tip={t('nav.sidebar.settingUpTitle', { step: a.setup })}
                >{t('nav.sidebar.settingUp')}</span
              >
            {:else if pending}
              <!-- Its turn is over, but what it left goes on (ESC-138): not « Terminé » yet. -->
              <span class="a-state status wk" use:tip={pendingText(pending, app.now)} data-pending
                >{#if pending.wake}<Icon name="clock" size={12} />{/if}{pendingLabel(pending, app.now)}</span
              >
            {:else}
              <span class="a-state status">{statusLabel(a.status)}</span>
            {/if}
          </div>
          {#if a.progressLine}
            <!-- What the agent reported through Escouade's MCP server, until its next report. -->
            <div class="said" use:tip={{ text: a.progressLine, cut: true }}>
              <span class="sr">{t('mcp.progress.label')}{' '}</span>{a.progressLine}
            </div>
          {/if}
          <div class="a-meta">
            <EngineMark engine={a.engine} />
            {#if ticket?.column === 'doing' || ticket?.column === 'review'}
              <span class="ticket-chip">{ticket.key}</span>
              {#if ticket.column === 'doing'}
                <!-- The ticket's loop keeps its place; the step of its workflow, if it follows one, is in the tooltip (choice 145). -->
                <span
                  class="loop"
                  use:tip={wfStep
                    ? [t('workflow.cardStep', { step: wfStep }), wfNext].filter(Boolean).join('\n')
                    : t('workflow.next', { what: t('board.columns.review') })}
                  ><span>{t('nav.sidebar.loop', { iteration: ticket.iteration, max: ticket.maxLoops })}</span>
                  <span class="loops" aria-hidden="true"
                    >{#each { length: Math.min(ticket.maxLoops, 10) } as _, i (i)}<i class:on={i < ticket.iteration}></i>{/each}</span
                  ></span
                >
              {:else}
                <span>{t('nav.sidebar.toTest')}</span>
              {/if}
            {:else}
              <span class="model">{modelLabel(a.model, app.models)}</span>
              {#if a.prompts || a.activeMs || a.activeSince}
                <span class="sep">·</span><span class="num cut">{duration(a)}</span>
              {:else}
                <span class="sep">·</span><span class="cut">{t('nav.sidebar.noMessage')}</span>
              {/if}
              {#if wfStep}
                <!-- Where it is in its workflow takes the place of its plan, which stays in its banner (choice 145). -->
                <span class="sep">·</span><span class="wf-mini" use:tip={wfNext}><Icon name="flow" size={14} /><span>{wfStep}</span></span>
              {:else if plan}
                <!-- How far the agent is through its plan: the bar says it at a glance, the words are read. -->
                <span class="sep">·</span><span class="plan-mini"
                  ><span class="plan-text"
                    >{plan.subs
                      ? t('plan.miniSubs', { done: plan.done, total: plan.total, count: plan.subs })
                      : t('plan.mini', { done: plan.done, total: plan.total })}</span
                  ><span class="plan-bar" aria-hidden="true"><i style:--pct={plan.percent}></i></span></span
                >
              {:else if a.worktree}
                <!-- Its own branch, when the line has room for it (no plan): whole in the tooltip. -->
                <span class="sep">·</span><span class="branch" use:tip={a.worktree.branch}
                  ><Icon name="branch" size={12} /><span class="bn"
                    >{shortBranch(a.worktree.branch, a.worktree.existing ? '' : project.agentBranchPrefix)}</span
                  ></span
                >
              {/if}
            {/if}
            {#if a.worktreePending}<span class="sep">·</span><span class="wt" use:tip={t('nav.sidebar.worktreeLaterTitle')}
                >{t('nav.sidebar.worktreeLater')}</span
              >{/if}
            <!-- The temperature of its cache (ESC-398), at the end of the line of its model. -->
            <HeatFigure agent={a} />
          </div>
          {#if a.status === 'waiting'}
            <!-- What it waits on, answered from here (2.0, new): its choices clicked, or its keys 1, 2… (listKey). Questions one
               after the other; a permission allowed, for good or for the session, or refused. What does not hold on the card
               is answered in its conversation. -->
            {@const req = a.requests?.[0]}
            <div class="a-ask">
              {#if ask?.kind === 'question'}
                {@const step = shownStep(ask)}
                {#if ask.note}<p class="note">{revealHidden(ask.note)}</p>{/if}
                {#if ask.questions.length > 1}
                  <span class="step">{t('nav.sidebar.step', { n: step + 1, count: ask.questions.length })}</span>
                {/if}
                <p>{ask.questions[step].question}</p>
                {@render choices(a, ask)}
              {:else if ask}
                <p class="perm"><span class="tool">{toolLabel(ask.tool)}</span> <span class="arg mono">{revealHidden(ask.arg)}</span></p>
                {#if ask.note}<p class="note">{revealHidden(ask.note)}</p>{/if}
                {@render choices(a, ask)}
              {:else if req}
                <p class="clip">
                  {req.kind === 'question'
                    ? req.questions.map((x) => x.question).join(' · ')
                    : req.tool === 'ExitPlanMode'
                      ? t('stats.overview.proposesPlanThen', { engine: speakerOf(a.engine), mode: modeLabel(planResume(a)) })
                      : `${req.tool}${req.arg ? ` ${req.arg.split('\n')[0]}` : ''}`}
                </p>
                <button
                  class="in-conv"
                  onclick={(e) => {
                    e.stopPropagation();
                    app.selectAgent(a.id);
                  }}>{t('nav.sidebar.inConversation')}<Icon name="right" size={12} /></button
                >
              {/if}
            </div>
          {:else if act || s.tokens > 0 || s.cost > 0}
            <div class="a-act" use:tip={s.estimated ? estimateHint(s) : undefined}>
              {#if act?.kind === 'doing'}
                <b>{act.verb}</b>{#if act.what}<span class="tgt mono">{act.what}</span>{/if}
              {:else if act?.kind === 'thinking'}
                <span class="tgt">{t('stats.overview.thinking')}</span>
              {:else if act?.kind === 'over'}
                <span class="ok"><Icon name="check" size={14} /></span><span class="tgt">{t('nav.sidebar.over')} · {act.goes}</span>
              {:else if act?.kind === 'result'}
                <span class="ok"><Icon name="check" size={14} /></span><span class="tgt"
                  >{t('nav.sidebar.changed', { count: act.files })}</span
                >
              {/if}
              <span class="spend">{cardSpend(a)}</span>
            </div>
          {/if}
        </div>
        {#if a.prDown && !a.ticketId}
          <!-- Its code host's plugin does not answer (INT-8.1): said under the card, the pull requests read last kept. -->
          <div class="prdown" role="status" data-pr-down={a.id} use:tip={t('prs.downHint', { why: a.prDown, word: prWord(a) })}>
            <Icon name="plug" size={12} />{t('prs.down', { word: prWord(a) })}
          </div>
        {/if}
        {#if !prs.length && !a.ticketId && ciShown(a.branchCi)}
          <!-- Its branch pushed without a pull request (ESC-63): its CI, its jobs on a click. -->
          <div class="prs" role="group" aria-label={t('prs.group', { name: a.name })} data-branch-ci={a.id}>
            <CiPill ci={a.branchCi} of={{ kind: 'agent', id: a.id }} projectId={a.projectId} />
          </div>
        {/if}
        {#if prs.length}
          <!-- Under the card, not in it (the card is a button): its pull requests (ESC-64), the ones to merge first; a number
             opens it, the corrections go to the agent on a click only. -->
          {@const word = prWord(a)}
          <div class="prs" role="group" aria-label={t('prs.group', { name: a.name })} data-prs={a.id}>
            {#each prs as p (prKey(p))}
              <div class="pr" data-pr={p.number}>
                <button
                  class="prnum {prTone(p)}"
                  aria-label={t('prs.chipLabel', { word, number: p.number, state: stateWord(p) })}
                  use:tip={{ text: prTip(word, p), aria: false }}
                  onclick={() => openPr(p, word)}
                  ><Icon name={prIcon(p)} size={12} />{p.repo ? `${repoLabel(p.repo)} ` : ''}#{p.number}</button
                >
                <!-- Where it goes and its CI in words (ESC-305): « #41 → main · CI en échec ». -->
                <span class="prto">{t('prs.to', { base: p.base })}</span>
                {#if p.checks && p.state === 'open'}
                  <!-- Its CI opens the side panel's « PR et CI » on it, even where the panel is too narrow for that tab (choice 189). -->
                  <button class="prci {p.checks}" use:tip={t('prs.seeCi', { word, number: p.number })} onclick={() => showCi(a.id)}
                    >{t(`prs.checks.${p.checks}`)}</button
                  >
                {/if}
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
        {#if subs}
          <!-- Under the card of the agent open only (choice 10 of the 2.0 design): its subagents, unfolded while one runs. -->
          {@const open = app.subsOpen[a.id] ?? subs.running.length > 0}
          {@const doneOpen = finishedOpen[a.id] ?? false}
          <div class="subl" class:sel role="group" aria-label={t('subagents.toggle', { name: a.name })}>
            <button class="sh subs-toggle" aria-expanded={open} onclick={() => (app.subsOpen[a.id] = !open)}>
              {@render chev(open)}
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
                      {@render chev(doneOpen)}{t('subagents.finishedToggle', { count: subs.finished.length })}
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
        {#if bg}
          <!-- Under the card of the agent open too, after the subagents: what its harness runs in the background. -->
          {@const open = app.bgOpen[a.id] ?? bg.runningTotal > 0}
          {@const doneOpen = bgFinishedOpen[a.id] ?? false}
          <div class="subl bgs" class:sel role="group" aria-label={t('bgtasks.toggle', { name: a.name })} data-bg-group={a.id}>
            <button class="sh subs-toggle" aria-expanded={open} onclick={() => (app.bgOpen[a.id] = !open)}>
              {@render chev(open)}
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
                      {@render chev(doneOpen)}{t('bgtasks.finishedToggle', { count: bg.finishedTotal })}
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
        {#if driven.length}
          <!-- Under the card too (ESC-138): the agents of Escouade it set to work through the MCP server, while they work. -->
          {@const open = drivenOpen[a.id] ?? true}
          <div class="subl" class:sel role="group" aria-label={t('bgtasks.later.drivenToggle', { name: a.name })} data-driven-group={a.id}>
            <button class="sh subs-toggle" aria-expanded={open} onclick={() => (drivenOpen[a.id] = !open)}>
              {@render chev(open)}
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
      </div>
    {/snippet}

    <div class="agents" data-sidebar-drop role="presentation">
      {#each items as it (it.kind === 'agent' ? it.agent.id : it.folder.id)}
        {@render line(null, it.kind === 'agent' ? it.agent.id : it.folder.id)}
        {#if it.kind === 'agent'}
          {@render card(it.agent)}
        {:else}
          {@const f = it.folder}
          {@const cards = f.folded ? foldedCards(it, selectedAgentId) : [...(it.principal ? [it.principal] : []), ...it.agents]}
          <section class="folder" class:over={overFolder === f.id} class:dragging={dragging === f.id} data-folder={f.id}>
            <!-- A click on the header outside its name and its field folds it, as the chevron does (which has the keys). -->
            <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
            <div
              class="folder-h"
              role="group"
              aria-label={f.name}
              onpointerdown={(e) => {
                if (renaming !== f.id) paneDrag.press(e, null, f.name, '', { id: f.id, drop: () => dropHere(f.id) });
              }}
              oncontextmenu={(e) => folderMenu(e, f)}
              onclick={(e) => {
                if (!(e.target as HTMLElement).closest('.name, .rename, .chev')) app.run(api.agentFolderFold(project.id, f.id, !f.folded));
              }}
            >
              <button
                class="chev fold"
                aria-expanded={!f.folded}
                aria-label={t(f.folded ? 'nav.sidebar.folders.expand' : 'nav.sidebar.folders.collapse', { name: f.name })}
                onclick={() => app.run(api.agentFolderFold(project.id, f.id, !f.folded))}
                onkeydown={(e) => stepKey(e, f.id, `[data-folder="${CSS.escape(f.id)}"] .chev`)}
              >
                <Icon name={f.folded ? 'right' : 'down'} size={12} />
              </button>
              {#if renaming === f.id}
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  class="rename"
                  aria-label={t('nav.sidebar.folders.rename')}
                  bind:value={renameValue}
                  autofocus
                  onkeydown={(e) => {
                    e.stopPropagation();
                    if (e.key === 'Enter') commitFolderRename(f);
                    if (e.key === 'Escape') renaming = null;
                  }}
                  onblur={() => commitFolderRename(f)}
                />
              {:else}
                <span class="name" ondblclick={() => startFolderRename(f)} role="presentation">{f.name}</span>
              {/if}
              <span class="cnt" aria-hidden="true">{(it.principal ? 1 : 0) + it.agents.length}</span>
              {#if it.running}<span class="pill run" aria-hidden="true">{it.running}</span>{/if}
              {#if it.waiting}<span class="pill wait" aria-hidden="true">{it.waiting}</span>{/if}
              <span class="sr">{folderCounts(it)}</span>
            </div>
            <div class="folder-b" role="presentation">
              {#each cards as a (a.id)}
                {@render line(f.id, a.id)}
                {@render card(a, a.id === it.principal?.id)}
              {/each}
              {@render line(f.id, null)}
              {#if !it.principal && !it.agents.length}
                <p class="folder-empty">{t('nav.sidebar.folders.empty')}</p>
              {/if}
            </div>
          </section>
        {/if}
      {/each}
      {@render line(null, null)}
      {#if !shownAgents(items).length}
        <!-- No agent at all (empty folders may remain): the invitation stays. -->
        <div class="empty">
          <p>{t('nav.sidebar.empty')}</p>
          <button class="btn" onclick={() => app.newAgent(project.id)}><Icon name="plus" size={14} />{t('nav.sidebar.createAgent')}</button>
        </div>
      {/if}

      {#if app.archivedAgents.length}
        <button class="archived-h" aria-expanded={app.showArchived} onclick={() => (app.showArchived = !app.showArchived)}>
          {@render chev(app.showArchived)}{t('nav.sidebar.archived', { n: app.archivedAgents.length })}
        </button>
        {#if app.showArchived}
          {#each app.archivedAgents as a (a.id)}
            <div
              class="agent card done archived"
              class:sel={a.id === selectedAgentId && !otherView}
              role="button"
              tabindex="0"
              aria-current={a.id === selectedAgentId && !otherView ? 'true' : undefined}
              onclick={() => app.selectAgent(a.id)}
              onkeydown={(e) => e.key === 'Enter' && app.selectAgent(a.id)}
              oncontextmenu={(e) => agentMenu(e, a)}
            >
              <div class="a-top">
                <StatusDot status="done" />
                <span class="name">{a.name}</span>
                <span class="a-state status">{fUsd(a.cost)}</span>
              </div>
              {#if a.worktreeKept}
                <!-- Its ticket over, its worktree stayed: what it holds would be lost (ESC-143). -->
                <div class="kept"><Icon name="branch" size={12} />{a.worktreeKept}</div>
              {/if}
            </div>
          {/each}
        {/if}
      {/if}
    </div>

    <RunsSection {project} />

    <section class="sec terms" class:folded={termsFolded} aria-label={t('nav.sidebar.terminals')}>
      <div class="sec-h">
        <FoldTitle
          title={t('nav.sidebar.terminals')}
          short={shortWindow.now}
          folded={termsFolded}
          ontoggle={() => (termsUnfolded = !termsUnfolded)}
        />
        <span class="n">{terms.length}</span>
        <span class="acts">
          <button
            class="icon-btn"
            bind:this={termMenuBtn}
            aria-label={t('nav.sidebar.newTerminal')}
            use:tip={{ text: t('nav.sidebar.newTerminal'), aria: false }}
            onclick={shellMenu}><Icon name="plus" size={14} /></button
          >
        </span>
      </div>
      <div class="rows">
        {#each terms as term (term.id)}
          <div
            class="row-item term"
            class:sel={paneKeys ? app.panes?.active === `term:${term.id}` : termSelected === term.id}
            class:inpane={inPane(`term:${term.id}`)}
            role="button"
            tabindex="0"
            aria-current={termSelected === term.id ? 'true' : undefined}
            data-shell={term.shell}
            onclick={(e) => open(e, { kind: 'term', id: term.id }, () => app.selectTerm(term.id))}
            onpointerdown={(e) => paneDrag.press(e, { kind: 'term', id: term.id }, term.name)}
            oncontextmenu={(e) => menu.show(e, besideItems({ kind: 'term', id: term.id }))}
            onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && app.selectTerm(term.id)}
          >
            <span class="ti"><Icon name="term" size={14} /></span>
            <span class="nm">{term.name}</span>
            {#if app.exitedTerms[term.id] !== undefined}<span class="st">{t('nav.sidebar.exited')}</span>{/if}
            <span class="ra">
              <button
                class="icon-btn x"
                aria-label={t('common.close')}
                use:tip={{ text: t('common.close'), aria: false }}
                onclick={(e) => {
                  e.stopPropagation();
                  closeTerminal(term.id);
                }}><Icon name="close" size={12} /></button
              >
            </span>
          </div>
        {:else}
          <span class="none">{t('nav.sidebar.noTerminals')}</span>
        {/each}
      </div>
    </section>
  {/if}

  <div class="side-foot">
    <button
      class="foot-row"
      aria-label={t('nav.sidebar.browseFolder', { path: tildify(project.path) })}
      use:tip={{ text: project.path, aria: false }}
      onclick={() => app.openEditor({ projectId: project.id, source: 'project' })}
    >
      <Icon name="folder" size={14} /><span class="grow mono path">{tildify(project.path)}</span><span class="browse" aria-hidden="true"
        >{t('common.browse')}</span
      >
    </button>
    <div class="foot-row">
      <Icon name="branch" size={14} />
      {#if git?.isRepo}
        {#if git.repos?.length}
          <!-- Of several repositories (ESC-169): their branch when they share it, and how many they are. -->
          {@const s = branchSummary(git)}
          <span class="grow mono branch">{s.branch ?? ''}<span class="repos">{s.branch ? ' · ' : ''}{s.text}</span></span>
        {:else}
          <span class="grow mono branch">{git.branch || '—'}</span>
        {/if}
        <span
          class="changes mono"
          use:tip={git.repos?.length ? deltaDetail(git) : t('nav.sidebar.changesTitle', { m: git.modified, a: git.added, d: git.deleted })}
        >
          <span class="mod" aria-label={t('nav.sidebar.modified', { n: git.modified })}>~{git.modified}</span>
          <span class="add" aria-label={t('nav.sidebar.added', { n: git.added })}>+{git.added}</span>
          <span class="del" aria-label={t('nav.sidebar.deleted', { n: git.deleted })}>−{git.deleted}</span>
        </span>
      {:else}
        <!-- No git information yet, or no repository. -->
        <span class="grow norepo">{git ? t('nav.sidebar.noRepo') : ''}</span>
      {/if}
    </div>
  </div>
  <!-- Its right edge sets its width (ESC-204), as the rail's does. -->
  <div class="edge">
    <Splitter
      value={sideWidth.shown}
      min={SIDE_MIN}
      max={sideWidth.max}
      onreset={() => sideWidth.reset()}
      label={t('nav.sidebar.width')}
      title={t('nav.sidebar.widthTip')}
      onresize={(w) => sideWidth.set(w)}
      oncommit={(w) => sideWidth.commit(w)}
    />
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
  /* The buttons of the sidebar draw only what their class says (the preview's reset). */
  .new-agent,
  .new-more,
  .sh,
  .done-toggle,
  .more-toggle,
  .sub,
  .archived-h,
  .opt,
  .prnum,
  .prfix,
  .foot-row {
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
  }

  /* On the frame, beside the panel of the work (design/app-redesign/index.html, .sidebar). */
  .side {
    /* Its width chosen by its edge (ESC-204): App sets --side. */
    position: relative;
    width: var(--side);
    flex: none;
    container: sidebar / inline-size;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 6px 10px 10px;
  }
  /* The rail sits beside the sidebar in a narrow window: its left margin shrinks, its width stays (ESC-158). */
  @media (max-width: 1240px) {
    .side {
      padding-left: 6px;
    }
  }

  /* The right edge, which sets the width: the rail's, on the line between the sidebar and the work, seen once hovered. */
  .edge {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    display: flex;
  }
  .edge :global(.splitter) {
    background: transparent;
  }
  /* Its grip inside the sidebar: over the work, it would cover the panel's edge. */
  .edge :global(.splitter::before) {
    inset: 0 0 0 -6px;
  }

  /*
   * Agents | Kanban | PR | Docs (direction A, ESC-390): words alone, each tab its own width and the room left shared,
   * the tab shown raised as a key of the panel; three tabs without a code host fill the bar the same way.
   */
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .seg button {
    flex: 1 1 auto;
    min-width: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    height: 28px;
    padding: 0 6px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    font-weight: 550;
    white-space: nowrap;
    cursor: pointer;
    transition:
      color 0.15s,
      background-color 0.15s;
  }
  .seg button:hover {
    color: var(--text);
  }
  .seg button.on {
    background: var(--panel);
    color: var(--text);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 1px 2px rgba(0, 0, 0, calc(0.18 * var(--shade)));
  }
  .seg button:focus-visible {
    outline: 2px solid var(--ring);
    outline-offset: -2px;
  }
  .seg .count {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .seg .badge {
    min-width: 16px;
    height: 16px;
    font-size: 10px;
  }
  .badge {
    min-width: 17px;
    height: 17px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--wait);
    color: var(--on-wait);
    font-size: 10.5px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  /* « Nouvel agent » and its menu « Harness ». */
  .new-split {
    display: flex;
    align-items: center;
    gap: 2px;
    margin: 12px 0 8px;
  }
  .new-agent {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 32px;
    padding: 0 8px 0 9px;
    border-radius: 8px;
    color: var(--soft);
    font-weight: 550;
    transition: background-color 0.15s;
  }
  .new-agent:hover,
  .new-more:hover {
    background: var(--hover);
    color: var(--text);
  }
  .new-agent .plus {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .new-agent .nl {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Narrow (ESC-204), « Nouvel agent » stays whole: its shortcut goes, still in its tooltip; on a card, the bars of a
     plan and of a ticket's loops go, their words stay. */
  @container sidebar (max-width: 250px) {
    .new-agent kbd,
    .plan-bar,
    .loops {
      display: none;
    }
  }
  .new-more {
    display: grid;
    place-items: center;
    flex: none;
    width: 26px;
    height: 32px;
    border-radius: 8px;
    color: var(--muted);
  }
  .new-agent:focus-visible,
  .new-more:focus-visible,
  .agent:focus-visible,
  .row-item:focus-visible,
  .foot-row:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }

  /* The agents: cards without a frame, the one open in relief, the one asking in amber. */
  .agents {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-height: 80px;
    overflow: auto;
    margin: 0 -4px;
    padding: 0 4px 12px;
    mask-image: linear-gradient(to bottom, black calc(100% - 28px), transparent);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .agent {
    --tone: var(--run);
    flex: none;
    display: grid;
    /* One column held to the sidebar: a long choice or name never widens the card past it (ESC-249). */
    grid-template-columns: minmax(0, 1fr);
    gap: 3px;
    padding: 8px 10px 9px;
    border-radius: 9px;
    text-align: left;
    cursor: pointer;
    outline: none;
    transition:
      background-color 0.2s,
      box-shadow 0.2s;
  }
  .agent:hover {
    background: var(--hover);
  }
  .agent.sel {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  /* Shown in a pane of the split screen that is not the active one (ESC-392). */
  .agent.inpane,
  .row-item.inpane {
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .agent.waiting {
    --tone: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .agent.done,
  .agent.idle {
    --tone: var(--muted);
  }
  .agent.error {
    --tone: var(--del);
  }
  .agent.awaits {
    --tone: var(--soft);
  }
  /* Asked, finished or failed out of sight: its colour breathes until seen. */
  .agent.alert {
    box-shadow: inset 0 0 0 1px var(--alert);
    animation: card-alert 1.1s var(--ease) infinite alternate;
  }
  @keyframes card-alert {
    to {
      background: color-mix(in oklch, var(--alert) 14%, transparent);
    }
  }
  .a-top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-weight: 590;
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rename {
    flex: 1;
    min-width: 0;
    height: 22px;
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--p-ink);
    color: var(--text);
    font: inherit;
    font-weight: 590;
    outline: none;
  }
  .a-state {
    flex: none;
    color: var(--tone);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .agent.done .a-state,
  .agent.idle .a-state {
    font-weight: 500;
  }
  .a-state.wk {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .agent:not(.awaits) .a-state.wk {
    color: var(--wait);
  }
  .bg-chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .bg-chip {
    gap: 7px;
  }
  .bg-chip .part {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }
  .rc {
    position: relative;
    flex: none;
    display: inline-flex;
    color: var(--dim);
  }
  .rc.on {
    color: var(--teal-ink-2);
  }
  /* Off for Escouade, still linked in the interface left open: barred, in the colour of the other warnings. */
  .rc.warn {
    color: var(--wait);
  }
  .rc .bar {
    position: absolute;
    left: 50%;
    top: -1px;
    width: 1.5px;
    height: 15px;
    background: currentColor;
    transform: rotate(-45deg);
  }
  .a-meta,
  .a-act,
  .said {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding-left: 16px;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .a-meta {
    overflow: hidden;
  }
  .a-meta .sep {
    color: var(--sep);
  }
  .a-meta .branch {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .a-meta .branch :global(.icon) {
    flex: none;
  }
  .a-meta .model,
  .a-meta .bn,
  .a-meta .wt,
  .a-meta .cut {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Narrow, the branch is cut before the model, which keeps its name (ESC-249). */
  .a-meta .model {
    flex: none;
    max-width: 45%;
  }
  .said {
    display: block;
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .a-act {
    color: var(--soft);
  }
  .a-act b {
    flex: none;
    font-weight: 550;
  }
  .a-act .tgt {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    text-overflow: ellipsis;
  }
  /* As the preview's .mono: a size under the line's, set closer. */
  .a-act .mono {
    font-size: 0.92em;
    letter-spacing: -0.01em;
  }
  .a-act .ok {
    display: inline-flex;
    color: var(--add);
  }
  .a-act .spend {
    flex: none;
    margin-left: auto;
    padding-left: 8px;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .ticket-chip {
    flex: none;
    color: var(--p-ink);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .loops {
    display: inline-flex;
    gap: 2px;
  }
  .loops i {
    width: 8px;
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.12);
  }
  .loops i.on {
    background: var(--p-ink);
  }
  /* Where the card meets the workflow (ESC-197): the preview's `.wf-mini`. */
  .loop {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .wf-mini {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    white-space: nowrap;
  }
  .wf-mini :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .wf-mini span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .plan-mini {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .plan-text {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .plan-bar {
    flex: none;
    width: 46px;
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.1);
    overflow: hidden;
  }
  .plan-bar i {
    display: block;
    height: 100%;
    background: var(--soft);
    transform: scaleX(calc(var(--pct) / 100));
    transform-origin: left;
    transition: transform 1s var(--ease);
  }

  /* The question, answered from the list. */
  .a-ask {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 8px;
    padding: 4px 0 2px 16px;
  }
  .a-ask p {
    margin: 0;
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.4;
    overflow-wrap: anywhere;
  }
  .a-ask .step,
  .a-ask .note {
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .a-ask p.note {
    white-space: pre-line;
  }
  .a-ask .perm {
    display: grid;
    gap: 2px;
  }
  .a-ask .tool {
    color: var(--muted);
    font-size: 11.5px;
  }
  .a-ask .arg {
    font-size: 11.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .in-conv {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    gap: 4px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--wait);
    font-size: 12px;
    font-weight: 550;
    text-align: left;
  }
  .in-conv:hover {
    text-decoration: underline;
  }
  .agent.flash {
    animation: flash 0.9s var(--ease);
  }
  @keyframes flash {
    from {
      background: oklch(from var(--run) l c h / 0.16);
    }
  }
  .a-ask p.clip {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* A long choice on more lines, as in the conversation's card, its number on its first one (ESC-249). */
  .opt {
    display: inline-flex;
    align-items: flex-start;
    gap: 7px;
    max-width: 100%;
    min-height: 26px;
    padding: 4px 9px 4px 6px;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12px;
    font-weight: 550;
    line-height: 18px;
    text-align: left;
    overflow-wrap: anywhere;
    transition:
      background-color 0.15s,
      box-shadow 0.15s;
  }
  .opt:hover:not(:disabled) {
    background: oklch(from var(--wait) l c h / 0.14);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .opts.stack {
    flex-direction: column;
    align-items: stretch;
  }
  .opt.primary {
    background: oklch(from var(--wait) l c h / 0.14);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .opt:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .opt kbd {
    flex: none;
    margin-top: 1px;
    height: 16px;
    min-width: 16px;
    justify-content: center;
    padding: 0 3px;
    font-size: 10.5px;
  }

  /* The pull requests under a card (ESC-64): a number with its state, the one correction that helps. */
  .prs {
    display: grid;
    gap: 4px;
    margin: -2px 6px 6px 30px;
  }
  .pr {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .prnum,
  .prfix {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 5px;
    color: var(--soft);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .prnum {
    gap: 5px;
    padding: 0 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-family: var(--mono);
  }
  .prnum.ok :global(.icon) {
    color: var(--run);
  }
  .prnum.fail :global(.icon) {
    color: var(--del);
  }
  .prnum.wait :global(.icon) {
    color: var(--wait);
  }
  .prnum.merged :global(.icon) {
    color: var(--syn-k);
  }
  .prnum.dim {
    color: var(--muted);
  }
  .prdown {
    display: flex;
    gap: 4px;
    align-items: center;
    margin: -2px 6px 4px 30px;
    color: var(--warn-ink);
    font-size: 11px;
  }
  .prto {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font: 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .prci {
    padding: 0;
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .prci:hover {
    text-decoration: underline;
  }
  .prci.success {
    color: var(--run);
  }
  .prci.failure {
    color: var(--del);
  }
  .prci.pending {
    color: var(--wait);
  }
  .prfix {
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--text);
    font-weight: 550;
  }
  .prnum:hover,
  .prfix:hover {
    background: var(--hover);
    color: var(--text);
  }

  /* Subagents, background tasks and driven agents: a thread under their agent's card. A list shows its first rows and a
     button unrolls the others: it never scrolls inside the list of the agents, which scrolls already. */
  .subl {
    display: grid;
    /* One column no wider than the sidebar: a long command of a background task is cut short, never pushes out. */
    grid-template-columns: minmax(0, 1fr);
    gap: 1px;
    margin: -1px 0 4px 18px;
    padding-left: 8px;
    border-left: 1px solid var(--line-2);
  }
  .sh,
  .done-toggle,
  .more-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    border-radius: 6px;
    color: var(--muted);
    font-size: 11.5px;
    text-align: left;
  }
  .sh .lbl {
    font-weight: 600;
  }
  .sh:hover,
  .done-toggle:hover,
  .more-toggle:hover {
    background: var(--hover);
    color: var(--text);
  }
  .chev {
    display: inline-flex;
    flex: none;
    color: var(--dim);
  }
  .cnt {
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .cnt.run {
    color: var(--run);
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
    min-width: 0;
    padding: 5px 6px 5px calc(6px + var(--depth, 0) * 12px);
    border-radius: 6px;
    color: var(--soft);
    font-size: 12px;
    text-align: left;
  }
  .sub:hover:not(.on) {
    background: var(--hover);
  }
  .sub.on {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  .sub:focus-visible,
  .sh:focus-visible,
  .done-toggle:focus-visible,
  .more-toggle:focus-visible,
  .archived-h:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .sdot {
    flex: none;
    width: 6px;
    height: 6px;
    margin-top: 5px;
    border-radius: 50%;
    background: none;
    box-shadow: inset 0 0 0 1.5px var(--done);
  }
  .sdot.running {
    background: var(--run);
    box-shadow: none;
  }
  .sdot.failed {
    background: var(--del);
    box-shadow: none;
  }
  .sdot.stopped,
  .sdot.interrupted {
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .stext {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stitle.mono {
    font-size: 11.5px;
  }
  .smodel,
  .sstate,
  .sdur {
    flex: none;
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .sdoing {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* A folder of agents: a frame (never `.agent`, which the cards are), its cards under a rule. */
  .folder {
    border: 1px solid var(--line);
    border-radius: 10px;
    margin: 2px 0 4px;
    background: color-mix(in oklch, var(--p-ink) 3%, var(--panel));
  }
  .folder.over {
    box-shadow: inset 0 0 0 1px var(--p-ink);
  }
  .folder.dragging,
  .agent.dragging {
    opacity: 0.45;
  }
  .folder-h {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    font-size: 12px;
  }
  .folder-h .name {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
  }
  .folder-h .rename {
    flex: 1;
    min-width: 0;
  }
  .folder-h .cnt {
    margin-right: auto;
  }
  .folder-h .fold {
    padding: 0;
    cursor: pointer;
  }
  .folder-h .pill {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  .pill.run {
    color: var(--run);
  }
  .pill.wait {
    color: var(--wait);
  }
  .folder-b {
    margin: 0 6px 4px 14px;
    padding: 0 0 0 8px;
    border-left: 1px solid var(--line-2);
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 4px;
  }
  .folder-empty {
    margin: 0;
    padding: 6px 4px;
    color: var(--dim);
    font-size: 11.5px;
  }
  .star {
    display: inline-flex;
    color: var(--p-ink);
  }
  .star :global(svg) {
    fill: currentColor;
  }
  /* Where a dragged agent or folder lands. */
  .drop {
    pointer-events: none;
    flex: none;
    height: 3px;
    margin: -2px 0;
    border-radius: 2px;
    background: var(--p-ink);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 24px 8px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .empty p {
    margin: 0;
  }
  .archived-h {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 8px 6px 4px;
    padding: 2px 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    text-align: left;
  }
  .archived-h:hover {
    color: var(--text);
  }
  .agent.archived {
    opacity: 0.85;
  }
  .kept {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin-top: 3px;
    color: var(--warn-ink);
    font-size: 12px;
    line-height: 1.35;
  }
  .kept :global(.icon) {
    margin-top: 2px;
  }

  /* Terminaux, as Lancement (RunsSection): a section under a hairline, its rows. */
  .sec {
    display: grid;
    gap: 1px;
    min-height: 0;
    margin: 6px -2px 0;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 2px 0 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .sec-h .n {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .sec-h .acts {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  .rows {
    display: grid;
    gap: 1px;
    max-height: 30vh;
    overflow: auto;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 4px 0 9px;
    border-radius: 7px;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    outline: none;
  }
  .row-item:hover {
    background: var(--hover);
  }
  .row-item.sel {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .row-item .ti {
    display: inline-flex;
    color: var(--muted);
  }
  .row-item .nm {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-item .st {
    flex: none;
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  /* Shown on hover or focus; out of sight otherwise, but still there for the keyboard and a screen reader. */
  .row-item .ra {
    display: inline-flex;
    width: 0;
    margin-left: auto;
    overflow: hidden;
  }
  .row-item .st + .ra {
    margin-left: 2px;
  }
  .row-item:hover .ra,
  .row-item.sel .ra,
  .row-item:focus-within .ra {
    width: auto;
  }
  .row-item .ra .icon-btn {
    width: 22px;
    height: 22px;
  }
  .none {
    padding: 2px 9px 6px;
    color: var(--dim);
    font-size: 12px;
  }

  /* The project's folder and branch. */
  .branch .repos {
    color: var(--muted);
    font-family: var(--font);
  }
  .side-foot {
    display: grid;
    gap: 2px;
    margin: 8px -2px 0;
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }
  .foot-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 28px;
    padding: 0 8px;
    border-radius: 7px;
    color: var(--muted);
    font-size: 12px;
  }
  button.foot-row:hover {
    background: var(--hover);
    color: var(--text);
  }
  .foot-row .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .foot-row .branch {
    color: var(--soft);
  }
  .browse {
    display: none;
    font-size: 11.5px;
    font-weight: 550;
  }
  button.foot-row:hover .browse,
  button.foot-row:focus-visible .browse {
    display: inline;
  }
  .changes {
    display: inline-flex;
    gap: 8px;
    font-variant-numeric: tabular-nums;
  }
  .mod {
    color: var(--wait);
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }

  /* A short window (820 px and less): Terminaux folds to its header, the agents get the room (choice 9), until its title
     unfolds it. */
  .terms.folded .rows {
    display: none;
  }
  @media (max-height: 820px) {
    .sec {
      padding-bottom: 2px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .agent.alert {
      animation: none;
      background: color-mix(in oklch, var(--alert) 12%, transparent);
    }
    .plan-bar i {
      transition: none;
    }
  }
</style>

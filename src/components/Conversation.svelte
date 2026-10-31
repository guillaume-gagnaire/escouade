<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { onMount, tick, untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import { isMulti, repoLabel, touchedRepos } from '../lib/repos';
  import Rich from '../lib/i18n/Rich.svelte';
  import { api } from '../lib/ipc';
  import { withoutPassage } from '../lib/conversation';
  import { canSwitchView, inHarnessView, switchView } from '../lib/agent-actions';
  import { agentGitOf } from '../lib/agent-branch';
  import {
    askOpenPr,
    canOpenPr,
    cardPrs,
    openPr,
    openPrOf,
    ownBranch,
    prIcon,
    prKey,
    prTip,
    prTone,
    prWord,
    PRS_EVERY_MS,
    refreshPrs,
    stateWord,
  } from '../lib/prs';
  import { conversationOf, type ReadingPlace } from '../lib/conversations.svelte';
  import { heldOf } from '../lib/outbox.svelte';
  import { accountsOf } from '../lib/accounts';
  import { bgCount } from '../lib/bgtasks';
  import { splitEscouade } from '../lib/escouade';
  import { fDur, fInt, fNum, fTime, fTok } from '../lib/format';
  import { stepDividers } from '../lib/workflow-view';
  import { workflows } from '../lib/workflows.svelte';
  import { contextUse, estimateHint, fSpentUsd, spent } from '../lib/spend';
  import { capsOf, engineName, showsCost, speakerOf } from '../lib/engines';
  import { injectedSource, parseAgentMessage, subagentLabels } from '../lib/events';
  import { localCommandViews } from '../lib/localcmd';
  import { subagentInfo } from '../lib/subagents';
  import { editsByTurn } from '../lib/tools';
  import { app, type HeaderFolded, type SubFocus } from '../lib/state.svelte';
  import { canPrepare, canTest, prepareLaunch, testAgent } from '../lib/test-launch.svelte';
  import type { Agent, ConvItem, Project } from '../lib/types';
  import AgentTerminal from './AgentTerminal.svelte';
  import Composer from './Composer.svelte';
  import PlanBanner from './PlanBanner.svelte';
  import CriteriaReport from './conv/CriteriaReport.svelte';
  import EventMessage from './conv/EventMessage.svelte';
  import Markdown from './conv/Markdown.svelte';
  import Notice from './conv/Notice.svelte';
  import PermissionCard from './conv/PermissionCard.svelte';
  import QuestionCard from './conv/QuestionCard.svelte';
  import SentFiles from './conv/SentFiles.svelte';
  import SetupProgress from './conv/SetupProgress.svelte';
  import Thinking from './conv/Thinking.svelte';
  import ToolRow from './conv/ToolRow.svelte';
  import TurnCard from './conv/TurnCard.svelte';
  import Handover from './conv/Handover.svelte';
  import NoAccount from './conv/NoAccount.svelte';
  import UserMessage from './conv/UserMessage.svelte';
  import HeldMessage from './conv/HeldMessage.svelte';
  import EngineMark from './EngineMark.svelte';
  import Icon from './Icon.svelte';
  import { tip } from '../lib/tip';

  let { agent, project }: { agent: Agent; project: Project } = $props();

  /** The words that tell what an agent is doing, read when shown (the language of the interface can change). */
  function statusLabel(status: string): string {
    switch (status) {
      case 'running':
        return t('conv.status.running');
      case 'waiting':
        return t('conv.status.waiting');
      case 'idle':
        return t('conv.status.idle');
      case 'done':
        return t('conv.status.done');
      default:
        return t('conv.status.error');
    }
  }
  /** The colour of its state in the header's pill: at work, waiting, at rest, failed. */
  const PILL: Record<string, string> = { running: 'run', waiting: 'wait', idle: 'idle', done: 'idle', error: 'err' };
  /** How long a view put back in place is kept there while its messages are measured. */
  const SETTLE_MS = 1000;
  /** How long a message brought into view from a search stays highlighted. */
  const FOUND_MS = 2000;
  /** How many items the view draws at first, and adds each time the reader asks for the ones before. */
  const SLICE = 80;
  /** Items drawn above a message brought into view from the older ones, for it to sit in the middle of the view. */
  const LEAD = 10;

  const conv = $derived(conversationOf(agent.id));
  // The ticket the agent works on, whose criteria the reports of its turns name.
  const ticket = $derived(app.ticketOf(agent.id));
  const top = $derived(conv.items.filter((i) => !i.parent));
  // A long conversation is drawn from an item on (each block is a DOM node, a message may be heavy Markdown): its
  // latest SLICE items at first, more when the reader asks for them. `from` is that first item, by its id; null for the
  // latest SLICE, which slide as items come while the view follows the bottom. It is pinned as soon as the view leaves
  // the bottom, so that new items never take away what the reader is on.
  let from = $state<string | null>(null);
  const rankOf = (id: string) => top.findIndex((i) => i.id === id);
  const start = $derived.by(() => {
    const rank = from === null ? -1 : rankOf(from);
    return rank >= 0 ? rank : Math.max(0, top.length - SLICE);
  });
  const drawn = $derived(top.slice(start));
  // A divider at each entry of the agent in a step of its project's workflows (ESC-197, choice 145), before the message
  // of its door or the call that signalled it: as the project has them (corrections in, those dismissed left out).
  const dividers = $derived.by(() => {
    const track = workflows.tracks[project.id]?.[agent.id];
    return track ? stepDividers(top, track.passages, workflows.scans[project.id]?.workflows ?? []) : new Map();
  });
  /** The Workflow tab, on the workflow of the divider. */
  function openWorkflow(workflow: string) {
    workflows.chosen[project.id] = workflow;
    app.panelTab = 'workflow';
    if (!app.split) app.filesOpen = true;
  }
  // The next slice: what « Afficher les N précédents » adds.
  const older = $derived(Math.min(SLICE, start));
  const children = $derived.by(() => {
    const m = new Map<string, ConvItem[]>();
    for (const i of conv.items) {
      if (!i.parent) continue;
      let list = m.get(i.parent);
      if (!list) m.set(i.parent, (list = []));
      list.push(i);
    }
    return m;
  });
  // What the last turn edited, for its recap: worked out once it has ended, not while streaming,
  // and not again on each update of the agent (a new object, same folder).
  const cwd = $derived(agent.cwd);
  const lastEdits = $derived.by(() => {
    const last = top.at(-1);
    return last?.kind === 'turn' ? (editsByTurn(conv.items, cwd).get(last.id) ?? []) : [];
  });
  // The conversation's latest turn, which tells when an agent stopped by the usage limit resumes.
  const latestTurn = $derived.by(() => {
    for (let i = top.length - 1; i >= 0; i--) if (top[i].kind === 'turn') return top[i].id;
    return null;
  });
  // Once the conversation went from one harness to another (ESC-88), each turn says the one that ran it.
  const marked = $derived(conv.items.some((i) => i.kind === 'handover'));
  // Subagents named after their tasks, for their messages.
  const labels = $derived(subagentLabels(conv.items));
  // The local commands conversations saved before kept with their tags (ESC-154).
  const commands = $derived(localCommandViews(conv.items));
  // Its worktree waits for its first message (ESC-156): on no branch yet, not on the project's folder's.
  // Without one, the worktree it went into by itself (ESC-161), else the project's folder's.
  const branch = $derived(ownBranch(agent) ?? (agent.worktreePending ? '' : (app.git[project.id]?.branch ?? '')));
  /** Of several repositories (ESC-169): those the agent changed, after its branch (« · api, front »). */
  const changedRepos = $derived(isMulti(project) ? touchedRepos(project, agent).map(repoLabel) : []);
  const files = $derived(app.git[project.id]?.agents[agent.id] ?? 0);
  // Its pull requests (ESC-64): read when it is shown, then every minute while it is (the backend asks its host at most
  // every 20 s, and no more once they are merged).
  const prs = $derived(cardPrs(agent));
  const word = $derived(prWord(agent));
  /** « Ouvrir 2 PR »: one per repository changed. */
  const openPrLabel = $derived(
    changedRepos.length > 1 ? t('repos.pr.button', { count: changedRepos.length, word }) : t('prs.openButton', { word }),
  );
  const agentId = $derived(agent.id);
  $effect(() => {
    const id = agentId;
    untrack(() => refreshPrs(app.agents[id]));
    const timer = setInterval(() => refreshPrs(app.agents[id]), PRS_EVERY_MS);
    return () => clearInterval(timer);
  });
  // The branch its folder is really on (`GitInfo.agentGit`), else the one it shows: switched from the status bar, by the
  // agent itself, or the project's folder moved, the new one's pull requests are read at once.
  const realBranch = $derived(agentGitOf(agent, app.git[project.id])?.branch ?? branch);
  let readOn: string | null = null;
  $effect(() => {
    const now = realBranch;
    if (readOn !== null && now !== readOn) untrack(() => refreshPrs(app.agents[agentId], true));
    readOn = now;
  });
  const used = $derived(spent(agent));
  // How full the context is, out of the window of the conversation's model (once a turn told it).
  // How full the context is, out of the window of the conversation's model (once a turn told it): its window in a lighter
  // weight after the tokens, and a thin gauge under them.
  const context = $derived.by(() => {
    const used = agent.contextTokens;
    const size = agent.contextWindow;
    const use = contextUse(agent);
    if (!use) return { shown: used ? fTok(used) : '—', of: '', pct: null, title: t('conv.header.contextNow'), full: false };
    const window =
      size >= 1e6 ? t('format.tokens.million', { n: fNum(size / 1e6, 2) }) : t('format.tokens.thousand', { n: Math.round(size / 1e3) });
    return { shown: fTok(used), of: window, pct: Math.min(100, use.pct), title: use.title, full: use.full };
  });
  // Its sandbox, which its harness's modes set (Codex), said in its header: what it means in its mode. Bypass has none.
  const sandbox = $derived.by(() => {
    const caps = capsOf(agent.engine);
    if (!caps?.sandbox || agent.mode === 'bypassPermissions') return '';
    return caps.modes.find((m) => m.id === agent.mode)?.meaning ?? t('conv.header.sandboxDefault');
  });
  // A message refused for want of an account of its harness: said until one is there (choice 19).
  const noAccount = $derived(conv.noAccount && !accountsOf(conv.noAccount.engine).some((a) => a.active) ? conv.noAccount : null);
  // « Ouvrir une PR » takes the place of the launch while the branch has commits without a pull request (choice 26).
  const prFirst = $derived(canOpenPr(agent) && !openPrOf(agent));
  // The editor shows the agent's own checkout: its worktree, else the project's.
  const editorSource = $derived(agent.worktree ? agent.id : 'project');
  const openFile = (abs: string, line: number | null) =>
    app.openEditor({ projectId: project.id, source: editorSource, abs, line: line ?? undefined });
  /** Opens the conversation of the subagent `call` launched, in place of the agent's. */
  const openSub = (call: string) => app.openSubagent(agent.id, call);
  /** The subagent that made a request, if the conversation or the plan still knows it. */
  const subOf = (call: string) => subagentInfo(agent, conv.items, call);
  const duration = $derived(fDur(agent.activeMs + (agent.activeSince ? app.now - agent.activeSince : 0)));
  const running = $derived(agent.status === 'running');

  let scroller = $state<HTMLDivElement>();
  let content = $state<HTMLDivElement>();
  let olderButton = $state<HTMLButtonElement>();
  let stick = true;
  let showJump = $state(false);
  let lastTop = 0;
  // The observer's first report is the size the view starts from, not new content.
  let sized = false;
  // A view put back in place is put back again while the messages around it are drawn at their real
  // height (content-visibility sizes those off screen by a guess), until the reader takes over.
  let settling: ReadingPlace | null = null;
  let settleEnd = 0;
  // The view is put in place once its conversation is loaded: until then, what it shows is not the reader's place,
  // which is kept as it was, and its content growing is no news.
  let placed = false;

  function atBottom() {
    return !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 24;
  }

  function toBottom() {
    if (scroller) {
      scroller.scrollTop = scroller.scrollHeight;
      lastTop = scroller.scrollTop;
    }
    stick = true;
    showJump = false;
    slid = false;
    // Following the bottom draws the latest items only: those the reader had asked for go.
    from = null;
    remember();
  }

  /** The view leaves the bottom: the items it draws stay, whatever comes after them. */
  function pin() {
    if (from === null) from = top[start]?.id ?? null;
  }

  // Kept as the reader moves: when the view is destroyed it is already out of the page, with no layout to read.
  function remember() {
    if (!scroller || !placed) return;
    conv.place = stick
      ? { stick, top: scroller.scrollTop, anchor: null, from: null }
      : { stick, top: scroller.scrollTop, anchor: anchorOf(scroller), from: top[start]?.id ?? null };
  }

  /**
   * The item whose block is the first to end below the top of the view (found by halving: blocks are in order), and
   * where the block's top edge is relative to the view's. The button above the items and the bubbles below them are
   * not items: the next block that draws one is taken.
   */
  function anchorOf(view: HTMLElement) {
    const blocks = content?.children;
    if (!blocks) return null;
    const viewTop = view.getBoundingClientRect().top;
    let lo = 0;
    let hi = blocks.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (blocks[mid].getBoundingClientRect().bottom > viewTop + 1) hi = mid;
      else lo = mid + 1;
    }
    for (let i = lo; i < blocks.length; i++) {
      const item = (blocks[i] as HTMLElement).dataset.item;
      if (item) return { item, offset: blocks[i].getBoundingClientRect().top - viewTop };
    }
    return null;
  }

  /** The block that draws the item `id`, if it is drawn (not every item draws one). */
  function blockOf(id: string): HTMLElement | undefined {
    for (const b of content?.children ?? []) if (b instanceof HTMLElement && b.dataset.item === id) return b;
  }

  function restore(place: ReadingPlace) {
    if (!scroller) return;
    const block = place.anchor && blockOf(place.anchor.item);
    if (block && place.anchor) {
      scroller.scrollTop += block.getBoundingClientRect().top - scroller.getBoundingClientRect().top - place.anchor.offset;
    } else {
      scroller.scrollTop = place.top;
    }
    lastTop = scroller.scrollTop;
    remember();
  }

  /** Where the items drawn start for a place put back: at the first it drew, or at its anchor if that is older. */
  function windowFor(place: ReadingPlace): string | null {
    const ranks = [place.from, place.anchor?.item].map((id) => (id ? rankOf(id) : -1)).filter((r) => r >= 0);
    return ranks.length ? top[Math.min(...ranks)].id : null;
  }

  // What a slice drawn above says to a screen reader, the view staying on the reader's message. A new node each time
  // (`seq`), for the same text to be read again.
  let drewOlder = $state<{ text: string; seq: number } | null>(null);

  /**
   * Draws the slice before the items drawn. The reader stays on what they were reading: the first message in view keeps
   * its place on screen, as a place put back does while the blocks drawn above it are measured. The focus stays on the
   * button, for the next slice; it goes with the last one: the focus moves on to the conversation, so that the keyboard
   * neither falls back to the page nor starts over. A screen reader is told how many messages came.
   */
  async function showOlder() {
    if (!scroller) return;
    const count = older;
    const at: ReadingPlace = { stick: false, top: scroller.scrollTop, anchor: anchorOf(scroller), from: null };
    stick = false;
    from = top[Math.max(0, start - SLICE)].id;
    await tick();
    settling = at;
    settleEnd = performance.now() + SETTLE_MS;
    restore(at);
    if (!olderButton) scroller.focus({ preventScroll: true });
    drewOlder = {
      text: t('conv.older.drawn', { count, n: fInt(count) }),
      seq: (drewOlder?.seq ?? 0) + 1,
    };
  }

  /** Draws the item `id`, with a few items before it, if it is older than the items drawn. */
  function include(id: string | null) {
    const rank = id === null ? -1 : rankOf(id);
    if (rank < 0) return;
    const first = Math.max(0, rank - LEAD);
    if (first < start) from = top[first].id;
  }

  // The reader scrolling up leaves the bottom, however close to it: messages rendered as they come
  // into view (content-visibility) resize the content, which must not pull them back. Only the
  // reader does, but not going down: the wheel down and the keys that scroll down are left out, so
  // are the modifiers pressed alone (the first keystroke of Ctrl+Entrée or Alt+1…9 answering a card,
  // which then folds), every other key counts (a key up, Ctrl+Home, Shift+Tab, whatever a focused
  // element does with it), and a drag, whose way is not told. Content shrinking under the view, or
  // the message field shrinking back once a message is sent, moves the view up too, even while the
  // reader wheels down, and the conversation must go on following.
  let readerAt = 0;
  let dragging = false;
  const reading = () => dragging || performance.now() - readerAt < 500;
  const STAYING_KEYS = new Set(['ArrowDown', 'PageDown', 'End', 'Shift', 'Control', 'Alt', 'AltGraph', 'Meta', 'OS', 'Hyper', 'Super']);
  const byReader = (e: WheelEvent | KeyboardEvent) => {
    settling = null;
    const stays = e instanceof WheelEvent ? e.deltaY >= 0 : STAYING_KEYS.has(e.key) || (e.key === ' ' && !e.shiftKey);
    if (!stays) readerAt = performance.now();
  };
  const grab = () => {
    dragging = true;
    settling = null;
  };

  // The focus moving to an element above the bottom view (Shift+Tab, from the message field too, whose keys are not
  // the conversation's): the browser brings it into view, which is the reader going up. One already in the bottom
  // view (a card's button, being answered) leaves the conversation following.
  function onFocusIn(e: FocusEvent) {
    const el = e.target;
    if (!scroller || !(el instanceof HTMLElement) || el === scroller) return;
    const at = el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop;
    if (at >= scroller.scrollHeight - scroller.clientHeight) return;
    readerAt = performance.now();
    settling = null;
  }

  // The latest items sliding under a view that follows the bottom take blocks away above it, and the browser lowers
  // the view (scroll anchoring, or clamping): that is no reader leaving the bottom, even one dragging (a selection).
  let slid = false;
  let lastStart = 0;
  $effect(() => {
    const s = start;
    untrack(() => {
      if (s > lastStart && from === null) slid = true;
      lastStart = s;
    });
  });

  function onScroll() {
    if (!scroller) return;
    const at = scroller.scrollTop;
    if (at < lastTop - 1 && reading() && !slid) stick = false;
    else if (atBottom()) stick = true;
    slid = false;
    lastTop = at;
    if (stick) showJump = false;
    else if (placed) pin();
    remember();
  }

  // The view is re-created for each agent ({#key}): put it back where the reader left it (the bottom,
  // for an agent opened the first time or left following the conversation) once, not on every agent update.
  function putBack() {
    const place = conv.place;
    if (place && !place.stick) {
      stick = false;
      from = windowFor(place);
      pin();
      settling = place;
      settleEnd = performance.now() + SETTLE_MS;
      requestAnimationFrame(() => {
        placed = true;
        restore(place);
      });
    } else {
      requestAnimationFrame(() => {
        placed = true;
        toBottom();
      });
    }
  }

  // Back from a subagent's conversation (left by its breadcrumb or Escape): the focus goes back to where it was opened
  // from; a button of this conversation is drawn anew, and found again by where it is (`data-open-from`) and what it
  // opens (`data-open-sub`) — but for the stack of dots of the plan's banner (`dots`), which opens any of the subagents
  // running (a menu, for several) and is found whichever it stands for by now; else the button of its call, once the
  // conversation draws it.
  let focusBack = $state<SubFocus | null>(null);
  $effect(() => {
    const from = focusBack;
    const el = from?.el?.isConnected ? from.el : null;
    if (!from || (!el && !conv.loaded)) return;
    focusBack = null;
    tick().then(() => {
      const same = from.opener
        ? [...document.querySelectorAll<HTMLElement>('main.conv [data-open-from]')].find(
            (e) => e.dataset.openFrom === from.opener && (from.opener === 'dots' || e.dataset.openSub === from.call),
          )
        : undefined;
      const row = [...(scroller?.querySelectorAll<HTMLElement>('[data-item]') ?? [])].find((e) => e.dataset.item === from.call);
      (el ?? same ?? row?.querySelector<HTMLElement>('.open-sub'))?.focus({ preventScroll: true });
    });
  });

  onMount(() => {
    focusBack = app.takeFocusBack(agent.id);
    // The view does not follow the bottom on its way to another place.
    if (conv.jump || (conv.place && !conv.place.stick)) stick = false;
    // Shown, the conversation stays in memory (read again if it had left it).
    return conv.show();
  });

  // Once the conversation is loaded, the view opens where the reader left it, unless a message is asked for (a search
  // result): that wins, whether the view was made before or after the request (another agent's result, or one of the
  // agent on screen). The message is drawn first, if it is older than the items drawn.
  let opened = false;
  let found = $state<string | null>(null);
  let foundTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    if (!conv.loaded) return;
    const id = conv.jump;
    // Not again for each item that comes: what is drawn is only read here.
    untrack(() => {
      if (id) {
        conv.jump = null;
        opened = true;
        include(conv.shownAs(id));
        tick().then(() => reveal(id));
      } else if (!opened) {
        opened = true;
        putBack();
      }
    });
  });
  $effect(() => () => clearTimeout(foundTimer));

  /**
   * Puts the block that shows the item `id` in the middle of the view (its top a little below the view's, when it
   * is taller) and highlights it. The block is found by the item it shows: not every item draws one. Kept there while
   * the blocks around it are measured, as a place put back is.
   */
  function reveal(id: string) {
    const shown = conv.shownAs(id);
    const block = shown === null ? undefined : blockOf(shown);
    if (!scroller || !shown || !block) {
      // Not in the conversation: the view goes where it would have gone.
      if (!placed) putBack();
      return;
    }
    placed = true;
    const room = scroller.clientHeight - block.getBoundingClientRect().height;
    const at: ReadingPlace = { stick: false, top: scroller.scrollTop, anchor: { item: shown, offset: Math.max(24, room / 2) }, from: null };
    stick = false;
    pin();
    showJump = false;
    settling = at;
    settleEnd = performance.now() + SETTLE_MS;
    restore(at);
    found = shown;
    clearTimeout(foundTimer);
    foundTimer = setTimeout(() => (found = null), FOUND_MS);
  }

  $effect(() => {
    if (!content) return;
    const ro = new ResizeObserver(() => {
      if (stick) toBottom();
      else if (settling && performance.now() < settleEnd) restore(settling);
      else if (sized && placed) showJump = true;
      sized = true;
    });
    ro.observe(content);
    return () => ro.disconnect();
  });

  /** « Relancer » of « Outils d’Escouade au prochain démarrage » under way, for this agent. */
  let restarting = $state<string | null>(null);
  /** A turn, a question or the setup of its worktree is under way: it ends by itself. */
  const atWork = $derived(agent.status === 'running' || agent.status === 'waiting' || !!agent.setup);
  /**
   * Something goes on in the background, past the turn: a subagent, a workflow, a task of Claude Code (a command, a
   * monitor, a remote agent, a teammate). A command or a dev server does not end by itself.
   */
  const inBackground = $derived(!!agent.plan?.agents.some((a) => a.status === 'running') || bgCount(agent.plan) > 0);
  /**
   * At work, or with something going on in the background. Its process is restarted once none of that is left (what it
   * waits for, or does, would be lost), as the backend refuses it: the very rows `PlanState::runs_something` counts.
   */
  const working = $derived(atWork || inBackground);
  /** Shown in its harness's own interface, which takes its process's place or joins it (OpenCode). */
  const harnessView = $derived(inHarnessView(agent));
  /** What is said of the wait: a turn ends by itself; what goes on in the background needs to be stopped first. */
  const waits = $derived(atWork ? t('conv.escouadeNext.busy') : inBackground ? t('conv.escouadeNext.busyBackground') : '');

  /** Its process started again on its session (ESC-91); in the Claude Code view, its interface (`AgentTerminal`). */
  async function restartProcess() {
    if (agent.terminal) {
      app.relaunched[agent.id] = (app.relaunched[agent.id] ?? 0) + 1;
      return;
    }
    const id = agent.id;
    restarting = id;
    await app.run(api.restartAgentProcess(id));
    if (restarting === id) restarting = null;
  }

  /** What the header, folded by its width (its container queries), hides: « … » under `more` offers it then. */
  function folded(more: HTMLElement): HeaderFolded {
    const head = more.closest('.head');
    const hidden = (sel: string) => {
      const el = head?.querySelector(sel);
      return !el || (typeof el.checkVisibility === 'function' && !el.checkVisibility());
    };
    return { files: !app.split && hidden('.fig.files'), layout: hidden('.layout-seg') };
  }

  function rename() {
    app.modal = {
      kind: 'rename',
      title: t('conv.header.renameTitle'),
      value: agent.name,
      onSubmit: async (name) => {
        // Its branch follows the name, unless it must keep its own: said, the agent renamed all the same.
        const kept = await app.run(api.renameAgent(agent.id, name));
        if (kept) app.toast(kept, 'info');
      },
    };
  }

  /** Whether the next item drawn after the `j`-th is a tool call too: the calls of a turn are tied in one thread. */
  function nextIsTool(j: number) {
    for (let k = j + 1; k < drawn.length; k++) if (shows(drawn[k])) return isCall(drawn[k]);
    return false;
  }

  /** A tool call drawn as a row of the thread: not the files the agent sent, which have their own card (ESC-171). */
  function isCall(item: ConvItem) {
    return item.kind === 'tool' && item.name !== 'SendUserFile';
  }

  /** Items that draw nothing: an empty block of text, the calls whose question or plan has its own card. */
  function shows(item: ConvItem) {
    if (item.kind === 'text') return !!item.text.trim() || item.streaming || !!item.spoken;
    if (item.kind === 'thinking') return !!item.text.trim() || item.streaming;
    if (item.kind === 'tool') return item.name !== 'AskUserQuestion' && item.name !== 'ExitPlanMode';
    if (item.kind === 'user') {
      const command = commands.get(item.id);
      return command?.kind !== 'notices' || command.notices.length > 0;
    }
    return true;
  }
</script>

<svelte:window onpointerup={() => (dragging = false)} onpointercancel={() => (dragging = false)} />

<!-- « Outils d’Escouade au prochain démarrage »: its process started before Claude and the agents could drive Escouade. -->
{#snippet nextStart()}
  {#if agent.escouadeNext && !agent.archived}
    <div class="banner next" class:top={harnessView} role="region" aria-label={t('conv.escouadeNext.title')}>
      <div class="what">
        <strong>{t('conv.escouadeNext.title')}</strong>
        <span>{t('conv.escouadeNext.text')}{waits ? ` ${waits}` : ''}</span>
      </div>
      <button class="btn" disabled={working || restarting === agent.id} onclick={restartProcess}>{t('conv.escouadeNext.restart')}</button>
    </div>
  {/if}
{/snippet}

<!-- Above a request a subagent made: which one, and the way to its conversation, where it is answered too. -->
{#snippet asks(sub: string | undefined, request: string)}
  {@const from = sub ? subOf(sub) : null}
  {#if sub && from}
    <div class="asks">
      <Icon name="sub" size={14} /><span>{t('subagents.asks', { name: from.title })}</span>
      <button class="open-sub" data-open-sub={sub} data-open-from="asks:{request}" onclick={() => openSub(sub)}
        >{t('subagents.open')}</button
      >
    </div>
  {/if}
{/snippet}

<!-- `.conv` is where the keys answering a waiting request apply (answersHere, in shortcuts.ts). -->
<main class="conv">
  <header class="head">
    <div class="h-title">
      <div class="row">
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <h1 class="name" ondblclick={rename} use:tip={t('conv.header.renameHint')}>{agent.name}</h1>
        <span class="state {PILL[agent.status] ?? 'err'}" data-status={agent.status}><i></i>{statusLabel(agent.status)}</span>
      </div>
      <div class="crumbs">
        <EngineMark engine={agent.engine} full />
        <span class="proj-name">{project.name}</span><span aria-hidden="true">/</span><span class="mono branch">{branch || '—'}</span
        >{#if changedRepos.length}<span class="repos" use:tip={t('repos.card.changed', { repos: changedRepos.join(', ') })}
            >· {changedRepos.join(', ')}</span
          >{/if}
        <!-- Its pull requests, each opening in the browser (ESC-64), beside its branch (choice 26). -->
        {#each prs as p (prKey(p))}
          <button
            class="prnum {prTone(p)}"
            aria-label={t('prs.chipLabel', { word, number: p.number, state: stateWord(p) })}
            use:tip={{ text: prTip(word, p), aria: false }}
            data-pr={p.number}
            onclick={() => openPr(p)}><Icon name={prIcon(p)} size={12} />{p.repo ? `${repoLabel(p.repo)} ` : ''}#{p.number}</button
          >
        {/each}
        <!-- Codex's sandbox, which its modes set (but Bypass, which has none): said where it matters. -->
        {#if sandbox}
          <span
            class="sbx"
            role="img"
            aria-label={t('conv.header.sandboxLabel', { meaning: sandbox })}
            use:tip={{ text: sandbox, aria: false }}
            ><Icon name="lock" size={12} /><span class="sbx-l">{t('conv.header.sandbox')}</span></span
          >
        {/if}
      </div>
    </div>
    <dl class="figures">
      <div class="fig" use:tip={context.title}>
        <dt>{t('conv.header.context')}</dt>
        <dd class:full={context.full}>
          {context.shown}{#if context.of}{' '}<small>/ {context.of}</small>{/if}
        </dd>
        {#if context.pct !== null}<div class="ctx-meter" class:full={context.full}><i style:width="{context.pct}%"></i></div>{/if}
      </div>
      {#if showsCost(agent)}
        <!-- A harness that tells neither tokens nor cost (Cursor CLI): neither is shown. -->
        <div class="fig fig-tok">
          <dt>{t('conv.header.tokens')}</dt>
          <dd>{fTok(used.tokens)}</dd>
        </div>
        <div class="fig" use:tip={used.estimated ? estimateHint(used) : undefined}>
          <dt>{t('common.cost')}</dt>
          <dd>
            {fSpentUsd(used)}{#if used.estimated}<small class="est">{t('stats.cost.estimated')}</small>{/if}
          </dd>
        </div>
      {/if}
      {#if app.split}
        <div class="fig fig-files">
          <dt>{t('common.files')}</dt>
          <dd>{files}</dd>
        </div>
      {:else}
        <button
          class="fig files fig-files"
          class:open={app.filesOpen}
          aria-pressed={app.filesOpen}
          use:tip={t('conv.header.filesHint')}
          onclick={() => (app.filesOpen = !app.filesOpen)}
        >
          <dt>{t('common.files')}</dt>
          <dd>{files}</dd>
        </button>
      {/if}
      <div class="fig fig-dur">
        <dt>{t('conv.header.duration')}</dt>
        <dd>{duration}</dd>
      </div>
    </dl>
    <div class="actions">
      <!-- One place for a button (choice 26): « Ouvrir une PR » while the branch has commits without one, the launch then
           in « … »; else the launch, prepared or tested. -->
      {#if prFirst}
        <button
          class="btn openpr"
          aria-label={openPrLabel}
          use:tip={t('prs.openHint', { word, branch: ownBranch(agent) ?? '' })}
          onclick={() => askOpenPr(agent)}
        >
          <Icon name="pr" size={14} /><span class="lbl">{openPrLabel}</span>
        </button>
      {:else if agent.recipe || agent.isola}
        {#if canTest(agent)}
          <button
            class="btn test"
            aria-label={t('conv.header.test')}
            use:tip={agent.isola ? t('conv.header.testIsolaHint') : t('conv.header.testHint')}
            onclick={() => testAgent(agent, project)}
          >
            <Icon name="play" size={14} /><span class="lbl">{t('conv.header.test')}</span>
          </button>
        {/if}
      {:else if canPrepare(agent)}
        <button
          class="btn test"
          aria-label={t('conv.header.prepare')}
          use:tip={t('conv.header.prepareHint')}
          onclick={() => prepareLaunch(agent)}
        >
          <Icon name="play" size={14} /><span class="lbl">{t('conv.header.prepare')}</span>
        </button>
      {/if}
      <!-- « Escouade | <harness> »: never for a ticket's agent, which its board drives over the stream. The view shown is
           named, the other one an icon. -->
      {#if canSwitchView(agent)}
        <div class="vseg" role="group" aria-label={t('conv.header.view')}>
          <button
            aria-pressed={!harnessView}
            aria-label={t('conv.header.toRich')}
            use:tip={harnessView ? t('conv.header.toRichHint') : undefined}
            onclick={() => harnessView && switchView(agent)}
            ><Icon name="agents" size={14} /><span class="lbl">{t('conv.header.rich')}</span></button
          >
          <button
            aria-pressed={harnessView}
            aria-label={t('conv.header.toTerminal', { engine: engineName(agent.engine) })}
            use:tip={harnessView ? undefined : t('conv.header.toTerminalHint', { engine: engineName(agent.engine) })}
            onclick={() => !harnessView && switchView(agent)}
            ><Icon name="term" size={14} /><span class="lbl">{engineName(agent.engine)}</span></button
          >
        </div>
      {/if}
      <!-- The agent's menu, the one of its card's right click (the sidebar draws it). -->
      <button
        class="btn icon more"
        aria-haspopup="menu"
        aria-label={t('conv.header.more')}
        use:tip={t('conv.header.more')}
        onclick={(e) => app.askAgentMenu(agent.id, e.currentTarget, folded(e.currentTarget))}><Icon name="more" size={14} /></button
      >
      <div class="layout-seg" role="group" aria-label={t('conv.header.layout', { key: keyLabel('Ctrl+Shift+L') })}>
        <button
          aria-pressed={!app.split}
          aria-label={t('conv.header.layoutClassic')}
          use:tip={t('conv.header.layoutClassic')}
          onclick={() => app.split && app.toggleLayout()}><Icon name="lay-1" size={14} /></button
        >
        <button
          aria-pressed={app.split}
          aria-label={t('conv.header.layoutSplit')}
          use:tip={t('conv.header.layoutSplit')}
          onclick={() => !app.split && app.toggleLayout()}><Icon name="lay-2" size={14} /></button
        >
      </div>
    </div>
  </header>

  <!-- Where the agent is in its plan, when it follows one: it takes the room it needs, never the messages' last 160 px. -->
  <PlanBanner {agent} />

  {#if harnessView && !agent.archived}
    {@render nextStart()}
    <AgentTerminal {agent} />
  {:else}
    <!-- Focusable for the keyboard to go on reading once the last of the older items are drawn (showOlder). -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="scroll"
      tabindex="-1"
      bind:this={scroller}
      onscroll={onScroll}
      onwheel={byReader}
      onkeydown={byReader}
      onfocusin={onFocusIn}
      onpointerdown={grab}
    >
      <div class="msgs" bind:this={content}>
        {#if conv.error}
          <div class="load-error">
            {t('conv.loadError', { error: conv.error })}
            <button class="btn" onclick={() => conv.load()}>{t('common.retry')}</button>
          </div>
        {:else if conv.loaded && top.length === 0 && !conv.waiting.length && !heldOf(agent.id).length}
          <div class="empty">
            <span class="t">{t('conv.empty.title')}</span>
            <span class="s"
              ><Rich k="conv.empty.hint" engine={speakerOf(agent.engine)}
                >{#snippet cwd()}<span class="mono">{agent.cwd}</span>{/snippet}</Rich
              ></span
            >
            {#if !app.enginesFound.includes(agent.engine ?? 'claude')}
              <span class="warn">{t('conv.empty.engineMissing', { engine: engineName(agent.engine) })}</span>
            {/if}
          </div>
        {/if}
        {#if older > 0}
          <button class="btn older" bind:this={olderButton} onclick={showOlder}>
            {t('conv.older.show', { count: older, n: fInt(older) })}
          </button>
        {/if}
        {#each drawn as item, j (item.id)}
          <!-- Its rank in the whole conversation. -->
          {@const i = start + j}
          {#each dividers.get(item.id) ?? [] as d (d.key)}
            <div class="wf-div" data-testid="step-divider">
              <button use:tip={t('workflow.divider')} onclick={() => openWorkflow(d.workflow)}
                ><Icon name="flow" size={14} />{d.title}</button
              >{#if d.door}<span class="mono">{d.door}</span>{/if}<span>{fTime(d.at)}</span>
            </div>
          {/each}
          {#if shows(item)}
            <!-- The block of an item, found by it (reveal). -->
            <div class="item" class:found={found === item.id} class:tool-item={isCall(item)} data-item={item.id}>
              {#if item.kind === 'user'}
                {@const injected = injectedSource(item)}
                {@const command = commands.get(item.id)}
                {#if command?.kind === 'user'}
                  <UserMessage item={{ ...item, text: command.text }} engine={agent.engine} />
                {:else if command?.kind === 'wake'}
                  <EventMessage source="wake" text={command.text} />
                {:else if command}
                  {#each command.notices as notice, k (k)}
                    <Notice item={{ kind: 'notice', id: item.id, ts: item.ts, ...notice }} />
                  {/each}
                {:else if injected}
                  <EventMessage source={injected} text={item.text} label={labels.get(parseAgentMessage(item.text).from ?? '')} />
                {:else}
                  <UserMessage {item} engine={agent.engine} />
                {/if}
              {:else if item.kind === 'event'}
                <EventMessage source={item.source} text={item.text} label={labels.get(item.from ?? '')} />
              {:else if item.kind === 'text'}
                <div class="assistant">
                  <div class="blocks">
                    {#each splitEscouade(item.streaming ? withoutPassage(item.text) : item.text) as seg, si (si)}
                      {#if seg.kind === 'report'}
                        <CriteriaReport report={seg.report} criteria={ticket?.criteria ?? []} {agent} />
                      {:else}
                        <Markdown text={seg.text} streaming={item.streaming} agentId={agent.id} />
                      {/if}
                    {/each}
                    {#if item.spoken}
                      <!-- The passage the agent wrote to be read aloud in a voice conversation (ESC-97). -->
                      <blockquote class="spoken" data-spoken>
                        <span class="aloud"><Icon name="sound" size={14} />{t('voice.conversation.read')}</span>
                        {item.spoken}
                      </blockquote>
                    {/if}
                  </div>
                </div>
              {:else if item.kind === 'thinking'}
                <Thinking {item} />
              {:else if item.kind === 'tool' && item.name === 'SendUserFile' && !item.parent}
                <SentFiles {item} agentId={agent.id} engine={agent.engine} />
              {:else if item.kind === 'tool'}
                <ToolRow
                  {item}
                  cwd={agent.cwd}
                  linked={nextIsTool(j)}
                  childrenOf={(id) => children.get(id) ?? []}
                  onOpenFile={openFile}
                  onOpenSub={openSub}
                />
              {:else if item.kind === 'question'}
                {@render asks(item.sub, item.id)}
                <QuestionCard {item} agentId={agent.id} pending={agent.pending.includes(item.id)} current={agent.pending[0] === item.id} />
              {:else if item.kind === 'permission'}
                {@render asks(item.sub, item.id)}
                <PermissionCard
                  {item}
                  agentId={agent.id}
                  cwd={agent.cwd}
                  pending={agent.pending.includes(item.id)}
                  current={agent.pending[0] === item.id}
                />
              {:else if item.kind === 'turn'}
                <TurnCard
                  {item}
                  {agent}
                  last={i === top.length - 1}
                  latest={item.id === latestTurn}
                  edits={i === top.length - 1 ? lastEdits : []}
                  {marked}
                />
              {:else if item.kind === 'handover'}
                <Handover {item} />
              {:else if item.kind === 'notice'}
                <Notice {item} />
              {/if}
            </div>
          {/if}
        {/each}
        {#each conv.waiting as item (item.id)}
          <UserMessage {item} waiting engine={agent.engine} />
        {/each}
        {#each heldOf(agent.id) as held (held.id)}
          <HeldMessage {held} engine={agent.engine} />
        {/each}
        {#if noAccount}
          <NoAccount {agent} engine={noAccount.engine} text={noAccount.text} />
        {/if}
        {#if running}
          <div class="working">
            <span class="spark"><Icon name="spark" size={14} /></span><span class="shimmer"
              >{t('conv.working', { engine: speakerOf(agent.engine) })}</span
            >
          </div>
        {/if}
        {#if agent.setup}
          <SetupProgress {agent} />
        {:else if agent.worktreePending}
          <p class="later">{t('conv.setup.later')}</p>
        {/if}
      </div>
    </div>

    <!-- Out of the list: its blocks are the conversation's. -->
    <div class="sr" aria-live="polite" aria-atomic="true">
      {#if drewOlder}
        {#key drewOlder.seq}<span>{drewOlder.text}</span>{/key}
      {/if}
    </div>

    {#if showJump}
      <button class="jump" onclick={toBottom}><Icon name="pull" size={14} />{t('conv.newMessages')}</button>
    {/if}

    {#if agent.archived}
      <div class="banner">
        {t('conv.archived.text')}
        <button class="btn" onclick={() => app.run(api.archiveAgent(agent.id, false))}>{t('conv.archived.restore')}</button>
      </div>
    {:else}
      {#if agent.status === 'error'}
        <div class="banner err">{t('conv.stopped', { engine: speakerOf(agent.engine) })}</div>
      {/if}
      {@render nextStart()}
      <Composer {agent} />
    {/if}
  {/if}
</main>

<style>
  /* `conv`: the container its header and its composer fold by (container queries), as the preview's. */
  .conv {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container: conv / inline-size;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 0 14px 0 22px;
    border-bottom: 1px solid var(--line);
  }
  .h-title {
    display: grid;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }
  .h-title .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .state {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px 0 7px;
    border-radius: 10px;
    background: oklch(from var(--run) l c h / 0.1);
    color: var(--run);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .state i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .state.wait {
    background: oklch(from var(--wait) l c h / 0.1);
    color: var(--wait);
  }
  .state.idle {
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--muted);
  }
  .state.err {
    background: oklch(from var(--del) l c h / 0.1);
    color: var(--del);
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }
  .crumbs :global(.mark) {
    margin-right: 2px;
  }
  /* Short of room, the project's name gives way first, then the branch: « dem…/agent-1 ». */
  .proj-name {
    flex: 0 1000 auto;
    min-width: 2ch;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .branch {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The repositories changed, after the branch: the last to go when the header folds (ESC-169). */
  .crumbs .repos {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A pull request's number beside the branch: the icon of its state coloured by what to look at first (ESC-64). */
  .prnum {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 20px;
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-family: var(--mono);
    font-size: 11.5px;
    cursor: pointer;
  }
  .prnum:hover {
    background: var(--hover);
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
  .sbx {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 18px;
    margin-left: 4px;
    padding: 0 7px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.045);
    color: var(--soft);
    font-size: 11px;
    font-weight: 500;
    cursor: help;
  }
  .sbx :global(.icon) {
    color: var(--muted);
  }
  /* The figures: a label over each value. The margin of a list, as the preview keeps it, makes the header 69 px. */
  .figures {
    flex: none;
    display: flex;
    gap: 20px;
    margin: 1em 0;
    padding-right: 6px;
  }
  .fig {
    display: grid;
    gap: 1px;
    text-align: left;
  }
  .fig dt {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 500;
  }
  .fig dd {
    margin: 0;
    font-size: 12.5px;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .fig dd small {
    color: var(--muted);
    font-size: inherit;
    font-weight: 450;
  }
  .fig dd .est {
    margin-left: 3px;
  }
  .fig dd.full {
    color: var(--wait);
  }
  .ctx-meter {
    width: 100%;
    height: 2px;
    margin-top: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.1);
  }
  .ctx-meter i {
    display: block;
    height: 100%;
    border-radius: 2px;
    background: var(--soft);
  }
  .ctx-meter.full i {
    background: var(--wait);
  }
  /* « Fichiers », which opens the files beside the conversation. */
  .fig.files {
    margin: -4px -8px;
    padding: 4px 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .fig.files:hover {
    background: var(--hover);
  }
  .fig.files.open {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .actions .btn {
    height: 30px;
  }
  .btn.icon {
    width: 30px;
    padding: 0;
    justify-content: center;
  }
  .vseg {
    display: inline-flex;
    padding: 2px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .vseg button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
    white-space: nowrap;
    cursor: pointer;
  }
  .vseg button:hover {
    color: var(--text);
  }
  .vseg button[aria-pressed='true'] {
    background: rgba(var(--fg-rgb), 0.09);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  .vseg button[aria-pressed='false'] .lbl {
    display: none;
  }
  .layout-seg {
    display: inline-flex;
    padding: 2px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .layout-seg button {
    display: grid;
    place-items: center;
    width: 26px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .layout-seg button[aria-pressed='true'] {
    background: rgba(var(--fg-rgb), 0.08);
    color: var(--text);
  }
  /* The header folds by the width of the conversation, not the window's (the side panel open, the split layout, a small
     window), for its name and its state to keep their room (ESC-166): the figures go one by one (Durée, Fichiers, Tokens),
     then the words of its buttons (their icon and tooltip stay), then the layout and the views, which « … » takes over. The
     path gives up its project's name before its branch. */
  @container conv (max-width: 1100px) {
    .figures .fig-dur,
    .vseg .lbl {
      display: none;
    }
    .sbx {
      gap: 0;
    }
    .sbx-l {
      display: none;
    }
  }
  @container conv (max-width: 960px) {
    .figures .fig-files {
      display: none;
    }
  }
  @container conv (max-width: 880px) {
    .figures .fig-tok {
      display: none;
    }
  }
  @container conv (max-width: 800px) {
    .head .btn .lbl {
      display: none;
    }
    .head .btn:has(.lbl) {
      width: 30px;
      padding: 0;
      justify-content: center;
    }
    .figures {
      gap: 14px;
    }
  }
  @container conv (max-width: 680px) {
    .layout-seg {
      display: none;
    }
  }
  @container conv (max-width: 580px) {
    .figures {
      display: none;
    }
  }
  @container conv (max-width: 440px) {
    .vseg {
      display: none;
    }
  }
  /* The plan banner above shrinks before the messages go under 160 px. */
  .scroll {
    flex: 1 0 160px;
    min-height: 160px;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .scroll:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  /* Read by screen readers only. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  /* Above the items drawn: the slice before them. */
  .older {
    justify-self: center;
    height: 28px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }
  /* The thread: one column of 780 px that shrinks with the panel. */
  .msgs {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 18px;
    width: min(780px, 100% - 64px);
    margin: 0 auto;
    padding: 28px 0 24px;
  }
  .msgs > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: auto 60px;
  }
  /* The block of an item lays its content out as the list does. */
  /* The entry in a step of a workflow (ESC-197): the preview's `.wf-div`. */
  .wf-div {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 18px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .wf-div::before,
  .wf-div::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--line);
  }
  .wf-div > * {
    flex: none;
    white-space: nowrap;
  }
  .wf-div button {
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--soft);
    font-weight: 550;
  }
  .wf-div button:hover {
    color: var(--text);
  }
  .wf-div :global(.icon) {
    color: var(--muted);
  }
  .wf-div .mono {
    color: var(--muted);
    font-size: 11px;
    font-weight: 500;
  }
  .item {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-radius: var(--r);
  }
  /* The calls of a turn in one thread, not a box each. */
  .item.tool-item + .item.tool-item {
    margin-top: -18px;
  }
  .item.found {
    animation: found 2s ease-out forwards;
  }
  @keyframes found {
    0%,
    70% {
      background: color-mix(in oklch, var(--p) 9%, transparent);
      outline: 2px solid color-mix(in oklch, var(--p) var(--sel), transparent);
      outline-offset: 6px;
    }
    100% {
      background: transparent;
      outline: 2px solid transparent;
      outline-offset: 6px;
    }
  }
  .empty {
    padding: 90px 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
  }
  .empty .t {
    font-size: 18px;
    font-weight: 600;
  }
  /* Narrow (beside the files), the agent's folder breaks rather than overflow on both sides. */
  .empty .s {
    color: var(--muted);
    max-width: min(420px, 100%);
    overflow-wrap: anywhere;
    text-wrap: pretty;
    line-height: 1.5;
  }
  .empty .warn {
    margin-top: 10px;
    max-width: 420px;
    color: var(--wait);
    font-size: 12.5px;
  }
  .load-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.45);
    font-size: 12.5px;
    color: var(--muted);
  }
  .assistant {
    min-width: 0;
  }
  .blocks {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  /* What the agent wrote to be read aloud in a voice conversation (ESC-97). */
  .spoken {
    margin: 0;
    padding: 2px 0 2px 12px;
    border-left: 1px solid var(--line-3);
    color: var(--text-ai);
    font-size: 13.5px;
    line-height: 1.62;
  }
  .aloud {
    display: flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 11.5px;
  }
  /* « Claude travaille… »: a spark that turns, a reflection that runs over the words. */
  .working {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .spark {
    display: flex;
    color: var(--p-ink);
    animation: spin 3.2s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .shimmer {
    background: linear-gradient(90deg, var(--muted) 0 40%, var(--text) 50%, var(--muted) 60% 100%);
    background-size: 250% 100%;
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
    animation: shimmer 2.4s linear infinite;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -50% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spark {
      animation: none;
    }
    .shimmer {
      animation: none;
      background: none;
      color: var(--muted);
    }
    .item.found {
      animation: none;
      background: color-mix(in oklch, var(--p) 9%, transparent);
      outline: 2px solid color-mix(in oklch, var(--p) var(--sel), transparent);
      outline-offset: 6px;
    }
  }
  .later {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .jump {
    position: absolute;
    left: 50%;
    bottom: 150px;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px 0 10px;
    border: none;
    border-radius: 15px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    color: var(--soft);
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
    z-index: 5;
  }
  .jump:hover {
    color: var(--text);
  }
  /* A band above the composer: archived, stopped, Escouade's tools at the next start. */
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    width: min(780px, 100% - 64px);
    margin: 0 auto 8px;
    padding: 8px 10px 8px 12px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .banner .btn {
    flex: none;
    height: 28px;
  }
  .banner.err {
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .banner.next {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 40%, transparent);
  }
  /* Above the terminal of the Claude Code view rather than above the composer. */
  .banner.top {
    margin-top: 10px;
  }
  .next .what {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .next strong {
    color: var(--text);
    font-weight: 600;
  }
  /* Above a request a subagent made: which one, and the way to its conversation. */
  .asks {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .open-sub {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: var(--p-ink);
    cursor: pointer;
  }
  .open-sub:hover {
    text-decoration: underline;
  }
</style>

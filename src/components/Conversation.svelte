<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { onMount, tick, untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { api } from '../lib/ipc';
  import { conversationOf, type ReadingPlace } from '../lib/conversations.svelte';
  import { splitEscouade } from '../lib/escouade';
  import { fDur, fInt, fNum, fTok } from '../lib/format';
  import { modelLabel } from '../lib/models';
  import { contextUse, ESTIMATE_HINT, fSpentUsd, spent } from '../lib/spend';
  import { injectedSource, parseAgentMessage, subagentLabels } from '../lib/events';
  import { editsByTurn } from '../lib/tools';
  import { app } from '../lib/state.svelte';
  import { canPrepare, canTest, prepareLaunch, testAgent } from '../lib/test-launch.svelte';
  import type { Agent, ConvItem, Project } from '../lib/types';
  import Composer from './Composer.svelte';
  import CriteriaReport from './conv/CriteriaReport.svelte';
  import EventMessage from './conv/EventMessage.svelte';
  import Markdown from './conv/Markdown.svelte';
  import Notice from './conv/Notice.svelte';
  import PermissionCard from './conv/PermissionCard.svelte';
  import QuestionCard from './conv/QuestionCard.svelte';
  import SetupProgress from './conv/SetupProgress.svelte';
  import Thinking from './conv/Thinking.svelte';
  import ToolRow from './conv/ToolRow.svelte';
  import TurnCard from './conv/TurnCard.svelte';
  import UserMessage from './conv/UserMessage.svelte';
  import StatusDot from './StatusDot.svelte';

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
  const SC: Record<string, string> = {
    running: 'var(--ok)',
    waiting: 'var(--wait)',
    idle: 'var(--dim)',
    done: 'var(--ok)',
    error: 'var(--del)',
  };
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
  // Subagents named after their tasks, for their messages.
  const labels = $derived(subagentLabels(conv.items));
  const branch = $derived(agent.worktree?.branch ?? app.git[project.id]?.branch ?? '');
  const files = $derived(app.git[project.id]?.agents[agent.id] ?? 0);
  const used = $derived(spent(agent));
  // How full the context is, out of the window of the conversation's model (once a turn told it).
  const context = $derived.by(() => {
    const used = agent.contextTokens;
    const size = agent.contextWindow;
    const use = contextUse(agent);
    if (!use) return { shown: used ? fTok(used) : '—', title: t('conv.header.contextNow'), full: false };
    const window =
      size >= 1e6 ? t('format.tokens.million', { n: fNum(size / 1e6, 2) }) : t('format.tokens.thousand', { n: Math.round(size / 1e3) });
    return { shown: `${fTok(used)} / ${window}`, title: use.title, full: use.full };
  });
  // The editor shows the agent's own checkout: its worktree, else the project's.
  const editorSource = $derived(agent.worktree ? agent.id : 'project');
  const openFile = (abs: string, line: number | null) =>
    app.openEditor({ projectId: project.id, source: editorSource, abs, line: line ?? undefined });
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

  onMount(() => {
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

  function rename() {
    app.modal = {
      kind: 'rename',
      title: t('conv.header.renameTitle'),
      value: agent.name,
      onSubmit: (name) => app.run(api.renameAgent(agent.id, name)),
    };
  }

  // The first block drawn shows its avatar, even after text left out above it.
  function prevIsText(i: number) {
    return i > start && top[i - 1].kind === 'text';
  }

  /** Items that draw nothing: an empty block of text, the calls whose question or plan has its own card. */
  function shows(item: ConvItem) {
    if (item.kind === 'text' || item.kind === 'thinking') return !!item.text.trim() || item.streaming;
    if (item.kind === 'tool') return item.name !== 'AskUserQuestion' && item.name !== 'ExitPlanMode';
    return true;
  }
</script>

<svelte:window onpointerup={() => (dragging = false)} onpointercancel={() => (dragging = false)} />

<!-- `.conv` is where the keys answering a waiting request apply (answersHere, in shortcuts.ts). -->
<main class="conv">
  <header class="head">
    <div class="who">
      <div class="line1">
        <span class="name" ondblclick={rename} role="presentation" title={t('conv.header.renameHint')}>{agent.name}</span>
        <span class="st" style:color={SC[agent.status]}><StatusDot status={agent.status} size={7} />{statusLabel(agent.status)}</span>
      </div>
      <span class="sub mono">{project.name} / {branch || '—'}</span>
    </div>
    <div style="flex:1"></div>
    <button
      class="btn edit"
      aria-label={t('common.editor')}
      title={t('conv.header.editorHint')}
      onclick={() => app.openEditor({ projectId: project.id, source: editorSource })}
    >
      <span class="mono glyph">&lt;/&gt;</span><span class="lbl">{t('common.editor')}</span>
    </button>
    {#if agent.recipe || agent.isola}
      {#if canTest(agent)}
        <button
          class="btn test"
          aria-label={`▶ ${t('conv.header.test')}`}
          title={agent.isola ? t('conv.header.testIsolaHint') : t('conv.header.testHint')}
          onclick={() => testAgent(agent, project)}
        >
          <span class="glyph">▶</span><span class="lbl">{t('conv.header.test')}</span>
        </button>
      {/if}
    {:else if canPrepare(agent)}
      <button
        class="btn test"
        aria-label={t('conv.header.prepare')}
        title={t('conv.header.prepareHint')}
        onclick={() => prepareLaunch(agent)}
      >
        <span class="glyph">▷</span><span class="lbl">{t('conv.header.prepare')}</span>
      </button>
    {/if}
    <div class="metrics">
      <span class="model mono">{modelLabel(agent.model, app.models)}</span>
      <div class="m" title={context.title}>
        <span class="k">{t('conv.header.context')}</span><span class="v mono" class:full={context.full}>{context.shown}</span>
      </div>
      <div class="m opt"><span class="k">{t('conv.header.tokens')}</span><span class="v mono">{fTok(used.tokens)}</span></div>
      <div class="m opt2" title={used.estimated ? ESTIMATE_HINT : undefined}>
        <span class="k">{t('common.cost')}</span><span class="v mono">{fSpentUsd(used)}</span>
      </div>
      {#if app.split}
        <div class="m opt2"><span class="k">{t('common.files')}</span><span class="v mono">{files}</span></div>
      {:else}
        <button
          class="m files opt2"
          class:open={app.filesOpen}
          title={t('conv.header.filesHint')}
          onclick={() => (app.filesOpen = !app.filesOpen)}
        >
          <span class="k">{t('common.files')} ▸</span><span class="v mono">{files}</span>
        </button>
      {/if}
      <div class="m opt"><span class="k">{t('conv.header.duration')}</span><span class="v mono">{duration}</span></div>
    </div>
    <div class="segmented layout" role="group" aria-label={t('conv.header.layout', { key: keyLabel('Ctrl+Shift+L') })}>
      <button
        class:on={!app.split}
        aria-pressed={!app.split}
        aria-label={t('conv.header.layoutClassic')}
        title={t('conv.header.layoutClassic')}
        onclick={() => app.split && app.toggleLayout()}
      >
        <svg width="16" height="12" viewBox="0 0 16 12" aria-hidden="true"
          ><rect x="0.5" y="0.5" width="15" height="11" rx="1.5" fill="none" stroke="currentColor" /><rect
            x="11"
            y="1"
            width="4"
            height="10"
            fill="currentColor"
            opacity="0.45"
          /></svg
        >
      </button>
      <button
        class:on={app.split}
        aria-pressed={app.split}
        aria-label={t('conv.header.layoutSplit')}
        title={t('conv.header.layoutSplit')}
        onclick={() => !app.split && app.toggleLayout()}
      >
        <svg width="16" height="12" viewBox="0 0 16 12" aria-hidden="true"
          ><rect x="0.5" y="0.5" width="15" height="11" rx="1.5" fill="none" stroke="currentColor" /><rect
            x="8"
            y="1"
            width="7"
            height="10"
            fill="currentColor"
            opacity="0.45"
          /></svg
        >
      </button>
    </div>
  </header>

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
      {:else if conv.loaded && top.length === 0 && !conv.waiting.length}
        <div class="empty">
          <span class="t">{t('conv.empty.title')}</span>
          <span class="s"><Rich k="conv.empty.hint">{#snippet cwd()}<span class="mono">{agent.cwd}</span>{/snippet}</Rich></span>
          {#if !app.claudeFound}
            <span class="warn">{t('conv.empty.claudeMissing')}</span>
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
        {#if shows(item)}
          <!-- The block of an item, found by it (reveal). -->
          <div class="item" class:found={found === item.id} data-item={item.id}>
            {#if item.kind === 'user'}
              {@const injected = injectedSource(item)}
              {#if injected}
                <EventMessage source={injected} text={item.text} label={labels.get(parseAgentMessage(item.text).from ?? '')} />
              {:else}
                <UserMessage {item} />
              {/if}
            {:else if item.kind === 'event'}
              <EventMessage source={item.source} text={item.text} label={labels.get(item.from ?? '')} />
            {:else if item.kind === 'text'}
              <div class="assistant">
                {#if !prevIsText(i)}<span class="avatar">C</span>{:else}<span class="avatar ghost"></span>{/if}
                <div class="blocks">
                  {#each splitEscouade(item.text) as seg, si (si)}
                    {#if seg.kind === 'report'}
                      <CriteriaReport report={seg.report} criteria={ticket?.criteria ?? []} {agent} />
                    {:else}
                      <Markdown text={seg.text} streaming={item.streaming} />
                    {/if}
                  {/each}
                </div>
              </div>
            {:else if item.kind === 'thinking'}
              <Thinking {item} />
            {:else if item.kind === 'tool'}
              <ToolRow {item} cwd={agent.cwd} childrenOf={(id) => children.get(id) ?? []} onOpenFile={openFile} />
            {:else if item.kind === 'question'}
              <QuestionCard {item} agentId={agent.id} pending={agent.pending.includes(item.id)} current={agent.pending[0] === item.id} />
            {:else if item.kind === 'permission'}
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
              />
            {:else if item.kind === 'notice'}
              <Notice {item} />
            {/if}
          </div>
        {/if}
      {/each}
      {#each conv.waiting as item (item.id)}
        <UserMessage {item} waiting />
      {/each}
      {#if running}
        <div class="working">
          <span class="dots"><span></span><span></span><span></span></span>{t('conv.working')}
        </div>
      {/if}
      {#if agent.setup}
        <SetupProgress {agent} />
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
    <button class="jump" onclick={toBottom}>↓ {t('conv.newMessages')}</button>
  {/if}

  {#if agent.archived}
    <div class="banner">
      {t('conv.archived.text')}
      <button class="btn" onclick={() => app.run(api.archiveAgent(agent.id, false))}>{t('conv.archived.restore')}</button>
    </div>
  {:else}
    {#if agent.status === 'error'}
      <div class="banner err">{t('conv.stopped')}</div>
    {/if}
    <Composer {agent} />
  {/if}
</main>

<style>
  .conv {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    container-type: inline-size;
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 24px;
    border-bottom: 1px solid var(--line);
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    white-space: nowrap;
  }
  .line1 {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    font-size: 15px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .st {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .sub {
    font-size: 11px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .edit,
  .test {
    height: 28px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 11px;
    font-size: 12px;
    font-weight: 600;
    flex: none;
  }
  .edit .glyph,
  .test .glyph {
    font-size: 11px;
    color: var(--accent);
  }
  .metrics {
    display: flex;
    align-items: center;
    gap: 22px;
    flex: none;
    white-space: nowrap;
  }
  .model {
    font-size: 11px;
    padding: 4px 8px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
  }
  .m {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .k {
    font-size: 10.5px;
    color: var(--dim);
  }
  .v {
    font-size: 12.5px;
  }
  .v.full {
    color: var(--wait);
  }
  .files {
    align-items: flex-start;
    padding: 4px 8px;
    margin: -4px -8px;
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    background: transparent;
    cursor: pointer;
    text-align: left;
  }
  .files:hover {
    border-color: var(--line2);
  }
  .files.open {
    background: var(--elev);
    border-color: var(--line2);
  }
  .layout {
    flex: none;
  }
  /* Half-width conversation (split layout): the agent's card in the sidebar shows these too. */
  @container (max-width: 680px) {
    .opt {
      display: none;
    }
    .edit .lbl,
    .test .lbl {
      display: none;
    }
  }
  @container (max-width: 440px) {
    .opt2 {
      display: none;
    }
  }
  .layout button {
    display: flex;
    align-items: center;
    padding: 0 7px;
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
  .scroll:focus-visible {
    outline: 2px solid var(--accent);
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
    align-self: center;
    height: 28px;
    padding: 0 12px;
    font-size: 12px;
    font-weight: 500;
    color: var(--muted);
  }
  .msgs {
    max-width: 780px;
    margin: 0 auto;
    padding: 28px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .msgs > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: auto 60px;
  }
  /* The block of an item lays its content out as the list does. */
  .item {
    display: flex;
    flex-direction: column;
    min-width: 0;
    border-radius: var(--r);
  }
  .item.found {
    animation: found 2s ease-out forwards;
  }
  @keyframes found {
    0%,
    70% {
      background: color-mix(in oklch, var(--accent) 9%, transparent);
      outline: 2px solid var(--accent-soft);
      outline-offset: 6px;
    }
    100% {
      background: transparent;
      outline: 2px solid transparent;
      outline-offset: 6px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .item.found {
      animation: none;
      background: color-mix(in oklch, var(--accent) 9%, transparent);
      outline: 2px solid var(--accent-soft);
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
  .empty .s {
    color: var(--muted);
    max-width: 420px;
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
    border-radius: var(--r);
    border: 1px solid color-mix(in oklch, var(--del) 50%, transparent);
    font-size: 12.5px;
    color: var(--muted);
  }
  .assistant {
    display: flex;
    gap: 12px;
    min-width: 0;
  }
  .blocks {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .avatar {
    width: 22px;
    height: 22px;
    flex: none;
    margin-top: 1px;
    border-radius: var(--r-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .avatar.ghost {
    background: transparent;
  }
  .working {
    margin-left: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: ccBlink 1.2s infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .jump {
    position: absolute;
    left: 50%;
    bottom: 150px;
    transform: translateX(-50%);
    height: 30px;
    padding: 0 14px;
    border-radius: 99px;
    border: 1px solid var(--line2);
    background: var(--elev);
    font-size: 12px;
    cursor: pointer;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    z-index: 5;
  }
  .banner {
    max-width: 780px;
    width: calc(100% - 56px);
    margin: 0 auto 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--panel);
    font-size: 12.5px;
    color: var(--muted);
  }
  .banner.err {
    border-color: color-mix(in oklch, var(--del) 50%, transparent);
  }
</style>

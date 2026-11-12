<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { conversationOf } from '../lib/conversations.svelte';
  import { fDur, fInt, fTok } from '../lib/format';
  import { modelLabel } from '../lib/models';
  import { app } from '../lib/state.svelte';
  import { menu } from '../lib/menu.svelte';
  import { crumbs, subagentInfo, subDuration, threadOf } from '../lib/subagents';
  import type { Agent, ConvItem, Project, TranscriptCursor } from '../lib/types';
  import Markdown from './conv/Markdown.svelte';
  import PermissionCard from './conv/PermissionCard.svelte';
  import QuestionCard from './conv/QuestionCard.svelte';
  import Thinking from './conv/Thinking.svelte';
  import ToolRow from './conv/ToolRow.svelte';
  import { tip } from '../lib/tip';
  import EngineMark from './EngineMark.svelte';
  import Icon from './Icon.svelte';

  // The conversation of a subagent of `agent`, in place of the agent's (ESC-13): `call` is the call that launched it.
  // Its thread is the items of the agent's conversation that name the call (`parent`), and the requests it made; its
  // head says what it is, its end what it handed back. « Arrêter » stops it alone (Claude Code's `stop_task`); a
  // message to it goes through the agent, which is said (Claude Code lets no host write to a subagent). In the Claude
  // Code view the stream tells nothing of it: its thread is read, and only read, from the transcript Claude Code keeps.
  let { agent, project, call }: { agent: Agent; project: Project; call: string } = $props();

  /** A prompt longer than that is folded to its first lines. */
  const LONG_PROMPT = 360;

  const conv = $derived(conversationOf(agent.id));
  const info = $derived(subagentInfo(agent, conv.items, call));
  const thread = $derived(threadOf(conv.items, call));
  const chain = $derived(crumbs(agent, conv.items, call));
  // The items of each thread, for the calls drawn: a tool row unfolds what its subagent did.
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
  const running = $derived(info?.status === 'running');
  /** Known to be over (a status nothing tells is not). */
  const ended = $derived(!!info?.status && info.status !== 'running');
  const duration = $derived(info ? subDuration(info, app.now) : null);
  /** In the Claude Code view, its thread is in Claude Code's transcript, and its interface acts on it. */
  const inTerminal = $derived(agent.terminal);
  // In the background only: in the foreground the agent waits for it (its own « Arrêter » stops it with its turn), and
  // stopping one alone there was never tried on a real stream.
  const stoppable = $derived(running && info?.background === true && !inTerminal);
  const canStop = $derived(stoppable && !!info?.taskId);
  // While it runs, and in the background: the plan never opens an ended row again, so one taken up again by a message
  // would still say it ended; in the foreground the agent waits for it, and would pass the message on once it ended.
  const canWrite = $derived(running && !!info?.taskId && info?.background === true && !inTerminal && !agent.archived);
  const statusWord = $derived(info ? t(`subagents.status.${info.status ?? 'unknown'}`) : '');
  const editorSource = $derived(agent.worktree ? agent.id : 'project');
  const openFile = (abs: string, line: number | null) =>
    app.openEditor({ projectId: project.id, source: editorSource, abs, line: line ?? undefined });
  const openSub = (id: string) => app.openSubagent(agent.id, id);

  let promptOpen = $state(false);
  let stopping = $state(false);
  let sending = $state(false);

  // What is typed to it is the app's, not the view's: it outlives the subagent and a change of view.
  const draftKey = $derived(`${agent.id}:${call}`);
  const message = $derived(app.subDrafts[draftKey] ?? '');
  const setMessage = (v: string) => {
    if (v) app.subDrafts[draftKey] = v;
    else delete app.subDrafts[draftKey];
  };
  const hasDraft = $derived(message.trim() !== '');
  // The field stays for a draft once it ended, even emptied: it does not vanish under the cursor.
  let kept = $state(false);
  $effect(() => {
    if (hasDraft) kept = true;
  });
  const showWrite = $derived(!!info && !inTerminal && (canWrite || (ended && (hasDraft || kept))));

  async function stop() {
    if (!canStop || stopping) return;
    stopping = true;
    await app.run(api.stopSubagent(agent.id, call));
    stopping = false;
  }

  async function send() {
    const text = message.trim();
    if (!text || sending || !canWrite) return;
    sending = true;
    const sent = await app.run(api.messageSubagent(agent.id, call, text).then(() => true));
    sending = false;
    if (sent) {
      setMessage('');
      app.toast(t('subagents.writeSent', { name: agent.name }));
    }
  }

  // Its transcript, in the Claude Code view: read when the view opens, then every 2 s while the subagent runs (and once
  // more when it ends, for what it said last). Each read gives back the cursor of the one before, and the backend answers
  // null while its files are as that read left them: nothing is read or parsed, and nothing is touched here. A read that
  // finds it ended (stopped in Claude Code's interface, an API error) has the backend end its row of the plan and send the
  // agent: the row changes, `running` with it, and the timer stops.
  const agentId = $derived(agent.id);
  const reading = $derived(inTerminal && info !== null);
  // The list is replaced as a whole by each read, never changed in place.
  let side = $state.raw<ConvItem[]>([]);
  let sideError = $state<string | null>(null);
  let sideRead = $state(false);
  // Not state: only the next read looks at it. Whose it is, for a view that goes to another agent's subagent.
  let cursor: TranscriptCursor | null = null;
  let cursorOf = '';
  $effect(() => {
    if (!reading) return;
    const id = agentId;
    const live = running;
    let gone = false;
    let busy = false;
    async function read() {
      // A read slower than the timer is not stacked on the next.
      if (busy) return;
      busy = true;
      try {
        if (cursorOf !== `${id}:${call}`) {
          cursorOf = `${id}:${call}`;
          cursor = null;
        }
        const got = await api.subagentTranscript(id, call, cursor);
        if (!gone) {
          if (got) {
            side = got.items;
            cursor = got.cursor;
          }
          sideError = null;
        }
      } catch (e) {
        if (!gone) sideError = String(e);
      } finally {
        busy = false;
        if (!gone) sideRead = true;
      }
    }
    read();
    const timer = live ? setInterval(read, 2_000) : undefined;
    return () => {
      gone = true;
      clearInterval(timer);
    };
  });
  /** What it handed back, as its transcript tells it: the message of its last `SubagentHandback`. */
  const handedBack = $derived.by(() => {
    for (let i = side.length - 1; i >= 0; i--) {
      const item = side[i];
      const text = item.kind === 'tool' && item.name === 'SubagentHandback' ? item.input?.message : undefined;
      if (typeof text === 'string' && text.trim()) return text;
    }
    return undefined;
  });
  const report = $derived(info?.report ?? (inTerminal ? handedBack : undefined));
  /** The items drawn: its thread in the agent's conversation, else its transcript, whose calls without a result are over with it. */
  const shown = $derived(
    inTerminal
      ? side.map((i) => (ended && i.kind === 'tool' && i.status === 'running' ? { ...i, status: 'interrupted' as const } : i))
      : thread,
  );

  // Escape goes back to the agent's conversation. Seen before the others (capture): a dialog or a menu open closes
  // first, alone; a field with text in it keeps its Escape (a draft is not lost to a key).
  function onWindowKey(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing || app.modal || menu.open) return;
    const typed = e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLInputElement ? e.target.value.trim() : '';
    if (typed) return;
    app.closeSubagent(agent.id);
  }

  /** Where the focus goes when the view opens: the crumb it is at, so that the keyboard is in the view (and Escape heard). */
  function focusHere(node: HTMLElement) {
    node.focus({ preventScroll: true });
  }

  // The view follows the bottom of its thread while the reader is there, as the agent's conversation does.
  let scroller = $state<HTMLDivElement>();
  let content = $state<HTMLDivElement>();
  let stick = true;
  const atBottom = () => !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 24;
  function toBottom() {
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }
  $effect(() => {
    if (!content) return;
    const ro = new ResizeObserver(() => stick && toBottom());
    ro.observe(content);
    return () => ro.disconnect();
  });
  onMount(() => {
    // Shown, the conversation stays in memory (read again if it had left it).
    const shown = conv.show();
    tick().then(toBottom);
    return shown;
  });

  /** Items that draw nothing: an empty block of text, the calls whose question or plan has its own card. */
  function shows(item: ConvItem) {
    if (item.kind === 'text' || item.kind === 'thinking') return !!item.text.trim() || item.streaming;
    // Its hand-back, in the Claude Code view, is the report below.
    if (item.kind === 'tool')
      return item.name !== 'AskUserQuestion' && item.name !== 'ExitPlanMode' && !(inTerminal && item.name === 'SubagentHandback');
    return item.kind === 'permission' || item.kind === 'question';
  }
</script>

<svelte:window onkeydowncapture={onWindowKey} />

<!-- `.conv`: the keys answering a waiting request apply here too (answersHere, in shortcuts.ts). -->
<main class="conv subview">
  <header class="head sv-head">
    <nav class="crumbs" aria-label={t('subagents.crumbs')}>
      <button class="crumb" use:tip={t('subagents.back', { name: agent.name })} onclick={() => app.closeSubagent(agent.id)}
        >{agent.name}</button
      >
      {#each chain as c, i (c.call)}
        <span class="sep" aria-hidden="true"><Icon name="right" size={12} /></span>
        {#if i < chain.length - 1}
          <button class="crumb" onclick={() => openSub(c.call)}>{c.title}</button>
        {:else}
          <span class="crumb here" aria-current="page" tabindex="-1" use:focusHere use:tip={{ text: c.title, cut: true }}>{c.title}</span>
        {/if}
      {/each}
    </nav>
    <div class="row">
      {#if info}
        <span class="st {info.status ?? 'unknown'}"><i aria-hidden="true"></i>{statusWord}</span>
      {/if}
      <EngineMark engine={agent.engine} full large />
      {#if stoppable}
        <!-- aria-disabled, not disabled: focusable and hovered still, for its tooltip to say why it cannot stop it yet. -->
        <button
          class="btn stop"
          aria-disabled={!canStop || stopping}
          use:tip={canStop ? t('subagents.stopHint') : t('subagents.stopLater')}
          onclick={stop}><Icon name="stop" size={14} />{t('subagents.stop')}</button
        >
      {/if}
    </div>
    {#if info}
      <dl class="facts facts-row" role="group" aria-label={info.title}>
        {#if info.kind}<div>
            <dt>{t('subagents.facts.kind')}</dt>
            <dd>{info.kind}</dd>
          </div>{/if}
        {#if info.model}<div>
            <dt>{t('subagents.facts.model')}</dt>
            <dd>{modelLabel(info.model, app.models)}</dd>
          </div>{/if}
        <div>
          <dt>{t('subagents.facts.tokens')}</dt>
          <dd>{info.tokens !== undefined ? fTok(info.tokens) : '—'}</dd>
        </div>
        {#if info.tools !== undefined}<div>
            <dt>{t('subagents.facts.tools')}</dt>
            <dd>{fInt(info.tools)}</dd>
          </div>{/if}
        <div>
          <dt>{t('subagents.facts.duration')}</dt>
          <dd>{duration !== null ? fDur(duration) : '—'}</dd>
        </div>
      </dl>
    {/if}
  </header>

  <div class="scroll" bind:this={scroller} onscroll={() => (stick = atBottom())}>
    <div class="msgs" data-gallery bind:this={content}>
      {#if conv.error}
        <div class="note err">{t('conv.loadError', { error: conv.error })}</div>
      {:else if !info}
        {#if conv.loaded}<div class="note">{t('subagents.gone')}</div>{/if}
      {:else}
        <section class="brief">
          {#if info.prompt}
            {@const long = info.prompt.length > LONG_PROMPT || info.prompt.split('\n').length > 6}
            <div class="prompt">
              <div class="k">
                {t('subagents.prompt')}
                {#if long}
                  <button class="more" aria-expanded={promptOpen} onclick={() => (promptOpen = !promptOpen)}
                    >{promptOpen ? t('subagents.promptLess') : t('subagents.promptMore')}</button
                  >
                {/if}
              </div>
              <div class="ptext" class:folded={long && !promptOpen}>{info.prompt}</div>
            </div>
          {/if}
        </section>

        {#if inTerminal}
          <div class="note">{t('subagents.terminal')}</div>
          {#if sideError}<div class="note err">{t('subagents.terminalError', { error: sideError })}</div>{/if}
        {/if}

        {#each shown as item (item.id)}
          {#if shows(item)}
            <div class="item" data-item={item.id}>
              {#if item.kind === 'text'}
                <div class="blocks"><Markdown text={item.text} streaming={item.streaming} /></div>
              {:else if item.kind === 'thinking'}
                <Thinking {item} />
              {:else if item.kind === 'tool'}
                <ToolRow
                  {item}
                  cwd={agent.cwd}
                  childrenOf={(id) => children.get(id) ?? []}
                  onOpenFile={openFile}
                  onOpenSub={inTerminal ? undefined : openSub}
                />
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
              {/if}
            </div>
          {/if}
        {:else}
          {#if inTerminal ? sideRead && !sideError : conv.loaded}<div class="note">{t('subagents.empty')}</div>{/if}
        {/each}

        {#if running}
          <div class="working">
            <span class="spark" aria-hidden="true"><Icon name="spark" size={14} /></span><span class="shimmer"
              >{t('subagents.working')}</span
            >
          </div>
        {:else if report}
          <section class="report" aria-label={t('subagents.report')}>
            <h3>{t('subagents.report')}</h3>
            <Markdown text={report} />
          </section>
        {:else if info.status}
          <div class="note">{t('subagents.noReport')}</div>
        {/if}
      {/if}
    </div>
  </div>

  {#if showWrite}
    <!-- Its keys are its own: Ctrl+Entrée sends the message, it answers no request waiting (answersHere). -->
    <form
      class="write"
      data-own-keys
      onsubmit={(e) => {
        e.preventDefault();
        send();
      }}
    >
      <label for="sub-write-{call}">{t('subagents.write', { name: agent.name })}</label>
      <div class="box">
        <textarea
          id="sub-write-{call}"
          rows="2"
          placeholder={t('subagents.writePlaceholder')}
          bind:value={() => message, setMessage}
          onkeydown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              send();
            }
          }}></textarea>
        <div class="wrow">
          <span class="hint">{canWrite ? t('subagents.writeHint', { name: agent.name }) : t('subagents.writeEnded')}</span>
          <button class="btn primary send" type="submit" disabled={!canWrite || !message.trim() || sending}
            >{t('subagents.writeSend', { name: agent.name })}<kbd aria-hidden="true"><Icon name="enter" size={12} /></kbd></button
          >
        </div>
      </div>
    </form>
  {/if}
</main>

<style>
  .conv {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container: conv / inline-size;
  }
  /* Its head (2.0 design): the breadcrumb, its state and harness with « Arrêter », its facts under them. */
  .sv-head {
    flex: none;
    display: grid;
    gap: 10px;
    padding: 14px 22px;
    border-bottom: 1px solid var(--line);
    min-width: 0;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 12.5px;
    white-space: nowrap;
  }
  .crumb {
    min-width: 0;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: none;
    font: inherit;
    color: var(--muted);
    cursor: pointer;
  }
  button.crumb:hover {
    color: var(--text);
  }
  button.crumb:focus-visible,
  .crumb.here:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .crumb.here {
    color: var(--text);
    font-weight: 600;
    cursor: default;
  }
  .sep {
    display: flex;
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .st {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px 0 7px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .st i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .st.running {
    background: oklch(from var(--run) l c h / 0.1);
    color: var(--run);
  }
  .st.failed {
    background: oklch(from var(--del) l c h / 0.1);
    color: var(--del);
  }
  .st.stopped,
  .st.interrupted {
    background: oklch(from var(--wait) l c h / 0.1);
    color: var(--wait);
  }
  .stop {
    margin-left: auto;
    flex: none;
    color: var(--del);
  }
  .stop :global(.icon) {
    color: var(--del);
  }
  .stop[aria-disabled='true'] {
    opacity: 0.5;
    cursor: default;
  }
  .facts-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 22px;
    margin: 0;
    font-size: 12px;
  }
  .facts-row div {
    display: grid;
    gap: 1px;
  }
  .facts-row dt {
    color: var(--muted);
    font-size: 11.5px;
  }
  .facts-row dd {
    margin: 0;
    color: var(--soft);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .scroll {
    flex: 1;
    min-height: 160px;
    overflow: auto;
    scrollbar-gutter: stable;
  }
  .msgs {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 18px;
    width: min(780px, 100% - 64px);
    margin: 0 auto;
    padding: 14px 0 24px;
  }
  .brief {
    display: grid;
    gap: 12px;
  }
  /* Its brief, folded to its first lines while long. */
  .prompt {
    display: grid;
    gap: 6px;
    padding: 11px 14px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .prompt .k {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .ptext {
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 420px;
    overflow: auto;
  }
  .ptext.folded {
    max-height: 3em;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, #000 60%, transparent);
  }
  .more {
    margin-left: auto;
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-weight: 500;
    color: var(--muted);
    cursor: pointer;
  }
  .more:hover {
    color: var(--text);
  }
  .item {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .blocks {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .note {
    padding: 10px 14px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 12.5px;
  }
  .note.err {
    background: oklch(from var(--del) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
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
  /* What it handed back: a card with a hairline in the green of what is done. */
  .report {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px oklch(from var(--add) l c h / 0.35);
  }
  .report h3 {
    margin: 0;
    color: var(--add);
    font-size: 12px;
    font-weight: 600;
  }
  /* A message to it, through the agent: said above a composer like the conversation's. */
  .write {
    display: grid;
    gap: 6px;
    width: min(780px, 100% - 64px);
    margin: 0 auto 18px;
  }
  .write label {
    color: var(--muted);
    font-size: 11.5px;
  }
  .write .box {
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
  }
  .write .box:focus-within {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  .write textarea {
    display: block;
    width: 100%;
    min-height: 52px;
    padding: 14px 16px 6px;
    border: 0;
    outline: 0;
    resize: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 13.5px;
  }
  .write textarea::placeholder {
    color: var(--muted);
  }
  .wrow {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 8px 8px 16px;
  }
  .hint {
    flex: 1;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.4;
  }
  .send {
    height: 32px;
    border-radius: 9px;
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
  }
</style>

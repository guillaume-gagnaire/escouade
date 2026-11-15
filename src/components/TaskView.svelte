<script lang="ts">
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { fDur, fInt, fTok } from '../lib/format';
  import Icon from './Icon.svelte';
  import {
    bgDuration,
    bgRow,
    cleanOutput,
    exitCode,
    hasOutput,
    isRunning,
    kindLabel,
    ownerOf,
    rowTitle,
    statusLabel,
  } from '../lib/bgtasks';
  import { menu } from '../lib/menu.svelte';
  import { app } from '../lib/state.svelte';
  import { inVscode } from '../lib/shortcuts';
  import { tip } from '../lib/tip';
  import type { Agent, TaskOutput } from '../lib/types';

  // The view of a task Claude Code runs in the background for `agent` (a command, a monitor, a workflow, a remote agent,
  // a teammate), in place of the agent's conversation (ESC-14): `task` is Claude Code's id of the task. Its head says what
  // it is and how it is going, and « Arrêter » stops it (Claude Code's `stop_task`); the end of what a command or a monitor
  // writes is read from the backend as it comes, and only read. The facts are the plan's (`agent.plan`), which the backend
  // keeps current: a task that is no longer in it (a new session) closes the view (state.svelte.ts).
  let { agent, task }: { agent: Agent; task: string } = $props();

  /** A command longer than that is folded to its first lines. */
  const LONG_COMMAND = 360;
  const LONG_LINES = 6;
  /** How often the output is read while the task runs. */
  const POLL_MS = 1_500;
  /** Within that many pixels of the end, the log is read at its end. */
  const AT_END = 24;

  const plan = $derived(agent.plan);
  const row = $derived(bgRow(plan, task));
  const title = $derived(row ? rowTitle(row) : task);
  const running = $derived(!!row && isRunning(row));
  /** In the Claude Code view, the task is acted on in Claude Code's own interface. */
  const inTerminal = $derived(agent.terminal);
  const stoppable = $derived(running && !inTerminal);
  const duration = $derived(row ? bgDuration(row, app.now) : null);
  const code = $derived(row && !running ? exitCode(row) : null);
  const owner = $derived(row ? ownerOf(plan, row) : undefined);
  /** A command and a monitor write to an output that is followed; the other kinds write to none of their own. */
  const followed = $derived(!!row && hasOutput(row.kind));
  const command = $derived(row?.command?.trim() ? row.command.trim() : undefined);
  const description = $derived(!command && row?.title.trim() ? row.title.trim() : undefined);
  const longCommand = $derived(!!command && (command.length > LONG_COMMAND || command.split('\n').length > LONG_LINES));
  /** What Claude Code said of it once it ended; for a task stopped it is only its description, which is said already. */
  const summary = $derived(
    row && !running && row.summary?.trim() && row.summary.trim() !== row.title.trim() ? row.summary.trim() : undefined,
  );

  let commandOpen = $state(false);
  let stopping = $state(false);
  // The log follows what comes while it is read at its end, not once the reader went up in it.
  let log = $state<HTMLPreElement>();
  let stick = $state(true);

  async function stop() {
    if (!row || !stoppable || stopping) return;
    stopping = true;
    await app.run(api.stopBgTask(agent.id, row.id));
    stopping = false;
  }

  // Its output: the end of it (8 KiB at most while the task runs, 256 KiB once it ended), read when the view opens, then
  // every 1.5 s while the task runs and the window is seen, and once more when the task ends (for what it wrote last): a
  // task that ended is read once, with no timer, so the longer end of it is cleaned (`text`) once per read. Each read gives
  // the whole of the end and REPLACES what is shown, nothing is added to it. One read at a time: a tick during a slow read
  // waits for the next; the last read, asked by the end of the task, waits for the one under way and goes after it. A read
  // that fails is told quietly and the next one tries again. Not state: only `read` looks at them.
  interface Reader {
    agentId: string;
    task: string;
    /** The view or the task it reads for is gone: its answer is dropped. */
    gone: boolean;
  }
  let reader: Reader | null = null;
  let inflight = false;
  let queued = false;
  let readKey = '';
  let got = $state.raw<TaskOutput | null>(null);
  let readError = $state<string | null>(null);
  let loaded = $state(false);

  async function read(last: boolean) {
    const r = reader;
    if (!r || r.gone) return;
    if (inflight) {
      if (last) queued = true;
      return;
    }
    inflight = true;
    try {
      const answer = await api.bgTaskOutput(r.agentId, r.task);
      if (!r.gone) {
        got = answer;
        readError = null;
      }
    } catch (e) {
      if (!r.gone) readError = String(e);
    } finally {
      inflight = false;
      if (!r.gone) loaded = true;
      if (queued) {
        queued = false;
        void read(true);
      }
    }
  }

  $effect(() => {
    if (!followed) return;
    const r: Reader = { agentId: agent.id, task, gone: false };
    reader = r;
    // Another task: nothing of the last one stays.
    const key = `${r.agentId}:${r.task}`;
    if (key !== readKey) {
      readKey = key;
      got = null;
      readError = null;
      loaded = false;
      stick = true;
    }
    const seen = () => document.visibilityState !== 'hidden';
    // Run at the start, and again as the task ends (the effect runs anew when `running` changes).
    void read(true);
    let timer: ReturnType<typeof setInterval> | undefined;
    let onSeen: (() => void) | undefined;
    if (running) {
      timer = setInterval(() => seen() && void read(false), POLL_MS);
      onSeen = () => seen() && void read(false);
      document.addEventListener('visibilitychange', onSeen);
    }
    return () => {
      r.gone = true;
      clearInterval(timer);
      if (onSeen) document.removeEventListener('visibilitychange', onSeen);
    };
  });

  const text = $derived(got ? cleanOutput(got.output) : '');
  /** What the empty log says, if anything. */
  const placeholder = $derived.by(() => {
    if (text) return null;
    if (!loaded) return t('bgtasks.reading');
    if (!got) return null;
    if (got.source === 'none') return t('bgtasks.noOutputFile');
    return running ? t('bgtasks.emptyRunning') : t('bgtasks.emptyDone');
  });

  // Where the reader is, measured before the new output is drawn: one who went up in it just before (whose scroll event has
  // not come yet) is not brought back to the end by it.
  let atEndBefore = true;
  $effect.pre(() => {
    void text;
    atEndBefore = !log || log.scrollHeight - log.scrollTop - log.clientHeight < AT_END;
  });
  $effect(() => {
    void text;
    if (log && stick && atEndBefore) log.scrollTop = log.scrollHeight;
  });
  /** « Suivre la fin »: to the end now, and following it from there. */
  function followEnd() {
    stick = true;
    if (log) log.scrollTop = log.scrollHeight;
  }
  function onScroll() {
    if (log) stick = log.scrollHeight - log.scrollTop - log.clientHeight < AT_END;
  }

  // Escape goes back to the agent's conversation. Seen before the others (capture): a dialog or a menu open closes first,
  // alone; a field with text in it keeps its Escape (a draft is not lost to a key). The view has no field of its own: it is
  // one elsewhere in the window (the composer of the agent) that has the focus.
  function onWindowKey(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing || app.modal || menu.open || inVscode(e.target)) return;
    const typed = e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLInputElement ? e.target.value.trim() : '';
    if (typed) return;
    app.closeTask(agent.id);
  }

  /** Where the focus goes when the view opens: the crumb it is at, so that the keyboard is in the view (and Escape heard). */
  function focusHere(node: HTMLElement) {
    node.focus({ preventScroll: true });
  }
</script>

<svelte:window onkeydowncapture={onWindowKey} />

<main class="taskview" data-task-view data-task={task} data-kind={row?.kind} data-status={row?.status}>
  <header class="head sv-head">
    <nav class="crumbs" aria-label={t('bgtasks.crumbs')}>
      <button class="crumb" use:tip={t('bgtasks.back', { name: agent.name })} onclick={() => app.closeTask(agent.id)}>{agent.name}</button>
      <span class="sep" aria-hidden="true"><Icon name="right" size={12} /></span>
      <span class="crumb here" aria-current="page" tabindex="-1" use:focusHere use:tip={{ text: title, cut: true }}>{title}</span>
    </nav>
    <div class="row">
      {#if row}
        <span class="st {row.status}" data-task-state><i aria-hidden="true"></i>{statusLabel(row.status)}</span>
      {/if}
      {#if stoppable}
        <button class="btn stop" data-task-stop disabled={stopping} use:tip={t('bgtasks.stopHint')} onclick={stop}
          ><Icon name="stop" size={14} />{t('bgtasks.stop')}</button
        >
      {:else if running && inTerminal}
        <span class="where">{t('bgtasks.stopInTerminal')}</span>
      {/if}
    </div>
    {#if row}
      <dl class="facts facts-row" role="group" aria-label={title}>
        <div>
          <dt>{t('bgtasks.facts.kind')}</dt>
          <dd>{kindLabel(row.kind)}</dd>
        </div>
        <div>
          <dt>{t('bgtasks.facts.duration')}</dt>
          <dd data-task-duration>{duration !== null ? fDur(duration) : '—'}</dd>
        </div>
        {#if code !== null}
          <div>
            <dt>{t('bgtasks.facts.exit')}</dt>
            <dd data-task-exit>{code}</dd>
          </div>
        {/if}
        {#if owner}
          <div>
            <dt>{t('bgtasks.facts.by')}</dt>
            <dd>{owner.title}</dd>
          </div>
        {/if}
        {#if row.kind === 'workflow'}
          {#if row.now}
            <div>
              <dt>{t('bgtasks.facts.phase')}</dt>
              <dd>{row.now}</dd>
            </div>
          {/if}
          <div>
            <dt>{t('bgtasks.facts.tools')}</dt>
            <dd>{fInt(row.tools ?? 0)}</dd>
          </div>
          <div>
            <dt>{t('bgtasks.facts.tokens')}</dt>
            <dd>{fTok(row.tokens ?? 0)}</dd>
          </div>
        {/if}
      </dl>
    {/if}
  </header>

  <div class="page">
    {#if !row}
      <div class="note">{t('bgtasks.gone')}</div>
    {:else}
      <section class="brief">
        {#if command}
          <div class="block">
            <div class="k">{t('bgtasks.command')}</div>
            <div class="cmd mono" class:folded={longCommand && !commandOpen} data-task-command>{command}</div>
            {#if longCommand}
              <button class="more" aria-expanded={commandOpen} onclick={() => (commandOpen = !commandOpen)}
                >{commandOpen ? t('bgtasks.commandLess') : t('bgtasks.commandMore')}</button
              >
            {/if}
          </div>
        {:else if description}
          <div class="block">
            <div class="k">{t('bgtasks.description')}</div>
            <div class="cmd" data-task-description>{description}</div>
          </div>
        {/if}
      </section>

      {#if summary}
        <section class="summary" class:bad={row.status === 'failed'} aria-label={t('bgtasks.summary')} data-task-summary>
          <h3>{t('bgtasks.summary')}</h3>
          <p>{summary}</p>
        </section>
      {/if}

      <section class="outsec" aria-labelledby="task-out-{task}">
        <div class="outhead">
          <h3 class="k" id="task-out-{task}">{t('bgtasks.output')}</h3>
          {#if followed && got}
            {#if got.truncated}<span class="meta" data-task-truncated>{t('bgtasks.truncated')}</span>{/if}
            {#if got.source === 'file'}<span class="meta" data-task-source="file">{t('bgtasks.fromFile')}</span>{/if}
          {/if}
          {#if readError}
            <span class="meta err" data-task-read-error
              >{t('bgtasks.readError', { error: readError })}{running ? ` ${t('bgtasks.readRetry')}` : ''}</span
            >
          {/if}
        </div>
        {#if followed}
          <div class="logwrap">
            <!-- Focusable, as every scrolled region: read from the keyboard too. Not read aloud as it comes (a build writes
                 hundreds of lines). -->
            <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
            <pre
              class="log"
              role="log"
              aria-live="off"
              aria-label={t('bgtasks.outputLabel', { name: title })}
              tabindex="0"
              data-task-output
              bind:this={log}
              onscroll={onScroll}>{text}</pre>
            {#if placeholder}<p class="ph">{placeholder}</p>{/if}
            {#if !stick}
              <button class="follow" data-task-follow onclick={followEnd}>{t('bgtasks.follow')}</button>
            {/if}
          </div>
        {:else}
          <div class="note">{t('bgtasks.noOutputType')}</div>
        {/if}
      </section>
    {/if}
  </div>
</main>

<style>
  .taskview {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* Its head (2.0 design): the breadcrumb, its state with « Arrêter », its facts under them. */
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
    max-width: 320px;
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
  .where {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
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
    min-width: 0;
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
    overflow-wrap: anywhere;
  }
  .page {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 14px 22px 22px;
  }
  .brief {
    flex: none;
    display: grid;
    gap: 12px;
  }
  .k {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .block {
    display: grid;
    gap: 6px;
    padding: 11px 14px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .cmd {
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 420px;
    overflow: auto;
  }
  .cmd.folded {
    max-height: 4.5em;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, #000 60%, transparent);
  }
  .more {
    justify-self: start;
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
  }
  .more:hover {
    color: var(--text);
  }
  .summary {
    flex: none;
    display: grid;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px oklch(from var(--add) l c h / 0.35);
  }
  .summary.bad {
    background: oklch(from var(--del) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .summary h3 {
    margin: 0;
    color: var(--add);
    font-size: 12px;
    font-weight: 600;
  }
  .summary.bad h3 {
    color: var(--del);
  }
  .summary p {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  /* Its output, followed: a terminal's well. */
  .outsec {
    flex: 1 1 260px;
    min-height: 200px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .outhead {
    flex: none;
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 4px 12px;
  }
  .outhead .k {
    color: var(--soft);
    font-size: 12px;
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
  }
  .meta.err {
    color: var(--del);
  }
  .logwrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .log {
    flex: 1;
    min-width: 0;
    margin: 0;
    padding: 12px 14px;
    overflow: auto;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .log:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .ph {
    position: absolute;
    inset: 0;
    margin: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 20px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
    pointer-events: none;
  }
  .follow {
    position: absolute;
    right: 14px;
    bottom: 12px;
    height: 28px;
    padding: 0 11px;
    border: none;
    border-radius: 8px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    color: var(--text);
    font: inherit;
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
  }
  .follow:hover {
    background: var(--raise);
  }
  .follow:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .note {
    flex: none;
    padding: 10px 14px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12.5px;
    color: var(--muted);
  }
</style>

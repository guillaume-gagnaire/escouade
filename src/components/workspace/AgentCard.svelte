<script lang="ts">
  // The card of an agent near, at the centre of its compass (spec 2026-11-25-vue-workspace-design.md, « Au centre ») :
  // the sidebar's card (its dot, name and state, model · duration and heat, what it does and its spend), and a bar to
  // write to it. Enter sends by the conversation's path
  // (`submitDraft`), from the agent's own draft: what is typed here is in its conversation once the canvas is left.
  // While it waits, the card grows with its requests (« La question ») ; a subagent's are on the subagent's card.
  import { untrack } from 'svelte';
  import { conversationOf } from '../../lib/conversations.svelte';
  import { effortChoicesOf, modeChoicesOf, modelChoicesOf, setAgentOption, type OptionChoice } from '../../lib/agent-options';
  import { insertAt } from '../../lib/dictation';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { getDraft, setDraft } from '../../lib/drafts';
  import { refreshModels, searchesModels } from '../../lib/engines';
  import { fDur } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { interactiveOf } from '../../lib/interactive';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { modelLabel } from '../../lib/models';
  import { observeHeight } from '../../lib/resize';
  import { HOLD_MS } from '../../lib/answer-keys.svelte';
  import { submitDraft } from '../../lib/send.svelte';
  import { tip } from '../../lib/tip';
  import { cardActivity, cardSpend } from '../../lib/sidebar';
  import { app } from '../../lib/state.svelte';
  import type { Agent } from '../../lib/types';
  import { askMaxH, CARD, subName, waits } from '../../lib/workspace-layout';
  import AskInset from './AskInset.svelte';
  import EngineMark from '../EngineMark.svelte';
  import HeatFigure from '../HeatFigure.svelte';
  import Icon from '../Icon.svelte';
  import MicButton from '../MicButton.svelte';
  import StatusDot from '../StatusDot.svelte';

  // `shown`: the subagents that have a card on the canvas; a request of another is shown here.
  // `zoom`: the camera's, which the 70 % cap of the card is on screen at.
  let { agent, onsize, shown = [], zoom = 1 }: { agent: Agent; onsize?: (h: number) => void; shown?: string[]; zoom?: number } = $props();

  const agentId = untrack(() => agent.id);
  let text = $state(getDraft(agentId).text);
  let files = $state(getDraft(agentId).files.length);
  let field = $state<HTMLInputElement>();
  let wrap = $state<HTMLElement>();

  // The sidebar's words for its state (« En cours », « Question »…).
  const stateWord = $derived(t(`nav.sidebar.status.${agent.status}`));
  const duration = $derived(fDur(agent.activeMs + (agent.activeSince ? app.now - agent.activeSince : 0)));
  const act = $derived(cardActivity(agent, 0));

  // Its conversation is loaded while it waits: what is typed answers the request there, as in the conversation.
  const waiting = $derived(waits(agent));
  $effect(() => {
    if (waiting) return untrack(() => conversationOf(agentId).show());
  });
  // Read only while it waits (lazy), and apart: a conversation made inside a derived would not be tracked by it.
  const conv = $derived(conversationOf(agentId));
  /** A request is this card's: the agent's own, or one of a subagent with no card of its own (folded, dropped). */
  const mine = (sub: string | undefined) => !sub || !shown.includes(sub);
  /** The request what is typed here answers: the first this card shows; one shown on another card is not answered here. */
  const answerable = $derived(
    agent.pending.length
      ? agent.pending
          .map((id) => conv.items.find((i) => i.id === id))
          .find((i) => (i?.kind === 'question' || i?.kind === 'permission') && mine(i.sub ?? undefined))
      : undefined,
  );
  // Enter waits HOLD_MS once a request shows, as in the conversation: pressed as it comes, it would answer it unread.
  let shownAt = 0;
  $effect(() => {
    if (answerable?.id) shownAt = performance.now();
  });
  // The subagents asking that have a card, by name: their requests are on their own cards.
  const asking = $derived.by(() => {
    if (!agent.pending.length) return [];
    const items = conv.items;
    const subs = agent.pending.map((id) => (items.find((i) => i.id === id) as { sub?: string } | undefined)?.sub);
    return [...new Set(subs.filter((s): s is string => !!s && !mine(s)))].map((s) => subName(agent, items, s));
  });
  let innerHeight = $state(window.innerHeight);
  // shortcut: the card's own lines counted as its default height, a line of progress more lets it pass 70 % by
  // that line; measure them if it shows.
  const askH = $derived(askMaxH(innerHeight, zoom, CARD.h));

  // A message given back to the draft (refused), or the draft changed by the conversation: read again.
  $effect(() => {
    if (!app.draftReturned[agentId]) return;
    untrack(read);
  });

  function read() {
    const d = getDraft(agentId);
    text = d.text;
    files = d.files.length;
  }

  function write(value: string) {
    text = value;
    setDraft(agentId, { text, files: getDraft(agentId).files });
  }

  const dictated: DictationTarget = {
    kind: 'composer',
    id: `card-${Math.random().toString(36).slice(2)}`,
    el: () => wrap,
    insert: (said) => {
      const r = insertAt(text, field?.selectionStart ?? text.length, field?.selectionEnd ?? text.length, said);
      write(r.text);
      queueMicrotask(() => {
        field?.focus();
        field?.setSelectionRange(r.caret, r.caret);
      });
    },
  };

  function send() {
    const body = text.trim();
    const d = getDraft(agentId);
    if (!body && !d.files.length) return;
    // An interactive command of its harness never goes as a message: its conversation opens, the text kept in the draft.
    if (interactiveOf(agent.engine, body)) return app.selectAgent(agentId);
    // Its requests not read yet: the text would leave as a new message instead of answering one.
    if (agent.pending.length && !conv.loaded) return;
    // A file still being copied has no path yet: it would be dropped on the way, as the composer waits for it.
    if (d.files.some((f) => f.copying)) return;
    const s = submitDraft(agent, agentId, text, d.files, answerable ?? null);
    if (s.cleared === 'none') return;
    setDraft(agentId, { text: '', files: s.cleared === 'all' ? [] : d.files });
    read();
    // Refused for want of an account: the conversation says it, which the canvas does not show.
    void s.done.then(() => {
      const missing = conversationOf(agentId).noAccount;
      if (missing) app.toast(missing.text, 'error');
    });
  }

  function key(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    if (answerable && performance.now() - shownAt < HOLD_MS) return;
    send();
  }

  const entries = (choices: OptionChoice[], current: string, set: (v: string) => void): MenuItem[] =>
    choices.map((c) => ({
      label: c.label,
      detail: c.detail,
      title: c.title,
      disabled: c.disabled,
      reason: c.reason,
      checked: c.value === current,
      onClick: () => set(c.value),
    }));

  function settings(e: MouseEvent) {
    refreshModels(agent);
    const many = searchesModels(agent);
    // A menu does not search: a long list shows the favorites and the agent's own, the rest in the conversation.
    const models = modelChoicesOf(agent).filter((c) => !many || c.favorite || c.value === agent.model);
    const efforts = effortChoicesOf(agent);
    const items: MenuItem[] = [
      {
        label: t('workspace.card.model'),
        submenu: [
          ...entries(models, agent.model, (model) => setAgentOption(agentId, { model })),
          ...(many
            ? [
                { separator: true, label: '' },
                { label: t('workspace.card.allModels'), onClick: () => app.selectAgent(agentId) },
              ]
            : []),
        ],
      },
      {
        label: t('workspace.card.effort'),
        disabled: !efforts.length,
        submenu: efforts.length ? entries(efforts, agent.effort, (effort) => setAgentOption(agentId, { effort })) : undefined,
      },
      { label: t('workspace.card.mode'), submenu: entries(modeChoicesOf(agent), agent.mode, (mode) => setAgentOption(agentId, { mode })) },
      { separator: true, label: '' },
      // The clip and the copy of files are the conversation's: the card only counts them.
      { label: t('workspace.card.attach'), icon: 'clip', onClick: () => app.selectAgent(agentId) },
    ];
    menu.showAt(e.currentTarget as HTMLElement, items);
  }

  function open(e: MouseEvent) {
    if ((e.target as HTMLElement).closest('input, button')) return;
    app.selectAgent(agentId);
  }
  /** Enter on the card itself opens its conversation, as a double click. */
  function enter(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.target !== e.currentTarget) return;
    e.preventDefault();
    app.selectAgent(agentId);
  }
</script>

<svelte:window bind:innerHeight />

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<article
  class="agent {agent.status}"
  data-side="centre"
  tabindex="0"
  onkeydown={enter}
  aria-label={t('workspace.card.label', { name: agent.name, state: stateWord })}
  class:sub-asks={asking.length > 0}
  style:min-height="{CARD.h}px"
  ondblclick={open}
  bind:this={wrap}
  use:observeHeight={(h) => onsize?.(h)}
>
  <div class="a-top">
    <StatusDot status={agent.status} />
    <span class="name">{agent.name}</span>
    <span class="a-state">{stateWord}</span>
  </div>
  {#if agent.progressLine}<div class="said">{agent.progressLine}</div>{/if}
  <div class="a-meta">
    <EngineMark engine={agent.engine} />
    <span class="model">{modelLabel(agent.model, app.models)}</span>
    <span class="sep">·</span><span class="num">{duration}</span>
    <HeatFigure {agent} />
  </div>
  <div class="a-act">
    {#if agent.status === 'waiting'}
      <span class="tgt">{t('workspace.waiting')}</span>
    {:else if act?.kind === 'doing'}
      <b>{act.verb}</b>{#if act.what}<span class="tgt mono">{act.what}</span>{/if}
    {:else if act?.kind === 'thinking'}
      <span class="tgt">{t('stats.overview.thinking')}</span>
    {:else if act?.kind === 'over'}
      <span class="ok"><Icon name="check" size={14} /></span><span class="tgt">{t('nav.sidebar.over')} · {act.goes}</span>
    {/if}
    <span class="spend">{cardSpend(agent)}</span>
  </div>
  {#if asking.length}<p class="from">{t('workspace.ask.fromSub', { name: asking.join(', ') })}</p>{/if}
  <AskInset {agent} {mine} label maxH={askH} onfocusback={() => field?.focus()} />
  <div class="bar">
    <input
      bind:this={field}
      value={text}
      aria-label={t('workspace.card.write', { name: agent.name })}
      placeholder={t('workspace.card.write', { name: agent.name })}
      oninput={(e) => write(e.currentTarget.value)}
      onkeydown={key}
    />
    {#if files}<span class="files">{t('workspace.card.files', { count: files })}</span>{/if}
    <button type="button" class="icon" aria-label={t('workspace.card.settings')} use:tip={t('workspace.card.settings')} onclick={settings}>
      <Icon name="sliders" size={14} />
    </button>
    <MicButton target={dictated} panel={false} />
    <button type="button" class="btn primary send" disabled={!text.trim() && !files} onclick={send}>{t('composer.send.label')}</button>
  </div>
</article>

<style>
  /* The sidebar's card (Sidebar.svelte, `.agent`), as it is when selected: on the canvas it stands alone. */
  .agent {
    --tone: var(--run);
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: 3px;
    box-sizing: border-box;
    width: 100%;
    padding: 8px 10px 9px;
    border-radius: 9px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 13px;
  }
  .agent.waiting {
    --tone: var(--wait);
    background: color-mix(in oklch, var(--wait-bg), var(--panel));
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  /* A subagent asks: the card says it, in the tone of a question. */
  .agent.sub-asks {
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .from {
    margin: 0;
    padding-left: 16px;
    color: var(--wait);
    font-size: 12px;
    font-weight: 550;
  }
  .agent.done,
  .agent.idle {
    --tone: var(--muted);
  }
  .agent.error {
    --tone: var(--del);
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
  .a-meta .model {
    flex: none;
    max-width: 45%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .said {
    display: block;
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
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
  /* The composer's field, in one line. */
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-top: 8px;
    padding: 4px 4px 4px 10px;
    border-radius: var(--r-sm);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .bar:focus-within {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  .agent.waiting .bar {
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  input {
    flex: 1;
    min-width: 0;
    padding: 4px 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }
  .files {
    flex: none;
    padding: 1px 6px;
    border-radius: 99px;
    background: var(--hover);
    color: var(--soft);
    font-size: 10.5px;
  }
  .icon {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .icon:hover {
    background: var(--hover);
    color: var(--text);
  }
  .send {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
</style>

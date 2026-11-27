<script lang="ts">
  // One of the team of an agent near, east of its card (spec 2026-11-25-vue-workspace-design.md, « À l'est ») : a
  // subagent, the agent's card in smaller (a field to write to it while it runs in the background), a background task in
  // dots, or the pile of the subagents that ended. A subagent that asks grows with its request (« La question »).
  import { bgDuration, kindLabel, rowTitle } from '../../lib/bgtasks';
  import { conversationOf } from '../../lib/conversations.svelte';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { insertAt } from '../../lib/dictation';
  import { fDur, fTok } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import { api } from '../../lib/ipc';
  import { modelLabel } from '../../lib/models';
  import { observeHeight } from '../../lib/resize';
  import { app } from '../../lib/state.svelte';
  import { canWriteSub, subDuration } from '../../lib/subagents';
  import type { Agent } from '../../lib/types';
  import { askMaxH, SUB, toolName, type TeamItem } from '../../lib/workspace-layout';
  import AskInset from './AskInset.svelte';
  import MicButton from '../MicButton.svelte';
  import Icon from '../Icon.svelte';
  import StatusDot from '../StatusDot.svelte';

  let {
    agent,
    item,
    ontoggle,
    onsize,
    zoom = 1,
  }: { agent: Agent; item: TeamItem; ontoggle: () => void; onsize?: (id: string, h: number) => void; zoom?: number } = $props();

  let field = $state<HTMLInputElement>();
  let wrap = $state<HTMLElement>();

  const sub = $derived(item.kind === 'sub' ? item.sub : null);
  const name = $derived(sub ? (sub.kind ?? sub.title) : '');
  // The rank of its task in the plan, when it is one of the plan's.
  const task = $derived.by(() => {
    const i = sub?.planTask ? (agent.plan?.tasks.findIndex((x) => x.id === sub.planTask) ?? -1) : -1;
    return i >= 0 ? i + 1 : null;
  });
  // Its dot is an agent's: green while it runs, red failed, grey ended.
  const dot = $derived(sub?.status === 'running' ? 'running' : sub?.status === 'failed' ? 'error' : 'done');
  const canWrite = $derived(!!sub && canWriteSub(agent, sub));
  // Its skills and plugins: their cards are above the agent's, a tag of each stays on its own.
  const kits = $derived(sub ? (agent.canvas?.subs.find((s) => s.call === sub.id)?.tools ?? []) : []);
  // One of its agent's requests is its own (the agent's card keeps the conversation loaded while it waits).
  // Read only while it waits (lazy), and apart: a conversation made inside a derived would not be tracked by it.
  const conv = $derived(conversationOf(agent.id));
  const asks = $derived.by(() => {
    if (!sub || !agent.pending.length) return false;
    const items = conv.items;
    return agent.pending.some((id) => (items.find((i) => i.id === id) as { sub?: string } | undefined)?.sub === sub.id);
  });
  let innerHeight = $state(window.innerHeight);
  // What is typed to it is the app's, as in its view: it outlives the card and a change of altitude.
  const draftKey = $derived(sub ? `${agent.id}:${sub.id}` : '');
  const message = $derived(app.subDrafts[draftKey] ?? '');
  const setMessage = (v: string) => {
    if (v) app.subDrafts[draftKey] = v;
    else delete app.subDrafts[draftKey];
  };

  const dictated: DictationTarget = {
    kind: 'composer',
    id: `team-${Math.random().toString(36).slice(2)}`,
    el: () => wrap,
    insert: (said) => {
      const r = insertAt(message, field?.selectionStart ?? message.length, field?.selectionEnd ?? message.length, said);
      setMessage(r.text);
      queueMicrotask(() => {
        field?.focus();
        field?.setSelectionRange(r.caret, r.caret);
      });
    },
  };

  // The request answered: its field takes the focus back, else the card itself (focusable by script alone).
  function focusBack() {
    if (field) return field.focus();
    wrap?.focus();
  }

  async function send() {
    const text = message.trim();
    if (!text || !sub || !canWrite) return;
    const sent = await app.run(api.messageSubagent(agent.id, sub.id, text).then(() => true));
    if (sent) {
      setMessage('');
      app.toast(t('subagents.writeSent', { name: agent.name }));
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    void send();
  }

  function open(e: MouseEvent) {
    if (!sub || (e.target as HTMLElement).closest('input, button')) return;
    app.openSubagent(agent.id, sub.id);
  }
  /** Enter on the card itself does what a double click does: its conversation, or its task's output. */
  function enter(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.target !== e.currentTarget) return;
    e.preventDefault();
    if (sub) app.openSubagent(agent.id, sub.id);
    else if (item.kind === 'bg') app.openTask(agent.id, item.row.id);
  }
</script>

<svelte:window bind:innerHeight />

{#if item.kind === 'folded'}
  <button type="button" class="folded" data-side="east" onclick={ontoggle}>
    {[
      item.capped ? t('workspace.team.foldedCapped') : t('workspace.team.folded', { count: item.done }),
      item.failed ? t('workspace.team.failed', { count: item.failed }) : '',
    ]
      .filter(Boolean)
      .join(' · ')}
  </button>
{:else if item.kind === 'bg'}
  {@const ms = bgDuration(item.row, app.now)}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <article
    class="card bg {item.row.status}"
    data-side="east"
    tabindex="0"
    onkeydown={enter}
    aria-label={t('workspace.card.label', { name: kindLabel(item.row.kind), state: t(`bgtasks.status.${item.row.status}`) })}
  >
    <header>
      <StatusDot status={item.row.status === 'running' ? 'running' : 'done'} />
      <b class="name">{kindLabel(item.row.kind)}</b>{#if ms !== null}<span class="st num">{fDur(ms)}</span>{/if}
    </header>
    <p class="cmd mono">{rowTitle(item.row)}</p>
  </article>
{:else if sub}
  {@const ms = subDuration(sub, app.now)}
  {@const word = t(`subagents.status.${sub.status ?? 'unknown'}`)}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <article
    class="card sub {sub.status}"
    class:asks
    data-side="east"
    tabindex="0"
    onkeydown={enter}
    aria-label={t('workspace.card.label', { name, state: word })}
    style:min-height="{SUB.h}px"
    ondblclick={open}
    bind:this={wrap}
    use:observeHeight={(h) => onsize?.(item.id, h)}
  >
    <header>
      <StatusDot status={dot} />
      <b class="name">{name}</b>
      {#if task}<span class="task">{t('workspace.team.task', { n: task })}</span>{/if}
      <span class="st">{word}</span>
    </header>
    <p class="meta">
      {[sub.model ? modelLabel(sub.model, app.models) : '', ms !== null ? fDur(ms) : '', sub.tokens ? fTok(sub.tokens) : '']
        .filter(Boolean)
        .join(' · ')}
    </p>
    {#if sub.doing}<p class="doing mono">{sub.doing}</p>{/if}
    {#if kits.length}
      <div class="kits" role="group" aria-label={t('workspace.tool.tags', { name })}>
        {#each kits as k (`${k.kind}:${k.name}`)}<span class="kit {k.kind}" class:running={k.running > 0} use:tip={k.name}
            ><Icon name={k.kind === 'skill' ? 'scroll' : 'box'} size={11} />{toolName(k)}{#if k.running > 0}<span class="sr"
                >, {t('workspace.tool.running')}</span
              >{/if}</span
          >{/each}
      </div>
    {/if}
    {#if asks}<AskInset {agent} mine={(s) => s === sub.id} onfocusback={focusBack} maxH={askMaxH(innerHeight, zoom, SUB.h)} />{/if}
    {#if canWrite}
      <div class="bar">
        <input
          bind:this={field}
          value={message}
          aria-label={t('workspace.team.write', { name })}
          placeholder={t('workspace.team.write', { name })}
          oninput={(e) => setMessage(e.currentTarget.value)}
          onkeydown={key}
        />
        <MicButton target={dictated} panel={false} />
      </div>
    {/if}
  </article>
{/if}

<style>
  /* The agent's card in smaller: the sidebar's card on the canvas. */
  .card {
    --tone: var(--muted);
    display: flex;
    flex-direction: column;
    gap: 3px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 8px 10px 9px;
    border-radius: 9px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12.5px;
  }
  .card.running {
    --tone: var(--run);
  }
  /* Its height is its own: it grows with a request, and its agent places the team below by it. */
  .card.sub {
    height: auto;
  }
  .card.asks {
    --tone: var(--wait);
    background: color-mix(in oklch, var(--wait-bg), var(--panel));
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .card.failed {
    --tone: var(--del);
  }
  /* A background task: a dotted edge. */
  .card.bg {
    box-shadow: none;
    outline: 1px dashed var(--line-3);
    outline-offset: -1px;
  }
  header {
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
  .task {
    flex: none;
    padding: 0 6px;
    border-radius: 99px;
    background: var(--hover);
    color: var(--soft);
    font-size: 11px;
  }
  .st {
    flex: none;
    color: var(--tone);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  p {
    margin: 0;
    overflow: hidden;
    padding-left: 16px;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .doing,
  .cmd {
    color: var(--soft);
    font-size: 11px;
  }
  /* Its tools: small tags, squared as their cards, coloured by kind. */
  /* As many rows as they need: the card's height is measured, the team below moves. */
  .kits {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding-left: 16px;
  }
  .kit {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 3px;
    padding: 0 5px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 10.5px;
  }
  .kit.skill :global(.icon) {
    color: var(--skill);
  }
  .kit.plugin :global(.icon) {
    color: var(--info);
  }
  .kit.running {
    box-shadow: inset 0 0 0 1px var(--run);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-top: auto;
    padding: 2px 2px 2px 8px;
    border-radius: var(--r-sm);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .bar:focus-within {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent),
      0 0 0 3px color-mix(in oklch, var(--p) 14%, transparent);
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 12px;
    outline: none;
  }
  /* The pile: a pill of the app. */
  .folded {
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 999px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
  }
  .folded:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>

<script lang="ts">
  import { conversationOf } from '../../lib/conversations.svelte';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { isWake, overseer, userWords } from '../../lib/overseer.svelte';
  import { app } from '../../lib/state.svelte';
  import type { ConvItem, OverseerAction } from '../../lib/types';
  import Markdown from '../conv/Markdown.svelte';
  import Icon from '../Icon.svelte';
  import Directives from './Directives.svelte';

  // The Overseer's minimal discussion (ESC-462), to try it before its screen (OV-2): a panel like an agent's
  // conversation, opened by its button in the title bar. The user's messages (typed), its answers, and under each
  // message what it did, with « Annuler » while it can be; the user's instructions (« Consignes · n ») unfold above.

  const view = $derived(overseer.view);
  const agentId = $derived(view.agentId);
  const conv = $derived(agentId ? conversationOf(agentId) : null);
  $effect(() => conv?.show());
  const agent = $derived(agentId ? app.agents[agentId] : undefined);
  const working = $derived(agent?.status === 'running');
  let text = $state('');
  let showDirectives = $state(false);
  let list = $state<HTMLDivElement>();

  interface Line {
    kind: 'user' | 'wake' | 'text';
    id: string;
    text: string;
    said?: boolean;
    ts?: number;
  }

  /** What the panel shows of its conversation: the user's words (their header taken off), its wake-ups folded, its answers. */
  function lines(items: ConvItem[]): Line[] {
    const out: Line[] = [];
    for (const it of items) {
      if (it.kind === 'user') {
        if (isWake(it.text)) {
          out.push({ kind: 'wake', id: it.id, text: it.text.split('\n')[0], ts: it.ts });
          continue;
        }
        const words = userWords(it.text);
        const said = words ? view.messages.find((m) => m.id === words.id)?.said : false;
        out.push({ kind: 'user', id: it.id, text: words?.text ?? it.text, said, ts: it.ts });
      } else if (it.kind === 'text' && it.text.trim() && !it.parent) {
        out.push({ kind: 'text', id: it.id, text: it.text });
      }
    }
    return out;
  }

  /** The lines, each user's message followed by what was said and done until the next one. */
  const thread = $derived.by(() => {
    const ls = lines(conv?.items ?? []);
    const starts = ls.map((l, i) => ({ i, ts: l.ts })).filter((x) => x.ts !== undefined) as { i: number; ts: number }[];
    const after = new Map<number, OverseerAction[]>();
    for (const a of view.actions) {
      // The last line that came before it, among those that tell their time.
      const at = [...starts].reverse().find((s) => s.ts <= a.at);
      const key = at ? nextStart(starts, at.i, ls.length) : ls.length;
      after.set(key, [...(after.get(key) ?? []), a]);
    }
    return { ls, after };
  });

  /** Where the actions that follow the line `i` go: before the next line that tells its time, else at the end. */
  function nextStart(starts: { i: number }[], i: number, end: number): number {
    return starts.find((s) => s.i > i)?.i ?? end;
  }

  const outcomeWord = (a: OverseerAction) => (a.outcome === 'done' ? '' : t(`overseer.panel.${a.outcome}` as 'overseer.panel.undone'));

  async function send() {
    const words = text;
    if (!words.trim() || overseer.sending) return;
    // The field empties at once (its first message may wait for its process to start); refused, the words come back.
    text = '';
    if (!(await overseer.send(words)) && !text) text = words;
  }

  $effect(() => {
    thread.ls.length;
    view.actions.length;
    queueMicrotask(() => list?.scrollTo({ top: list.scrollHeight }));
  });
</script>

{#snippet actions(list: OverseerAction[] | undefined)}
  {#each list ?? [] as a (a.id)}
    <div class="act {a.outcome}" data-action={a.tool} data-outcome={a.outcome}>
      <Icon name={a.outcome === 'refused' || a.outcome === 'failed' ? 'shield' : 'check'} size={13} />
      <span class="tool">{a.tool}</span>
      <span class="what">{a.text}</span>
      {#if outcomeWord(a)}<span class="state">{outcomeWord(a)}</span>{/if}
      {#if a.undoable}
        <button class="btn ghost small" onclick={() => overseer.undo(a.id)}>{t('overseer.panel.undo')}</button>
      {/if}
      {#if a.why}<p class="why">{a.why}</p>{/if}
    </div>
  {/each}
{/snippet}

<div class="panel" role="dialog" aria-label={t('overseer.panel.title')} data-overseer-panel>
  <header>
    <Icon name="sparkle" />
    <h2>{t('overseer.panel.title')}</h2>
    <button class="chip" aria-expanded={showDirectives} onclick={() => (showDirectives = !showDirectives)} data-directives-chip
      >{t('overseer.panel.directives', { n: view.directives.length })}</button
    >
    <button class="icon-btn" aria-label={t('overseer.panel.close')} onclick={() => (overseer.open = false)}>
      <Icon name="close" size={14} />
    </button>
  </header>
  {#if showDirectives}
    <div class="directives"><Directives /></div>
  {/if}
  {#if !view.running}
    <div class="off">
      <p>{view.enabled && view.needsMcp ? t('overseer.settings.needsMcp') : t('overseer.panel.off')}</p>
      <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'overseer' })}>{t('overseer.panel.settings')}</button>
    </div>
  {:else}
    <div class="thread" bind:this={list} data-overseer-thread>
      {#if !thread.ls.length && !view.actions.length}
        <p class="empty">{t('overseer.panel.empty')}</p>
      {/if}
      {#each thread.ls as l, i (l.id)}
        {@render actions(thread.after.get(i))}
        {#if l.kind === 'user'}
          <div class="msg user" data-user-message>
            <span class="meta">{l.said ? t('overseer.panel.said') : t('overseer.panel.typed')}{l.ts ? ` · ${fTime(l.ts)}` : ''}</span>
            <p>{l.text}</p>
          </div>
        {:else if l.kind === 'wake'}
          <div class="wake">{l.text}</div>
        {:else}
          <div class="msg reply" data-overseer-reply><Markdown text={l.text} {agentId} /></div>
        {/if}
      {/each}
      {@render actions(thread.after.get(thread.ls.length))}
      {#if working}<p class="working">{t('overseer.panel.working')}</p>{/if}
    </div>
    <form
      class="ask"
      onsubmit={(e) => {
        e.preventDefault();
        void send();
      }}
    >
      <textarea
        rows="2"
        bind:value={text}
        placeholder={t('overseer.panel.placeholder')}
        aria-label={t('overseer.panel.placeholder')}
        onkeydown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            void send();
          }
        }}></textarea>
      <button class="btn primary" type="submit" disabled={!text.trim() || overseer.sending}>{t('overseer.panel.send')}</button>
    </form>
    {#if overseer.error}<p class="error" role="alert">{overseer.error}</p>{/if}
  {/if}
</div>

<style>
  .panel {
    position: fixed;
    top: 52px;
    right: 12px;
    bottom: 40px;
    width: min(420px, calc(100vw - 32px));
    display: flex;
    flex-direction: column;
    background: var(--well, var(--bg));
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
    z-index: 40;
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--line);
  }
  h2 {
    font-size: 14px;
    margin: 0;
    flex: 1;
  }
  .chip {
    font-size: 12px;
    padding: 2px 8px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: inherit;
    cursor: pointer;
  }
  .directives {
    padding: 10px 12px;
    border-bottom: 1px solid var(--line);
    max-height: 40%;
    overflow: auto;
  }
  .off,
  .empty {
    padding: 16px;
    color: var(--muted);
  }
  .thread {
    flex: 1;
    overflow: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .msg {
    padding: 8px 10px;
    border-radius: 10px;
    max-width: 92%;
    overflow-wrap: anywhere;
  }
  .msg p {
    margin: 0;
    white-space: pre-wrap;
  }
  .user {
    align-self: flex-end;
    background: rgb(var(--fg-rgb) / 0.08);
  }
  .meta {
    display: block;
    font-size: 11px;
    color: var(--muted);
    margin-bottom: 2px;
  }
  .reply {
    align-self: flex-start;
  }
  .wake {
    font-size: 11.5px;
    color: var(--muted);
    align-self: center;
  }
  .act {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    padding: 6px 8px;
    border-left: 2px solid var(--add, var(--line));
    background: rgb(var(--fg-rgb) / 0.04);
    border-radius: 6px;
  }
  .act.refused,
  .act.failed {
    border-left-color: var(--del, var(--line));
  }
  .act.undone {
    opacity: 0.65;
  }
  .tool {
    font-family: var(--mono);
  }
  .what {
    color: var(--muted);
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .state {
    font-weight: 600;
  }
  .why {
    flex-basis: 100%;
    margin: 0;
    color: var(--muted);
  }
  .working {
    color: var(--muted);
    font-size: 12px;
    margin: 0;
  }
  .ask {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-top: 1px solid var(--line);
  }
  textarea {
    flex: 1;
    resize: none;
    font: inherit;
    padding: 6px 8px;
    border-radius: 8px;
    border: 1px solid var(--line);
    background: transparent;
    color: inherit;
  }
  .error {
    color: var(--del);
    margin: 0 12px 10px;
    font-size: 12px;
  }
</style>

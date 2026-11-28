<script lang="ts" module>
  import { SvelteSet } from 'svelte/reactivity';

  /** The lines whose command was read to its end in full screen. */
  export const fullRead = new SvelteSet<string>();
</script>

<script lang="ts">
  // An agent that stopped, unfolded on the spot (MO-5, direction B, `docs/MOBILE.md` § 3.4): what it was doing, then
  // its request whole. A permission: the command, every line, and « Autoriser » armed only once its last line was seen
  // (M4), beyond twelve lines read in full screen; then the system's lock (M3). « Refuser » takes a word, without the
  // lock. A plan: its text, « Approuver le plan » (under the lock) or « Refuser ». A question with options: its options
  // and « Répondre »; any other leads to the conversation. A space that is cut answers nothing (§ 5).
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { renderMarkdown } from '$shared/lib/markdown';
  import { answerQuestion, answerStop, conversationOf, type Outcome } from '../lib/act';
  import { COMMAND_LINES, detailOf, type Detail } from '../lib/detail';
  import { showCommand } from '../lib/feed.svelte';
  import { openTalk } from '../lib/nav.svelte';
  import type { StoppedLine } from '../lib/thread';
  import CommandBox from './CommandBox.svelte';
  import Word from './Word.svelte';

  let {
    line,
    cut,
    onSettled,
    inTalk = false,
  }: {
    line: StoppedLine;
    cut: boolean;
    onSettled: () => void;
    /** In its conversation already: no link to it. */ inTalk?: boolean;
  } = $props();

  let fetched = $state<Detail | null>(null);
  const detail = $derived(fetched ?? detailOf(line.request, null));
  let read = $state(false);
  let lines = $state(0);
  let denying = $state(false);
  let word = $state('');
  let busy = $state(false);
  let said = $state<string | null>(null);
  let picks = $state<string[]>([]);

  const q = $derived(line.request?.kind === 'question' ? line.request.questions : []);
  const onSpot = $derived(q.length === 1 && q[0].options.length > 0 && !line.request?.cut);
  const full = $derived(lines > COMMAND_LINES);
  const plan = $derived(line.stop === 'plan');

  // The conversation read once: what the agent was doing, and the request whole when the line's was cut.
  $effect(() => {
    let live = true;
    const r = line.request;
    conversationOf(line.space.id, line.agentId)
      .then((items) => live && (fetched = detailOf(r, items)))
      .catch(() => {});
    return () => (live = false);
  });

  // Read in full screen: the sheet says so.
  $effect(() => {
    if (fullRead.has(line.key)) read = true;
  });

  async function done(o: Outcome) {
    busy = false;
    if (o.ok) {
      said = t('mobile.thread.done');
      onSettled();
    } else said = 'locked' in o ? t('mobile.lock.cancelled') : o.error;
  }

  async function allow() {
    busy = true;
    said = null;
    await done(await answerStop(line, 'allow'));
  }

  async function deny() {
    busy = true;
    said = null;
    await done(await answerStop(line, 'deny', word));
  }

  async function answer() {
    const question = q[0];
    if (!picks.length) {
      said = t('mobile.answer.pickFirst');
      return;
    }
    busy = true;
    said = null;
    await done(await answerQuestion(line, { [question.question]: picks.join(', ') }));
  }

  function pick(o: string) {
    if (q[0]?.multi) picks = picks.includes(o) ? picks.filter((x) => x !== o) : [...picks, o];
    else picks = [o];
  }
</script>

<div class="fold">
  {#if detail.context}
    <p class="ctx"><b>{line.agentName}</b> · {detail.context}</p>
  {/if}

  {#if line.stop === 'permission'}
    {#if detail.text}
      <CommandBox text={detail.text} prompt={detail.shell} clamp={full && !read} onLines={(n) => (lines = n)} onEnd={() => (read = true)} />
    {/if}
    {#if line.request?.reason || (line.request?.description && line.request.description !== line.title)}
      <p class="why">{line.request?.reason ?? line.request?.description}</p>
    {/if}
    {#if !detail.whole}
      <p class="note warn">{t('mobile.answer.cut')}</p>
    {:else if full && !read}
      <button type="button" class="wide" onclick={() => showCommand(line)}>
        <Icon name="expand" size={16} />{t('mobile.answer.fullScreen', { count: lines })}
      </button>
    {/if}
  {:else if plan}
    {#if detail.plan}
      <div class="plan md">
        <!-- eslint-disable-next-line svelte/no-at-html-tags -- renderMarkdown sanitizes (DOMPurify) -->
        {@html renderMarkdown(detail.plan)}
        <span class="end"></span>
      </div>
    {/if}
  {:else if line.stop === 'question'}
    {#each q as one (one.question)}
      {#if q.length > 1 || one.question !== line.title}<p class="qq">{one.question}</p>{/if}
      {#if onSpot}
        <div class="opts" role={one.multi ? 'group' : 'radiogroup'} aria-label={one.question}>
          {#each one.options as o (o)}
            <button
              type="button"
              class="opt"
              role={one.multi ? 'checkbox' : 'radio'}
              aria-checked={picks.includes(o)}
              disabled={cut || busy}
              onclick={() => pick(o)}><span class="tick" aria-hidden="true"></span>{o}</button
            >
          {/each}
        </div>
      {/if}
    {/each}
  {:else}
    <p class="note">{t('mobile.answer.delete')}</p>
  {/if}

  {#if cut}
    <p class="note warn" role="status">{t('mobile.answer.offline')}</p>
  {/if}

  {#if denying}
    <Word bind:value={word} label={t('mobile.answer.wordFor', { agent: line.agentName })} draft={`${line.space.id}:deny:${line.key}`} />
  {/if}

  {#if line.stop === 'permission' || plan}
    <div class="acts">
      {#if denying}
        <button type="button" class="b" disabled={busy} onclick={() => (denying = false)}>{t('inbox.later.cancel')}</button>
        <button type="button" class="b" disabled={cut || busy} onclick={deny}>{t('mobile.answer.denySend')}</button>
      {:else}
        <button type="button" class="b" disabled={cut || busy} onclick={() => (denying = true)}>{t('mobile.answer.deny')}</button>
        <button
          type="button"
          class="b go"
          disabled={cut || busy || !detail.whole || (plan ? !detail.plan : !read)}
          onclick={allow}
          title={!read && !plan ? t('mobile.answer.readToEnd') : undefined}
        >
          <Icon name="lock" size={16} />{plan ? t('mobile.answer.approvePlan') : t('mobile.answer.allow')}
        </button>
      {/if}
    </div>
    {#if !plan && !read && detail.whole && !cut}
      <p class="note" data-testid="read-to-end">{t('mobile.answer.readToEnd')}</p>
    {/if}
  {:else if line.stop === 'question' && onSpot}
    <div class="acts one">
      <button type="button" class="b go" disabled={cut || busy} onclick={answer}>{t('mobile.answer.send')}</button>
    </div>
  {/if}

  {#if said}<p class="said" role="status">{said}</p>{/if}

  <div class="foot">
    {#if line.stop === 'permission' || plan}
      <span class="hint"><Icon name="lock" size={13} />{t('mobile.lock.hint')}</span>
    {/if}
    {#if !inTalk}
      <button type="button" class="link" onclick={() => openTalk(line.space.id, line.agentId)}>
        {t('mobile.answer.inConversation')}<Icon name="right" size={14} />
      </button>
    {/if}
  </div>
</div>

<style>
  .fold {
    display: grid;
    gap: 10px;
    padding: 2px 0 4px;
  }
  .ctx {
    margin: 0;
    color: var(--soft);
    font-size: 14px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .ctx b {
    font-weight: 600;
    color: var(--text);
  }
  .why,
  .qq {
    margin: 0;
    color: var(--soft);
    font-size: 14px;
  }
  .qq {
    color: var(--text);
    font-weight: 600;
  }
  .note {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
  .warn {
    color: var(--wait);
  }
  .plan {
    max-height: 50vh;
    overflow: auto;
    padding: 10px 14px;
    border-radius: var(--r-m);
    background: var(--well, rgba(0, 0, 0, 0.3));
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 14px;
  }
  .opts {
    display: grid;
    gap: 8px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--target);
    padding: 8px 12px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    text-align: left;
  }
  .opt[aria-checked='true'] {
    box-shadow: inset 0 0 0 2px var(--teal-ink);
  }
  .tick {
    flex: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--line-3);
  }
  .opt[aria-checked='true'] .tick {
    box-shadow: inset 0 0 0 5px var(--teal-ink);
  }
  .acts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--gap-target);
  }
  .acts.one {
    grid-template-columns: 1fr;
  }
  .b,
  .wide {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: calc(var(--target) + 2px);
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    font-size: 16px;
  }
  .b.go {
    background: var(--text);
    color: var(--panel);
    box-shadow: none;
  }
  .b:disabled {
    opacity: 0.45;
  }
  .wide {
    font-size: 14px;
    font-weight: 600;
  }
  .said {
    margin: 0;
    font-size: var(--small);
    color: var(--soft);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--small);
    color: var(--muted);
  }
  .hint {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    margin-left: auto;
    min-height: var(--target);
    padding: 0 4px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 650;
  }
</style>

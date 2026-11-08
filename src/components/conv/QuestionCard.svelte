<script lang="ts">
  import { captureKeys } from '../../lib/answer-keys.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { keyLabel } from '../../lib/platform';
  import { answersHere, ariaEnter, enterAnswer, optionAnswer } from '../../lib/shortcuts';
  import { speakerOf } from '../../lib/engines';
  import { conversation } from '../../lib/conversation.svelte';
  import { answerOf, draftIfAny, draftOf } from '../../lib/question-answers.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Question, QuestionItem } from '../../lib/types';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';
  import Via from './Via.svelte';

  // `current`: several requests can wait at once, and the keyboard answers the first (as the message field does).
  let { item, agentId, pending, current = true }: { item: QuestionItem; agentId: string; pending: boolean; current?: boolean } = $props();

  // The options ticked and what is written in the fields of a form (a question with `field`), by question: kept apart
  // from the card, where the voice conversation ticks them as they are said (ESC-131).
  const draft = $derived(draftIfAny(item.id));
  const picks = $derived(draft?.picks ?? {});
  const texts = $derived(draft?.texts ?? {});
  let busy = $state(false);
  let card = $state<HTMLElement>();
  /** The question the Alt+digit keys are for: the one being answered. */
  let cursor = $state(0);

  // In the split screen (ESC-392), only the active pane's request takes the keys.
  const keys = $derived(pending && current && app.keysFor(agentId));
  // « Claude attend ta réponse », its agent named as its harness names it.
  const waiting = $derived(t('composer.question.waiting', { engine: speakerOf(app.agents[agentId]?.engine) }));
  // « Codex t’a posé une question », once answered.
  const asked = $derived(t('composer.question.asked', { engine: speakerOf(app.agents[agentId]?.engine) }));
  // A form of an MCP server: who asks, and its fields to fill.
  const formTitle = $derived(
    item.form ? (item.form.source ? t('composer.question.form', { source: item.form.source }) : t('composer.question.formNoSource')) : '',
  );
  const needsConfirm = $derived(item.questions.length > 1 || item.questions.some((q) => q.multiSelect || q.field));
  // A field a form leaves optional (`required: false`) is answered empty too.
  const answered = (q: Question) =>
    q.required === false || (q.field ? (texts[q.question] ?? '').trim() !== '' : answerOf(draft, q.question) !== '');
  const complete = $derived(item.questions.every(answered));
  const firstUnanswered = () => item.questions.findIndex((q) => !answered(q));

  function choose(qi: number, label: string) {
    const q = item.questions[qi];
    const mine = draftOf(item.id);
    const cur = mine.picks[q.question] ?? [];
    if (q.multiSelect) {
      mine.picks[q.question] = cur.includes(label) ? cur.filter((x) => x !== label) : [...cur, label];
      // Several options can be ticked: the keys stay here until Ctrl+Enter moves on.
      cursor = qi;
      return;
    }
    mine.picks[q.question] = [label];
    delete mine.texts[q.question];
    // Ticked by hand while the voice asks it: the voice takes it and goes on.
    conversation.answeredInWindow(item.id);
    if (!needsConfirm) {
      submit();
      return;
    }
    // On to the next question that has no answer yet (this one again, when all have).
    const next = firstUnanswered();
    cursor = next >= 0 ? next : qi;
  }

  async function submit() {
    if (busy) return;
    busy = true;
    // The card is about to go: the focus it holds would be lost with it.
    const held = !!card?.contains(document.activeElement);
    const answers = Object.fromEntries(
      item.questions.map((q) => [q.question, q.field ? (texts[q.question] ?? '').trim() : answerOf(draft, q.question)]),
    );
    await app.run(api.answerQuestion(agentId, item.id, answers));
    busy = false;
    if (held) app.focusComposer++;
  }

  // Keys that do nothing here (a digit without an option, Ctrl+Enter on an unfinished answer) go on
  // to the message field.
  function onKeydown(e: KeyboardEvent) {
    if (!answersHere(e.target)) return;
    const n = optionAnswer(e);
    const option = n === null ? undefined : item.questions[cursor]?.options[n - 1];
    if (option) {
      e.preventDefault();
      e.stopPropagation();
      if (!busy) choose(cursor, option.label);
      return;
    }
    if (enterAnswer(e) !== 'plain') return;
    if (complete) {
      e.preventDefault();
      e.stopPropagation();
      submit();
    } else if (answered(item.questions[cursor])) {
      // This question has its answer: on to the next one that has none.
      e.preventDefault();
      e.stopPropagation();
      cursor = firstUnanswered();
    }
  }

  captureKeys(() => keys, onKeydown);
</script>

{#if pending}
  <div class="card pending" data-testid="question-pending" role="group" aria-label={waiting} bind:this={card}>
    <div class="rt"><Icon name="agents" size={14} />{waiting}</div>
    {#if item.form}
      <div class="form-head" data-testid="question-form">
        <span class="chip">{formTitle}</span>
        {#if item.form.message}<div class="text">{item.form.message}</div>{/if}
      </div>
    {/if}
    {#each item.questions as q, qi (qi)}
      <div class="q">
        {#if q.header && item.questions.length > 1}<span class="chip">{q.header}</span>{/if}
        <div class="text">
          {q.question}{#if q.required && item.form}<span class="req"> · {t('composer.question.required')}</span>{/if}
        </div>
        {#if q.field}
          <input
            class="field"
            data-own-keys
            inputmode={q.field === 'number' ? 'decimal' : 'text'}
            placeholder={q.field === 'number' ? t('composer.question.numberPlaceholder') : t('composer.question.fieldPlaceholder')}
            aria-label={q.question}
            disabled={busy}
            bind:value={() => texts[q.question] ?? '', (v) => (draftOf(item.id).texts[q.question] = v)}
            onkeydown={(e) => {
              if (e.key === 'Enter' && complete) {
                e.preventDefault();
                submit();
              }
            }}
          />
        {/if}
        <div class="opts">
          {#each q.options as o, j (j)}
            {@const on = (picks[q.question] ?? []).includes(o.label)}
            {@const hint = keys && qi === cursor && j < 9}
            <button
              class="opt"
              class:primary={j === 0 && !needsConfirm}
              class:on
              use:tip={o.description}
              disabled={busy}
              aria-keyshortcuts={hint ? `Alt+${j + 1}` : undefined}
              onclick={() => choose(qi, o.label)}
            >
              {#if q.multiSelect}<span class="box"><Icon name={on ? 'check-c' : 'circle'} size={14} /></span>{/if}{o.label}
              {#if hint}<kbd aria-hidden="true">{keyLabel(`Alt+${j + 1}`)}</kbd>{/if}
            </button>
          {/each}
        </div>
        <!-- A free answer said in the voice conversation: what was heard, to be seen. -->
        {#if !q.field && !(picks[q.question] ?? []).length && texts[q.question]}
          <div class="said" data-free-answer>{t('composer.question.saidAnswer', { text: texts[q.question] })}</div>
        {/if}
        {#if q.options.some((o) => o.description)}
          <div class="descs">
            {#each q.options as o, j (j)}
              {#if o.description}<div><b>{o.label}</b> — {o.description}</div>{/if}
            {/each}
          </div>
        {/if}
      </div>
    {/each}
    <div class="foot">
      <span class="hint">{t('composer.question.orWrite')}</span>
      {#if needsConfirm}
        <button
          class="btn primary"
          disabled={!complete || busy}
          aria-keyshortcuts={keys && complete ? ariaEnter() : undefined}
          onclick={submit}
        >
          {t('composer.question.submit')}
          {#if keys && complete}<kbd aria-hidden="true">{keyLabel('Ctrl+Enter')}</kbd>{/if}
        </button>
      {/if}
    </div>
  </div>
{:else if item.answers}
  <div class="card done">
    <div class="rt"><Icon name="agents" size={14} />{asked}</div>
    {#each item.questions as q, qi (qi)}
      {@const answer = item.answers[q.question] ?? ''}
      {@const picked = answer.split(', ')}
      <div class="done-q">{q.question}</div>
      <!-- Its choices, the one taken marked; a free answer (or a field's) said as it was given. -->
      <div class="opts-r">
        {#if q.options.length && !q.field && picked.every((p) => q.options.some((o) => o.label === p))}
          {#each q.options as o, j (j)}<span class="opt-r" class:picked={picked.includes(o.label)}>{o.label}</span>{/each}
        {:else}
          <span class="opt-r picked">{answer || '—'}</span>
        {/if}
      </div>
    {/each}
    <!-- Answered through the MCP server and not by the user in the window: it says by whom. -->
    <div class="by">
      <Icon name="check" size={14} /><span class="verdict">{t('composer.question.answered')}</span>
      {#if item.by}<Via author={item.by} />{/if}
    </div>
  </div>
{:else}
  <div class="card done">
    <div class="rt"><Icon name="agents" size={14} />{asked}</div>
    {#each item.questions as q, qi (qi)}<div class="done-q">{q.question}</div>{/each}
    <div class="by"><Icon name="circle" size={14} /><span>{t('composer.question.unanswered')}</span></div>
  </div>
{/if}

<style>
  /* A question of the agent (2.0 design): amber while it waits for the user, a hairline once answered. */
  .card {
    display: grid;
    gap: 9px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .pending {
    gap: 12px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    animation: ccFadeIn 0.2s ease-out;
  }
  .rt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .rt :global(.icon) {
    color: var(--muted);
  }
  .pending .rt :global(.icon) {
    color: var(--wait);
  }
  .q {
    display: grid;
    gap: 9px;
  }
  .chip {
    justify-self: start;
    padding: 2px 7px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
  }
  .text {
    color: var(--soft);
    font-size: 13.5px;
    line-height: 1.55;
    text-wrap: pretty;
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* A choice: a button of the app, the one ticked in the project's colour, the one a click sends in white. */
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 11px;
    border: none;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-weight: 550;
    cursor: pointer;
  }
  .opt:hover:not(:disabled) {
    background: rgba(var(--fg-rgb), 0.08);
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .opt.primary {
    background: var(--primary);
    box-shadow: inset 0 -1px 0 var(--on-primary-line);
    color: var(--on-primary);
  }
  .opt.primary:hover:not(:disabled) {
    background: var(--primary-hover);
  }
  .opt.on {
    background: color-mix(in oklch, var(--p) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent);
    color: var(--text);
  }
  .box {
    display: flex;
    color: var(--muted);
  }
  .on .box {
    color: var(--p-ink);
  }
  .opt kbd {
    margin-left: 2px;
  }
  .opt.primary kbd {
    border-color: var(--on-primary-kbd);
    color: var(--on-primary-dim);
  }
  .form-head {
    display: grid;
    gap: 8px;
  }
  .req {
    color: var(--muted);
    font-size: 11.5px;
  }
  .field {
    max-width: 360px;
  }
  .said {
    color: var(--muted);
    font-size: 12.5px;
  }
  .descs {
    display: grid;
    gap: 3px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .descs b {
    color: var(--soft);
    font-weight: 550;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .hint {
    color: var(--muted);
    font-size: 11.5px;
  }
  .done-q {
    color: var(--soft);
    font-size: 13px;
    line-height: 1.5;
    text-wrap: pretty;
  }
  .opts-r {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .opt-r {
    display: inline-flex;
    align-items: center;
    min-height: 26px;
    padding: 0 9px;
    border-radius: 7px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .opt-r.picked {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 55%, transparent);
    color: var(--text);
  }
  .by {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    color: var(--muted);
    font-size: 12px;
  }
  .by :global(.icon) {
    color: var(--add);
  }
  .verdict {
    color: var(--soft);
    font-weight: 550;
  }
  @media (prefers-reduced-motion: reduce) {
    .pending {
      animation: none;
    }
  }
</style>

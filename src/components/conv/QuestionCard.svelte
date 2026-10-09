<script lang="ts">
  import { captureKeys, takeFocusOnRequest } from '../../lib/answer-keys.svelte';
  import { api } from '../../lib/ipc';
  import { keyLabel } from '../../lib/platform';
  import { answersHere, ariaEnter, enterAnswer, optionAnswer } from '../../lib/shortcuts';
  import { app } from '../../lib/state.svelte';
  import type { Question, QuestionItem } from '../../lib/types';

  // `current`: several requests can wait at once, and the keyboard answers the first (as the message field does).
  let { item, agentId, pending, current = true }: { item: QuestionItem; agentId: string; pending: boolean; current?: boolean } = $props();

  let picks = $state<Record<string, string[]>>({});
  let busy = $state(false);
  let card = $state<HTMLElement>();
  /** The question the Alt+digit keys are for: the one being answered. */
  let cursor = $state(0);

  const keys = $derived(pending && current);
  const needsConfirm = $derived(item.questions.length > 1 || item.questions.some((q) => q.multiSelect));
  const answered = (q: Question) => (picks[q.question] ?? []).length > 0;
  const complete = $derived(item.questions.every(answered));
  const firstUnanswered = () => item.questions.findIndex((q) => !answered(q));

  function choose(qi: number, label: string) {
    const q = item.questions[qi];
    const cur = picks[q.question] ?? [];
    if (q.multiSelect) {
      picks[q.question] = cur.includes(label) ? cur.filter((x) => x !== label) : [...cur, label];
      // Several options can be ticked: the keys stay here until Ctrl+Enter moves on.
      cursor = qi;
      return;
    }
    picks[q.question] = [label];
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
    const answers = Object.fromEntries(item.questions.map((q) => [q.question, (picks[q.question] ?? []).join(', ')]));
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
  takeFocusOnRequest(
    () => card,
    () => agentId,
    () => keys,
  );
</script>

{#if pending}
  <div
    class="card pending"
    data-testid="question-pending"
    role="group"
    aria-label="Claude attend ta réponse"
    tabindex="-1"
    bind:this={card}
  >
    <div class="title"><span class="pulse" style="width:8px;height:8px"></span>Claude attend ta réponse</div>
    {#each item.questions as q, qi (qi)}
      <div class="q">
        {#if q.header && item.questions.length > 1}<span class="chip">{q.header}</span>{/if}
        <div class="text">{q.question}</div>
        <div class="opts">
          {#each q.options as o, j (j)}
            {@const on = (picks[q.question] ?? []).includes(o.label)}
            {@const hint = keys && qi === cursor && j < 9}
            <button
              class="opt"
              class:primary={j === 0 && !needsConfirm}
              class:on
              title={o.description}
              disabled={busy}
              aria-keyshortcuts={hint ? `Alt+${j + 1}` : undefined}
              onclick={() => choose(qi, o.label)}
            >
              {#if q.multiSelect}<span class="box">{on ? '☑' : '☐'}</span>{/if}{o.label}
              {#if hint}<kbd class="kbd" aria-hidden="true">{keyLabel(`Alt+${j + 1}`)}</kbd>{/if}
            </button>
          {/each}
        </div>
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
      <span class="hint">ou réponds librement dans le champ ci-dessous</span>
      {#if needsConfirm}
        <button
          class="btn primary"
          disabled={!complete || busy}
          aria-keyshortcuts={keys && complete ? ariaEnter() : undefined}
          onclick={submit}
        >
          Valider
          {#if keys && complete}<kbd class="kbd" aria-hidden="true">{keyLabel('Ctrl+Entrée')}</kbd>{/if}
        </button>
      {/if}
    </div>
  </div>
{:else if item.answers}
  <div class="card done">
    {#each item.questions as q, qi (qi)}
      <div class="done-q">{q.question}</div>
      <div class="ans mono">→ {item.answers[q.question] ?? '—'}</div>
    {/each}
  </div>
{:else}
  <div class="card done">
    {#each item.questions as q, qi (qi)}<div class="done-q">{q.question}</div>{/each}
    <div class="ans mono" style="color:var(--dim)">Question restée sans réponse</div>
  </div>
{/if}

<style>
  .card {
    margin-left: 34px;
    display: flex;
    flex-direction: column;
    border-radius: var(--r);
  }
  .pending {
    gap: 12px;
    padding: 16px 18px;
    border: 1px solid var(--wait);
    background: var(--wait-soft);
    animation: ccFadeIn 0.2s ease-out;
  }
  .card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    font-weight: 700;
    color: var(--wait);
    letter-spacing: 0.02em;
  }
  .q {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .chip {
    align-self: flex-start;
    font-family: var(--mono);
    font-size: 10.5px;
    padding: 2px 7px;
    border-radius: 99px;
    background: var(--elev2);
    color: var(--muted);
  }
  .text {
    font-size: 14px;
    line-height: 1.55;
    text-wrap: pretty;
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .opt {
    height: 32px;
    padding: 0 14px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: transparent;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .opt:hover:not(:disabled) {
    filter: brightness(1.08);
    border-color: var(--wait);
  }
  .opt.primary,
  .opt.on {
    background: var(--wait);
    border-color: var(--wait);
    color: #2a1f05;
  }
  .box {
    font-size: 13px;
  }
  /* The shortcut, in the button's own colors: the amber of a picked option is no place for the dim gray. */
  .opt .kbd,
  .btn .kbd {
    margin-left: 2px;
    color: inherit;
    opacity: 0.7;
  }
  .descs {
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--muted);
    line-height: 1.45;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .hint {
    font-size: 11.5px;
    color: var(--dim);
  }
  .done {
    gap: 6px;
    padding: 12px 16px;
    border: 1px solid var(--line);
  }
  .done-q {
    font-size: 13px;
    line-height: 1.5;
    color: var(--muted);
    text-wrap: pretty;
  }
  .ans {
    font-size: 11.5px;
  }
</style>

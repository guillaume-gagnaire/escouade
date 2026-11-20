<script lang="ts">
  import { fSince } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { act, later, run } from '../../lib/inbox/act';
  import { fLater, laterTimes } from '../../lib/inbox/inbox.svelte';
  import { answerable, blocking, gesturesOf, type Gesture, type InboxItem } from '../../lib/inbox/views';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import { toolLabel } from '../../lib/tools';
  import RejectForm from '../board/RejectForm.svelte';
  import Icon from '../Icon.svelte';
  import PrivateMark from '../PrivateMark.svelte';
  import { gestureLabel, viewTitle } from './view-title';

  // A view of the box « À toi » opened in the panel of its screen (ESC-513, in ESC-510's `InboxScreen`): what its source
  // says of it (the command or the plan asked for, the question and its options, the ticket's block), the gestures of its
  // source with what they need (a word with a refusal, the options of a question, what to change for a ticket sent back),
  // and « Plus tard » at the times the box offers. Everything it does, its source does: what it says under its gestures.
  // Where it comes from is in the screen's bar.
  let { view, ondone }: { view: InboxItem; ondone?: (v: InboxItem) => void } = $props();

  const gestures = $derived(gesturesOf(view));
  let busy = $state(false);
  let denying = $state(false);
  let rejecting = $state(false);
  let word = $state('');
  /** The option picked for each question, by question. */
  let picked = $state<Record<string, string>>({});
  const questions = $derived(view.request?.questions ?? []);
  const complete = $derived(questions.every((q) => picked[q.question]));

  // Another view in the pane: what was typed for the last one goes.
  $effect(() => {
    void view.key;
    denying = false;
    rejecting = false;
    word = '';
    picked = {};
  });

  async function go(p: Promise<boolean>) {
    if (busy) return;
    busy = true;
    try {
      if (await p) ondone?.(view);
    } finally {
      busy = false;
    }
  }

  function gesture(g: Gesture) {
    if (g.id === 'deny') denying = true;
    else if (g.id === 'reject') rejecting = true;
    else if (g.id === 'answer') void go(act(view, { act: 'answer', answers: picked }));
    else void go(run(view, g));
  }

  const note = $derived.by(() => {
    switch (view.source) {
      case 'permission':
      case 'question':
      case 'plan':
        return t('inboxViews.note.request', { agent: view.agentName ?? '' });
      case 'ticketReview':
      case 'ticketBlocked':
        return t('inboxViews.note.ticket');
      case 'decision':
        return t('inboxViews.note.decision');
      case 'docs':
        return t('inboxViews.note.docs');
      case 'review':
        return t('inboxViews.note.review');
      case 'handoff':
        return t('inboxViews.note.handoff', { branch: view.branch ?? '' });
    }
  });

  /** « Plus tard »: the times the box offers, as for a request. */
  function putOff(e: MouseEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = laterTimes(Date.now()).map((x) => ({
      label: t(`inbox.later.${x.key}`),
      hint: fLater(x.at),
      onClick: () => void go(later(view, x.at).then(() => true)),
    }));
    menu.open = { x: r.left, y: r.bottom + 4, items, keyboard: e.detail === 0 };
  }
</script>

<section class="ivw-pane" data-testid="ivw-pane" data-key={view.key} aria-labelledby="ivw-pane-title">
  <header>
    <div class="kind" class:stopped={blocking(view)}>
      {t(`inboxViews.kind.${view.source}`)}{#if blocking(view)}{' · '}{t('inboxViews.stopped')}{/if}
    </div>
    <h2 id="ivw-pane-title">{viewTitle(view)}</h2>
    <div class="meta">
      {#if view.private}<PrivateMark />{/if}
      <span>{view.projectName}</span>
      {#if view.agentName}<span>· {view.agentName}</span>{/if}
      {#if view.ticketKey}<span>· {view.ticketKey}</span>{/if}
      {#if view.since}<span>· {t('inboxViews.since', { since: fSince(view.since, app.now) })}</span>{/if}
      {#if view.snoozedUntil}<span>· {t('inboxViews.until', { when: fLater(view.snoozedUntil) })}</span>{/if}
    </div>
  </header>

  {#if view.request && view.source !== 'question'}
    <div class="what">
      <div class="tool">{toolLabel(view.request.tool)}</div>
      <pre class="arg">{view.request.arg}</pre>
      {#if view.request.description}<p class="desc">{view.request.description}</p>{/if}
      {#if view.request.reason}<p class="desc">{view.request.reason}</p>{/if}
    </div>
  {:else if view.source === 'question' && answerable(view)}
    {#each questions as q (q.question)}
      <fieldset class="question">
        <legend>{q.question}</legend>
        {#each q.options as o (o)}
          <label class="opt">
            <input type="radio" name={`ivw-${view.key}-${q.question}`} value={o} bind:group={picked[q.question]} />
            <span>{o}</span>
          </label>
        {/each}
      </fieldset>
    {/each}
  {:else if view.blocked}
    <p class="blocked">{view.blocked}</p>
  {/if}

  {#if denying}
    <div class="form">
      <label class="field-label" for="ivw-deny">{t('inboxViews.denyLabel')}</label>
      <textarea id="ivw-deny" class="field" rows="3" bind:value={word}></textarea>
      <div class="acts">
        <button class="btn small" disabled={busy} onclick={() => go(act(view, { act: 'deny', message: word.trim() || null }))}
          >{t('inboxViews.act.deny')}</button
        >
        <button class="btn small ghost" onclick={() => (denying = false)}>{t('common.cancel')}</button>
      </div>
    </div>
  {:else if rejecting}
    <div class="form">
      <RejectForm onsubmit={(comment) => go(act(view, { act: 'reject', comment }))} oncancel={() => (rejecting = false)} />
    </div>
  {:else}
    <div class="acts">
      {#each gestures as g (g.id)}
        <button
          class="btn"
          class:primary={g.primary}
          disabled={busy || (g.id === 'answer' && !complete)}
          data-gesture={g.id}
          onclick={() => gesture(g)}>{gestureLabel(view, g)}</button
        >
      {/each}
      {#if !blocking(view)}
        {#if view.snoozedUntil}
          <button class="btn ghost" onclick={() => later(view, null)}>{t('inboxViews.later.back')}</button>
        {:else}
          <button class="btn ghost" aria-haspopup="menu" onclick={putOff} data-gesture="later"
            ><Icon name="clock" size={13} />{t('inboxViews.later.button')}</button
          >
        {/if}
      {/if}
    </div>
  {/if}
  <p class="note">{note}</p>
</section>

<style>
  .ivw-pane {
    display: grid;
    align-content: start;
    gap: 14px;
    padding: 20px 24px;
    min-width: 0;
    color: var(--text);
  }
  header {
    display: grid;
    gap: 6px;
  }
  .kind {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .kind.stopped {
    color: var(--wait);
  }
  h2 {
    margin: 0;
    font-size: 17px;
    font-weight: 600;
    line-height: 1.3;
    overflow-wrap: anywhere;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--dim);
  }
  .what {
    display: grid;
    gap: 6px;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--well);
  }
  .tool {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .arg {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .desc,
  .blocked,
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .blocked {
    color: var(--warn-ink);
  }
  .question {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: var(--r);
  }
  legend {
    padding: 0 4px;
    font-size: 13px;
    font-weight: 600;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }
  .form {
    display: grid;
    gap: 8px;
  }
  .field-label {
    font-size: 12px;
    color: var(--muted);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .acts .btn :global(.icon) {
    margin-right: 4px;
  }
</style>

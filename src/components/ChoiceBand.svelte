<script lang="ts">
  // The band of the choice under an agent's page (AP-4; docs/AGENT-PAGES.md § 6 and § 8.2, direction A): native, out of
  // the rectangle of the page's webview, so that no page can draw it. A click in the page only proposes (`_/choose`,
  // checked by the core); here the proposal is said in the request's own words (« Choisi dans la page : A · Fusionner
  // dans main »), and « Envoyer » sends it by the box's own command, on the user's gesture alone. « Envoyer » waits a
  // moment after the proposal changed, so that a page cannot swap it under the click. A go-ahead is never proposed from
  // the page: the band answers it with its own « Non » and « Oui ». Then the time to take the answer back, and the answer.
  import { ARM_MS, answerOf, bandOf, type Chip } from '../lib/choiceband';
  import { t } from '../lib/i18n';
  import { inbox } from '../lib/inbox/inbox.svelte';
  import { api } from '../lib/ipc';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { AskAnswerIn, PageProposal, WindowAsk } from '../lib/types';
  import Icon from './Icon.svelte';

  let {
    page,
    askId,
    proposal = null,
    agent,
  }: {
    /** `p-<n>`. */
    page: string;
    /** The request the page carries. */
    askId: string;
    proposal?: PageProposal | null;
    agent: string;
  } = $props();

  let now = $state(Date.now());
  const ask = $derived<WindowAsk | null>(inbox.asks.find((a) => a.id === askId) ?? null);
  const view = $derived(bandOf(ask, proposal, now));
  let busy = $state(false);

  // « Envoyer » is armed a moment after what the page proposed last changed.
  let armed = $state(false);
  $effect(() => {
    void [proposal?.options.join(','), proposal?.note, proposal?.at];
    armed = false;
    const id = setTimeout(() => (armed = true), ARM_MS);
    return () => clearTimeout(id);
  });
  // The time to take the answer back runs out.
  $effect(() => {
    if (view.kind !== 'sending') return;
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  async function answer(a: AskAnswerIn) {
    if (!ask || busy) return;
    busy = true;
    const done = await app.run(api.inboxAnswer(ask.id, a));
    busy = false;
    if (!done) return;
    inbox.put(done);
    inbox.sending = { id: done.id, title: done.title, sendAt: done.answer?.sendAt ?? Date.now() + 10_000 };
    now = Date.now();
  }

  function send() {
    if (!ask || view.kind !== 'proposed' || !armed) return;
    void answer(answerOf(ask, view.options, view.note));
  }

  async function cancel() {
    await app.run(api.agentpageDropProposal(page));
  }

  async function takeBack() {
    if (!ask || busy) return;
    busy = true;
    const again = await app.run(api.inboxReopen(ask.id));
    busy = false;
    if (again) inbox.put(again);
  }

  function inBox() {
    if (!ask) return;
    inbox.select(ask.id);
    app.openInbox();
  }

  const said = (c: Chip) => (c.letter ? `${c.letter} · ${c.label}` : c.label);
</script>

{#if view.kind !== 'none'}
  <div
    class="choice"
    class:lit={view.kind === 'proposed' || view.kind === 'gate'}
    role="group"
    aria-label={t('devpage.choice.label', { agent })}
    data-testid="choice-band"
    data-state={view.kind}
  >
    <span class="ic"><Icon name={view.kind === 'answered' || view.kind === 'sending' ? 'check-c' : 'choice'} size={14} /></span>
    {#if view.kind === 'idle'}
      <p class="say quiet">{t('devpage.choice.idle')}</p>
      <div class="acts">
        <button class="btn ghost" onclick={inBox}>{t('devpage.choice.inBox')}</button>
      </div>
    {:else if view.kind === 'gate'}
      <p class="say">{t('devpage.choice.gate')}</p>
      <div class="acts">
        <button class="btn" disabled={busy} onclick={() => answer({ yes: false, via: 'click' })} data-testid="choice-no">{view.no}</button>
        <button class="btn primary" disabled={busy} onclick={() => answer({ yes: true, via: 'click' })} data-testid="choice-yes"
          >{view.yes}</button
        >
      </div>
    {:else if view.kind === 'proposed'}
      <p class="say" use:tip={{ text: t('devpage.choice.trust'), aria: false }}>
        <span class="lead">{t('devpage.choice.proposed')}</span>
        {#each view.chips as c (c.id)}<span class="chip" data-testid="choice-chip" data-option={c.id}>{said(c)}</span>{/each}
        {#if view.note}<span class="note">{t('devpage.choice.note', { note: view.note })}</span>{/if}
      </p>
      <div class="acts">
        <button class="btn ghost" onclick={cancel} data-testid="choice-cancel">{t('devpage.choice.cancel')}</button>
        <button class="btn primary" disabled={busy || !armed} onclick={send} data-testid="choice-send"
          ><Icon name="enter" size={13} />{t('devpage.choice.send', { agent })}</button
        >
      </div>
    {:else if view.kind === 'sending'}
      <p class="say">
        <span class="lead">{t('devpage.choice.answered')}</span>
        {#each view.chips as c (c.id)}<span class="chip">{said(c)}</span>{/each}
        <span class="note">{t('devpage.choice.sending', { agent })}</span>
      </p>
      <div class="acts">
        <button class="btn" disabled={busy} onclick={takeBack} data-testid="choice-take-back"
          ><Icon name="undo" size={13} />{t('devpage.choice.taken')}</button
        >
      </div>
    {:else}
      <p class="say quiet">
        <span class="lead">{t('devpage.choice.answered')}</span>
        {#each view.chips as c (c.id)}<span class="chip">{said(c)}</span>{/each}
      </p>
    {/if}
  </div>
{/if}

<style>
  /* Under the page, out of its webview: a hairline above, the band's ink; lit in amber while a choice waits for a gesture. */
  .choice {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 6px 10px 6px 14px;
    border-top: 1px solid var(--line);
    background: rgba(var(--fg-rgb), 0.02);
    min-width: 0;
    container: choiceband / inline-size;
  }
  .choice.lit {
    background: var(--wait-bg);
    border-top-color: var(--wait-line);
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--muted);
  }
  .lit .ic {
    color: var(--warn-ink);
  }
  .say {
    flex: 1;
    min-width: 0;
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--text);
  }
  .say.quiet {
    color: var(--muted);
  }
  .lead {
    color: var(--soft);
  }
  .chip {
    display: inline-flex;
    align-items: center;
    max-width: 100%;
    height: 22px;
    padding: 0 8px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    flex: none;
    gap: 6px;
  }
  @container choiceband (max-width: 520px) {
    .choice {
      flex-wrap: wrap;
    }
    .acts {
      margin-left: auto;
    }
  }
</style>

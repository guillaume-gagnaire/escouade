<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { agentOffer, pendingPlace, submitBlock, verdictsOf, type ReviewAsk } from '../../lib/myreview';
  import type { PrWriting } from '../../lib/prthreads';
  import { tip } from '../../lib/tip';
  import type { PendingNote, PendingReview, ReviewVerdict, Ticket } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';

  // The panel of the header's button « Ta relecture » (ESC-494, PR-7; the menu of design/pr-2-3 direction A): the review's
  // text (Markdown, needed to ask for changes), the remarks waiting (a click goes to the line; a stale one says so), the
  // verdict among those the host gives (a comment alone on the user's own pull request, the reason said), « Renvoyer aussi
  // à l'agent du ticket » for a ticket's pull request, and « Publier sur <hébergeur> », the only gesture that sends it.
  // « Abandonner ma relecture » asks first. Its text is kept on this machine as it is typed (`onBody`).
  let {
    review,
    number,
    host,
    word,
    writing,
    own,
    ticket,
    agentName,
    paths,
    onBody,
    onSubmit,
    onDiscard,
    onGo,
    onClose,
  }: {
    review: PendingReview;
    number: number;
    host: string;
    word: string;
    writing: PrWriting;
    own: boolean;
    /** The ticket whose pull request it is. */
    ticket: Ticket | null;
    agentName: string;
    /** The files in the order of the tab: the remarks in that order. */
    paths: string[];
    onBody: (body: string) => void;
    /** Published: null when done, else why not (all of it stays waiting). */
    onSubmit: (ask: ReviewAsk) => Promise<string | null>;
    onDiscard: () => Promise<void>;
    onGo: (n: PendingNote) => void;
    onClose: () => void;
  } = $props();

  let text = $state(untrack(() => review.body));
  let verdict = $state<ReviewVerdict>('commented');
  let preview = $state(false);
  let busy = $state(false);
  let failure = $state<string | null>(null);
  let confirming = $state(false);
  /** The user ticked or unticked the case: their say, kept when the verdict changes. */
  let agentSaid = $state<boolean | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);

  const offers = $derived(verdictsOf(writing, own, host, word));
  const notes = $derived(
    [...review.notes].sort((a, b) => {
      const at = (p: string) => paths.indexOf(p) + 1 || paths.length + 1;
      return at(a.path) - at(b.path) || a.line - b.line || a.id - b.id;
    }),
  );
  const offer = $derived(agentOffer(ticket, verdict, own));
  const toAgent = $derived(offer.offered && (agentSaid ?? offer.checked));
  const block = $derived(submitBlock(verdict, text, review.notes));

  // A verdict the host no longer offers (the pull request turned out to be the user's own): back to a comment.
  $effect(() => {
    const v = verdict;
    untrack(() => {
      if (!offers.find((o) => o.verdict === v)?.offered) verdict = 'commented';
    });
  });

  let saving: ReturnType<typeof setTimeout> | null = null;
  function typed(v: string) {
    text = v;
    if (saving) clearTimeout(saving);
    saving = setTimeout(() => onBody(text), 400);
  }

  async function publish() {
    if (block || busy) return;
    if (saving) clearTimeout(saving);
    busy = true;
    failure = null;
    failure = await onSubmit({ verdict, body: text, toAgent });
    busy = false;
  }

  async function discard() {
    busy = true;
    await onDiscard();
    busy = false;
    confirming = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (confirming) confirming = false;
      else onClose();
    } else if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void publish();
    }
  }

  $effect(() => {
    if (box) untrack(() => box?.focus());
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="mine-panel"
  id="pr-my-review"
  role="dialog"
  aria-label={t('prs.page.mine.title', { word, number })}
  tabindex="-1"
  onkeydown={onKey}
>
  <header class="mh">
    <h2>{t('prs.page.mine.title', { word, number })}</h2>
    <button class="icon-btn" aria-label={t('prs.page.mine.close')} use:tip={t('prs.page.mine.close')} onclick={onClose}
      ><Icon name="close" size={13} /></button
    >
  </header>

  <div class="sec">
    <div class="modes" role="tablist" aria-label={t('prs.page.mine.text')}>
      <button type="button" role="tab" aria-selected={!preview} onclick={() => (preview = false)}>{t('prs.page.threads.write')}</button>
      <button type="button" role="tab" aria-selected={preview} onclick={() => (preview = true)}>{t('prs.page.threads.preview')}</button>
    </div>
    {#if preview}
      <div class="preview">
        {#if text.trim()}<Markdown {text} />{:else}<p class="muted">{t('prs.page.threads.emptyPreview')}</p>{/if}
      </div>
    {:else}
      <textarea
        bind:this={box}
        rows="4"
        aria-label={t('prs.page.mine.text')}
        placeholder={t('prs.page.mine.textPlaceholder')}
        value={text}
        disabled={busy}
        oninput={(e) => typed(e.currentTarget.value)}></textarea>
    {/if}
  </div>

  <div class="sec">
    <h3>{t('prs.page.mine.notes', { count: notes.length })}</h3>
    {#if notes.length}
      <ul class="notes">
        {#each notes as n (n.id)}
          <li class:stale={n.stale}>
            <button class="go" onclick={() => onGo(n)} aria-label={t('prs.page.mine.goTo', { place: pendingPlace(n) })}>
              <span class="mono place">{pendingPlace(n)}</span>
              {#if n.stale}<span class="flag">{t('prs.page.mine.stale')}</span>{/if}
              <span class="ell">{n.body}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if notes.some((n) => n.stale)}<p class="hint">{t('prs.page.mine.staleHint', { word })}</p>{/if}
    {:else}
      <p class="hint">{t('prs.page.mine.noNotes')}</p>
    {/if}
  </div>

  <div class="sec" role="radiogroup" aria-label={t('prs.page.mine.verdict')}>
    {#each offers as o (o.verdict)}
      <label class="it" class:on={verdict === o.verdict} class:off={!o.offered} use:tip={o.why ?? ''}>
        <input
          type="radio"
          name="pr-verdict"
          value={o.verdict}
          checked={verdict === o.verdict}
          disabled={!o.offered || busy}
          onchange={() => (verdict = o.verdict)}
        />
        <span class="v"
          ><b>{t(`prs.page.mine.verdicts.${o.verdict}`)}</b><small>{o.why ?? t(`prs.page.mine.verdictHints.${o.verdict}`, { word })}</small
          ></span
        >
      </label>
    {/each}
    {#if own}<p class="hint" role="note"><Icon name="info" size={12} />{t('prs.page.mine.own', { host, word })}</p>{/if}
  </div>

  {#if offer.offered}
    <div class="sec">
      <label class="agent" use:tip={t('prs.page.mine.toAgentHint', { agent: agentName })}>
        <input type="checkbox" checked={toAgent} disabled={busy} onchange={(e) => (agentSaid = e.currentTarget.checked)} />
        <span>{t('prs.page.mine.toAgent')}</span>
      </label>
      {#if own && verdict === 'commented'}<p class="hint">{t('prs.page.mine.ownToAgent', { word })}</p>{/if}
    </div>
  {/if}

  {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
  {#if block && (text.trim() || review.notes.length || verdict === 'changesRequested')}<p class="hint block">{block}</p>{/if}

  {#if confirming}
    <div class="confirm" role="alertdialog" aria-label={t('prs.page.mine.discard')}>
      <p>{t('prs.page.mine.discardConfirm', { count: review.notes.length })}</p>
      <div class="acts">
        <button class="btn small danger" disabled={busy} onclick={discard}>{t('prs.page.mine.discardYes')}</button>
        <button class="btn small" disabled={busy} onclick={() => (confirming = false)}>{t('prs.page.mine.discardNo')}</button>
      </div>
    </div>
  {/if}

  <footer class="mf">
    {#if review.body.trim() || review.notes.length || text.trim()}
      <button class="link discard" disabled={busy} onclick={() => (confirming = true)}>{t('prs.page.mine.discard')}</button>
    {/if}
    <span class="sp"></span>
    <button class="btn small" disabled={busy} onclick={onClose}>{t('prs.page.mine.close')}</button>
    <button class="btn small primary" disabled={busy || !!block} use:tip={t('prs.page.mine.publishedAs', { host })} onclick={publish}
      ><Icon name="push" size={12} />{busy ? t('prs.page.mine.publishing') : t('prs.page.mine.publish', { host })}</button
    >
  </footer>
</div>

<style>
  .mine-panel {
    position: absolute;
    z-index: 20;
    top: calc(100% - 4px);
    right: 12px;
    width: min(400px, calc(100% - 24px));
    max-height: min(620px, calc(100vh - 140px));
    overflow: auto;
    display: grid;
    gap: 2px;
    padding: 6px;
    border-radius: 12px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 6px 24px oklch(0 0 0 / 0.22);
    font-size: 12.5px;
  }
  .mine-panel:focus {
    outline: none;
  }
  .mh {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 4px 2px 8px;
  }
  h2 {
    margin: 0;
    flex: 1;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  h3 {
    margin: 2px 0 4px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .sec {
    padding: 6px 8px;
    display: grid;
    gap: 4px;
  }
  .sec + .sec {
    border-top: 1px solid var(--line);
  }
  .modes {
    display: flex;
    gap: 2px;
  }
  .modes button {
    all: unset;
    cursor: pointer;
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .modes button[aria-selected='true'] {
    color: var(--text);
    background: var(--hover);
  }
  .modes button:focus-visible {
    outline: 2px solid var(--p-ink);
  }
  textarea,
  .preview {
    width: 100%;
    box-sizing: border-box;
    min-height: 74px;
    font: inherit;
    font-size: 13px;
    padding: 7px 9px;
    border-radius: 8px;
    border: none;
    color: var(--text);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  textarea {
    resize: vertical;
  }
  textarea:focus-visible {
    outline: 2px solid var(--p-ink);
  }
  .notes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .go {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 6px;
    border-radius: 6px;
    cursor: pointer;
    min-width: 0;
  }
  .go:hover,
  .go:focus-visible {
    background: var(--hover);
  }
  .go:focus-visible {
    outline: 2px solid var(--p-ink);
  }
  .place {
    flex: none;
    color: var(--soft);
    font-size: 11.5px;
  }
  .stale .place {
    color: var(--muted);
    text-decoration: line-through;
  }
  .flag {
    flex: none;
    font-size: 10.5px;
    font-weight: 600;
    color: var(--muted);
    padding: 0 5px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .it {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    align-items: start;
    padding: 6px 6px;
    border-radius: 7px;
    cursor: pointer;
  }
  .it.on {
    background: var(--hover);
  }
  .it.off {
    cursor: default;
    opacity: 0.6;
  }
  .it input {
    margin: 2px 0 0;
    accent-color: var(--p-ink);
  }
  .v {
    display: grid;
    gap: 1px;
  }
  .v small {
    color: var(--muted);
    font-size: 11.5px;
  }
  .agent {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .agent input {
    accent-color: var(--p-ink);
  }
  .hint {
    display: flex;
    gap: 5px;
    align-items: flex-start;
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .hint.block {
    padding: 0 8px;
  }
  .fail {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 4px 8px;
    padding: 6px 9px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .confirm {
    margin: 4px 8px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    display: grid;
    gap: 6px;
  }
  .confirm p {
    margin: 0;
  }
  .acts {
    display: flex;
    gap: 6px;
  }
  .mf {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px 4px;
    border-top: 1px solid var(--line);
  }
  .sp {
    flex: 1;
  }
  .link {
    all: unset;
    cursor: pointer;
    color: var(--muted);
    font-size: 12px;
  }
  .link:hover,
  .link:focus-visible {
    color: var(--del);
    text-decoration: underline;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
  }
</style>

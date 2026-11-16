<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { edited, noteKey, notePlace, suggestionDiff } from '../../lib/prreview';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { ReviewNote } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';

  // A remark of the reviewer agent as a thread under its line in the tab « Fichiers » (ESC-490, PR-5 of the PR in
  // Escouade): marked « agent, pas publié » (its colour, its tag, its icon: read without the colour too), its severity, its
  // text (the agent's, under a third party's influence: rendered as a third party's Markdown, nothing remote without a
  // click), its suggestion as a diff (read and copied, never run). « Écarter » resolves it here (folded, kept, « Reprendre »
  // opens it again); « Garder » sets it aside for the publication of those kept; « Publier » opens its text to read and
  // correct, then sends it on the host as the user's comment on the click of « Publier sur <hébergeur> ». Nothing else
  // leaves.
  let {
    note,
    host,
    word,
    onLine,
    current = false,
    showPlace = false,
    decide,
    publish,
  }: {
    note: ReviewNote;
    host: string;
    word: string;
    /** It sits on its line in the diff read now: published there, else in the conversation. */
    onLine: boolean;
    current?: boolean;
    showPlace?: boolean;
    /** Kept, dropped or opened again: null when done, else why not. */
    decide: (n: ReviewNote, choice: 'open' | 'kept' | 'dropped') => Promise<string | null>;
    /** Published with `body`: null when done, else why not. */
    publish: (n: ReviewNote, body: string) => Promise<string | null>;
  } = $props();

  let unfolded = $state(false);
  let writing = $state(false);
  let text = $state('');
  let busy = $state(false);
  let failure = $state<string | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);

  const key = $derived(noteKey(note));
  const place = $derived(notePlace(note, word));
  const folded = $derived(note.choice === 'dropped' && !unfolded);
  const severity = $derived(t(`prs.review.severity.${note.severity}`));
  const diff = $derived(suggestionDiff(note));

  async function choose(choice: 'open' | 'kept' | 'dropped') {
    busy = true;
    failure = null;
    failure = await decide(note, choice);
    busy = false;
    if (!failure && choice !== 'dropped') unfolded = false;
  }

  function openPublish() {
    text = note.body;
    failure = null;
    writing = true;
  }

  async function send() {
    if (!text.trim()) {
      failure = t('prs.review.empty');
      return;
    }
    busy = true;
    failure = null;
    failure = await publish(note, text);
    busy = false;
    if (!failure) writing = false;
  }

  async function copy() {
    try {
      await navigator.clipboard.writeText(note.suggestion ?? '');
      app.toast(t('prs.review.copied'), 'ok');
    } catch (e) {
      app.toast(String(e), 'error');
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void send();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      writing = false;
    }
  }

  $effect(() => {
    if (!writing || !box) return;
    untrack(() => box?.focus());
  });
</script>

{#snippet sev()}
  <span class="sev {note.severity}" aria-label={t('prs.review.severityOf', { severity })}
    ><Icon
      name={note.severity === 'security' ? 'shield' : note.severity === 'bug' ? 'info' : note.severity === 'spec' ? 'page' : 'circle'}
      size={11}
    />{severity}</span
  >
{/snippet}

{#if folded}
  <div class="thread agent folded" class:current data-thread-key={key} data-severity={note.severity} data-choice={note.choice}>
    <button class="unfold" aria-expanded="false" aria-label={t('prs.page.threads.unfold')} onclick={() => (unfolded = true)}>
      <span class="ts dropped"><Icon name="check-c" size={13} />{t('prs.review.dropped')}</span>
      {@render sev()}
      {#if showPlace}<span class="mono place">{place}</span>{/if}
      <span class="ell muted">{note.body}</span>
    </button>
    <button class="btn small" disabled={busy} onclick={() => choose('open')}>{t('prs.review.takeBack')}</button>
    {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
  </div>
{:else}
  <article
    class="thread agent"
    class:current
    data-thread-key={key}
    data-severity={note.severity}
    data-choice={note.choice}
    tabindex="-1"
    aria-label={t('prs.review.noteLabel', { place })}
  >
    <div class="msg">
      <span class="av" aria-hidden="true"><Icon name="sparkle" size={12} /></span>
      <div class="who">
        <b>{t('prs.review.reviewer')}</b>
        <span class="tag" use:tip={t('prs.review.tagHint', { host })}>{t('prs.review.tag')}</span>
        {@render sev()}
        {#if showPlace}<span class="mono place">{place}</span>{/if}
        {#if note.stale}<span class="flag">{t('prs.review.staleTag')}</span>{:else if note.outside}<span class="flag"
            >{t('prs.review.outsideTag')}</span
          >{/if}
        {#if note.choice === 'kept'}<span class="flag kept"><Icon name="check" size={11} />{t('prs.review.kept')}</span>{/if}
        {#if note.choice === 'dropped'}<span class="flag">{t('prs.review.dropped')}</span>{/if}
        {#if edited(note)}<span class="flag">{t('prs.review.edited')}</span>{/if}
        {#if note.choice === 'dropped'}
          <button class="icon-btn fold" aria-label={t('prs.page.threads.fold')} aria-expanded="true" onclick={() => (unfolded = false)}
            ><Icon name="up" size={12} /></button
          >
        {/if}
      </div>
      <div class="bd third"><Markdown text={note.body} /></div>
    </div>
    {#if diff.length}
      <div class="sugg" role="group" aria-label={t('prs.review.suggestion')}>
        <div class="sh">
          <Icon name="edit" size={12} /><span use:tip={t('prs.review.suggestionHint')}>{t('prs.review.suggestion')}</span>
          <button class="btn small copy" onclick={copy}><Icon name="copy" size={12} />{t('prs.review.copy')}</button>
        </div>
        <pre class="code mono">{#each diff as l, i (i)}<span class="dl {l.kind === '+' ? 'add' : 'del'}"
              ><span class="k" aria-hidden="true">{l.kind}</span><span class="tx">{l.text}</span>
</span>{/each}</pre>
      </div>
    {/if}
    {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
    {#if writing}
      <div class="pub">
        <textarea
          bind:this={box}
          bind:value={text}
          rows="3"
          aria-label={t('prs.review.publishLabel', { host })}
          onkeydown={onKey}
          disabled={busy}></textarea>
        <p class="hint">
          {onLine ? t('prs.review.publishOnLine', { host, place }) : t('prs.review.publishOff', { host, word })}
          {#if note.suggestion !== undefined}{' '}{t('prs.review.withSuggestion')}{/if}
        </p>
        <div class="acts">
          <button class="btn small primary" disabled={busy || !text.trim()} onclick={send}
            ><Icon name="push" size={12} />{busy ? t('prs.review.publishing') : t('prs.review.publishOn', { host })}</button
          >
          {#if text !== note.proposed}
            <button class="btn small" disabled={busy} onclick={() => (text = note.proposed)}>{t('prs.review.backToAgent')}</button>
          {/if}
          <button class="btn small" disabled={busy} onclick={() => (writing = false)}>{t('prs.review.cancel')}</button>
        </div>
      </div>
    {:else}
      <footer class="acts">
        <button class="btn small" disabled={busy} use:tip={t('prs.review.publishHint', { host })} onclick={openPublish}
          ><Icon name="push" size={12} />{t('prs.review.publish')}</button
        >
        {#if note.choice === 'kept'}
          <button class="btn small" disabled={busy} onclick={() => choose('open')}>{t('prs.review.unkeep')}</button>
        {:else if note.choice === 'open'}
          <button class="btn small" disabled={busy} use:tip={t('prs.review.keepHint')} onclick={() => choose('kept')}
            >{t('prs.review.keep')}</button
          >
        {/if}
        {#if note.choice === 'dropped'}
          <button class="btn small" disabled={busy} onclick={() => choose('open')}>{t('prs.review.takeBack')}</button>
        {:else}
          <button class="btn small" disabled={busy} use:tip={t('prs.review.dropHint')} onclick={() => choose('dropped')}
            ><Icon name="check" size={12} />{t('prs.review.drop')}</button
          >
        {/if}
      </footer>
    {/if}
  </article>
{/if}

<style>
  .thread {
    margin: 6px 12px 8px 44px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      inset 3px 0 0 var(--brain),
      inset 0 0 0 1px oklch(from var(--brain) l c h / 0.45);
    overflow: hidden;
    min-width: 0;
    font-family: var(--font, inherit);
    white-space: normal;
  }
  .thread:focus {
    outline: none;
  }
  .thread.current {
    box-shadow:
      inset 3px 0 0 var(--brain),
      inset 0 0 0 1px var(--brain),
      0 0 0 1px var(--brain);
  }
  .msg {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 3px 9px;
    padding: 9px 12px 7px 14px;
  }
  .av {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--brain);
    background: oklch(from var(--brain) l c h / 0.14);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.45);
  }
  .who {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 7px;
    min-width: 0;
    font-size: 12.5px;
  }
  .who b {
    font-weight: 620;
  }
  .tag {
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--brain);
    background: oklch(from var(--brain) l c h / 0.12);
  }
  .sev {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .sev.security {
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.1);
    box-shadow: none;
  }
  .sev.bug {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: none;
  }
  .sev.spec {
    color: var(--soft);
    background: var(--raise-2);
  }
  .flag {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    color: var(--muted);
  }
  .flag.kept {
    color: var(--run);
    font-weight: 600;
  }
  .place {
    color: var(--soft);
    font-size: 11.5px;
  }
  .fold {
    margin-left: auto;
  }
  .bd {
    grid-column: 2;
    color: var(--text-ai);
    min-width: 0;
    font-size: 13px;
  }
  .third :global(p:first-child) {
    margin-top: 0;
  }
  .third :global(p:last-child) {
    margin-bottom: 0;
  }
  .sugg {
    margin: 2px 12px 8px 45px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
  }
  .sh {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 6px 4px 10px;
    font-size: 11.5px;
    color: var(--muted);
    border-bottom: 1px solid var(--line);
  }
  .copy {
    margin-left: auto;
  }
  .code {
    margin: 0;
    padding: 4px 0;
    font-size: 12px;
    overflow-x: auto;
    background: var(--well);
  }
  .dl {
    display: flex;
    gap: 8px;
    padding: 0 10px;
    white-space: pre;
  }
  .dl.add {
    background: oklch(from var(--add) l c h / 0.12);
  }
  .dl.del {
    background: oklch(from var(--del) l c h / 0.12);
  }
  .dl .k {
    width: 10px;
    flex: none;
    color: var(--muted);
  }
  .pub {
    display: grid;
    gap: 6px;
    padding: 2px 12px 10px 45px;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    font: inherit;
    font-size: 13px;
    padding: 7px 9px;
    border-radius: 8px;
    border: none;
    color: var(--text);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  textarea:focus-visible {
    outline: 2px solid var(--brain);
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .acts {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  footer.acts {
    padding: 4px 12px 10px 45px;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .fail {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0 12px 8px 45px;
    padding: 6px 9px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .thread.folded {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 4px;
    background: transparent;
    box-shadow:
      inset 3px 0 0 oklch(from var(--brain) l c h / 0.5),
      inset 0 0 0 1px var(--line);
  }
  .unfold {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 4px 8px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
  }
  .unfold:hover {
    background: var(--raise);
  }
  .unfold:focus-visible {
    outline: 2px solid var(--brain);
  }
  .ts {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: 600;
    font-size: 11.5px;
    color: var(--run);
  }
  .thread.folded .fail {
    flex-basis: 100%;
    margin: 0;
  }
  .muted {
    color: var(--muted);
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

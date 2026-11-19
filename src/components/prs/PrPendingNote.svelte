<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { pendingKey, pendingPlace } from '../../lib/myreview';
  import { tip } from '../../lib/tip';
  import type { PendingNote } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';

  // A remark of the user's own review under its line in the tab « Fichiers » (« Ta relecture », ESC-494): « Toi », marked
  // « en attente · pas publiée » (its tag and its icon, read without the colour too), its text, « Modifier » and « Retirer ».
  // Stale (its line gone after a commit), it says so: it will go in the review's text as a comment of the pull request,
  // or the user takes it out. Nothing leaves from here: the review's « Publier » sends it.
  let {
    note,
    host,
    word,
    current = false,
    showPlace = false,
    edit,
    remove,
  }: {
    note: PendingNote;
    host: string;
    word: string;
    current?: boolean;
    showPlace?: boolean;
    /** Its text written again: null when done, else why not. */
    edit: (n: PendingNote, body: string) => Promise<string | null>;
    /** Taken out: null when done, else why not. */
    remove: (n: PendingNote) => Promise<string | null>;
  } = $props();

  let editing = $state(false);
  let text = $state('');
  let busy = $state(false);
  let failure = $state<string | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);

  const place = $derived(pendingPlace(note));

  function open() {
    text = note.body;
    failure = null;
    editing = true;
  }

  async function save() {
    if (!text.trim() || busy) return;
    busy = true;
    failure = await edit(note, text);
    busy = false;
    if (!failure) editing = false;
  }

  async function takeOut() {
    busy = true;
    failure = await remove(note);
    busy = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      void save();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      editing = false;
    }
  }

  $effect(() => {
    if (!editing || !box) return;
    untrack(() => box?.focus());
  });
</script>

<article
  class="thread mine"
  class:current
  class:stale={note.stale}
  data-thread-key={pendingKey(note)}
  data-pending={note.id}
  tabindex="-1"
  aria-label={t('prs.page.threads.label', { place })}
>
  <div class="msg">
    <span class="av" aria-hidden="true"><Icon name="edit" size={11} /></span>
    <div class="who">
      <b>{t('prs.page.mine.you')}</b>
      <span class="tag" use:tip={t('prs.page.mine.tagHint', { host })}><Icon name="clock" size={10} />{t('prs.page.mine.tag')}</span>
      {#if showPlace}<span class="mono place">{place}</span>{/if}
      {#if note.stale}<span class="flag" use:tip={t('prs.page.mine.staleHint', { word })}>{t('prs.page.mine.stale')}</span>{/if}
    </div>
    {#if !editing}<div class="bd"><Markdown text={note.body} /></div>{/if}
  </div>
  {#if note.stale && !editing}<p class="hint stale-hint">{t('prs.page.mine.staleHint', { word })}</p>{/if}
  {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
  {#if editing}
    <div class="pub">
      <textarea bind:this={box} bind:value={text} rows="3" aria-label={t('prs.page.mine.edit')} onkeydown={onKey} disabled={busy}
      ></textarea>
      <div class="acts">
        <button class="btn small primary" disabled={busy || !text.trim()} onclick={save}>{t('prs.page.mine.save')}</button>
        <button class="btn small" disabled={busy} onclick={() => (editing = false)}>{t('prs.page.mine.cancel')}</button>
      </div>
    </div>
  {:else}
    <footer class="acts">
      <button class="btn small" disabled={busy} onclick={open}><Icon name="edit" size={12} />{t('prs.page.mine.edit')}</button>
      <button class="btn small" disabled={busy} onclick={takeOut}><Icon name="close" size={12} />{t('prs.page.mine.remove')}</button>
    </footer>
  {/if}
</article>

<style>
  .thread {
    margin: 6px 12px 8px 44px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      inset 3px 0 0 var(--wait),
      inset 0 0 0 1px oklch(from var(--wait) l c h / 0.45);
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
      inset 3px 0 0 var(--wait),
      inset 0 0 0 1px var(--wait),
      0 0 0 1px var(--wait);
  }
  .thread.stale {
    box-shadow:
      inset 3px 0 0 oklch(from var(--wait) l c h / 0.5),
      inset 0 0 0 1px var(--line-2);
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
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px oklch(from var(--wait) l c h / 0.45);
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
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--wait);
    background: var(--wait-bg);
  }
  .flag {
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    padding: 1px 6px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .place {
    color: var(--soft);
    font-size: 11.5px;
  }
  .bd {
    grid-column: 2;
    min-width: 0;
    font-size: 13px;
  }
  .bd :global(p:first-child) {
    margin-top: 0;
  }
  .bd :global(p:last-child) {
    margin-bottom: 0;
  }
  .hint {
    margin: 0 12px 6px 45px;
    color: var(--muted);
    font-size: 11.5px;
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
    outline: 2px solid var(--wait);
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
</style>

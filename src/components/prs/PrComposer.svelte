<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import Markdown from '../conv/Markdown.svelte';

  // A comment to write on a pull request (ESC-474): a line, a thread's answer or the conversation's. Its text is a draft
  // kept in `app.prDrafts[draftKey]` until it is sent: nothing is lost by changing tabs, closing it, or reading another
  // pull request. « Aperçu » renders its Markdown as the app renders the rest. Nothing goes without the click on
  // `submit` (or Ctrl+Entrée), which `onSend` makes; `busy` while it goes.
  let {
    draftKey,
    label,
    submit,
    host,
    busy = false,
    autofocus = false,
    onSend,
    onCancel,
  }: {
    draftKey: string;
    label: string;
    submit: string;
    host: string;
    busy?: boolean;
    autofocus?: boolean;
    onSend: (body: string) => void;
    onCancel?: () => void;
  } = $props();

  let preview = $state(false);
  let box = $state<HTMLTextAreaElement | null>(null);
  const text = $derived(app.prDrafts[draftKey] ?? '');
  const empty = $derived(!text.trim());

  function set(v: string) {
    if (v) app.prDrafts[draftKey] = v;
    else delete app.prDrafts[draftKey];
  }

  function go() {
    if (empty || busy) return;
    onSend(text);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      go();
    } else if (e.key === 'Escape' && onCancel) {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    }
  }

  $effect(() => {
    if (!autofocus || !box) return;
    untrack(() => box?.focus());
  });
</script>

<form
  class="composer"
  aria-label={label}
  onsubmit={(e) => {
    e.preventDefault();
    go();
  }}
>
  <div class="modes" role="tablist" aria-label={label}>
    <button type="button" role="tab" aria-selected={!preview} onclick={() => (preview = false)}>{t('prs.page.threads.write')}</button>
    <button type="button" role="tab" aria-selected={preview} onclick={() => (preview = true)}>{t('prs.page.threads.preview')}</button>
    {#if !empty && !busy}<span class="kept">{t('prs.page.threads.draftKept')}</span>{/if}
  </div>
  {#if preview}
    <div class="preview" role="tabpanel">
      {#if empty}<p class="muted">{t('prs.page.threads.emptyPreview')}</p>{:else}<Markdown {text} />{/if}
    </div>
  {:else}
    <textarea
      bind:this={box}
      class="field"
      rows="4"
      aria-label={label}
      placeholder={t('prs.page.threads.placeholder')}
      value={text}
      disabled={busy}
      oninput={(e) => set(e.currentTarget.value)}
      onkeydown={onKey}></textarea>
  {/if}
  <div class="foot">
    <span class="hint">{t('prs.page.threads.publishedAs', { host })}</span>
    {#if onCancel}<button type="button" class="btn" onclick={onCancel}>{t('prs.page.threads.cancel')}</button>{/if}
    <button type="submit" class="btn primary" disabled={empty || busy}>{busy ? t('prs.page.threads.sending') : submit}</button>
  </div>
</form>

<style>
  .composer {
    display: grid;
    gap: 6px;
    padding: 8px 10px 10px;
    min-width: 0;
  }
  .modes {
    display: flex;
    align-items: center;
    gap: 2px;
    font-size: 12px;
  }
  .modes button {
    all: unset;
    cursor: pointer;
    padding: 3px 8px;
    border-radius: 6px;
    color: var(--muted);
  }
  .modes button[aria-selected='true'] {
    color: var(--text);
    background: var(--raise-2);
  }
  .modes button:focus-visible {
    outline: 2px solid var(--brain);
  }
  .kept {
    margin-left: auto;
    color: var(--muted);
    font-size: 11px;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: 76px;
    resize: vertical;
    font-size: 13px;
  }
  .preview {
    min-height: 76px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 13px;
    min-width: 0;
  }
  .preview :global(p:first-child) {
    margin-top: 0;
  }
  .preview :global(p:last-child) {
    margin-bottom: 0;
  }
  .muted {
    margin: 0;
    color: var(--muted);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .hint {
    flex: 1;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
</style>

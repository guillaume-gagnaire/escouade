<script lang="ts">
  // The files of a request (MO-5, `docs/MOBILE.md` § 3.6): its images in a gallery (a touch opens the viewer), a PDF
  // read page by page (pdf.js in the WebView, without its scripts, as on the desktop), a Markdown or a text read in a
  // sheet; any other downloaded and opened by the system's app that reads it, never a program.
  import Icon from '$shared/components/Icon.svelte';
  import { fSize } from '$shared/lib/format';
  import { t } from '$shared/lib/i18n';
  import { opensOutside } from '$shared/lib/runnable';
  import type { AskFile, WindowAsk } from '$shared/lib/types';
  import { failText, fileBytes } from '../lib/act';
  import { showFile, showImages, showPdf } from '../lib/feed.svelte';
  import { openWithSystem } from '../lib/voice';
  import AskImg from './AskImg.svelte';

  let { space, ask }: { space: string; ask: WindowAsk } = $props();

  const images = $derived(ask.files.filter((f) => f.kind === 'image' && f.option === null));
  const others = $derived(ask.files.filter((f) => f.kind !== 'image' && f.option === null));
  let said = $state<string | null>(null);

  const isText = (f: AskFile) => f.kind === 'markdown' || f.kind === 'csv' || /\.(md|txt|log|json|csv)$/i.test(f.name);
  const isPdf = (f: AskFile) => f.kind === 'pdf' || f.mediaType === 'application/pdf';

  async function open(f: AskFile) {
    said = null;
    if (isPdf(f)) return showPdf(space, ask, f);
    if (isText(f)) return showFile(space, ask, f);
    if (opensOutside(f.name)) {
      said = t('mobile.files.program', { name: f.name });
      return;
    }
    said = t('mobile.files.opening', { name: f.name });
    try {
      await openWithSystem(f.name, f.mediaType, await fileBytes(space, ask, f));
      said = null;
    } catch (e) {
      said = t('mobile.files.failed', { name: f.name, error: failText(e) });
    }
  }
</script>

{#if ask.filesErasedAt}
  <p class="note">{t('inbox.files.erased')}</p>
{:else if images.length || others.length}
  {#if images.length}
    <div class="gal">
      {#each images as f, i (f.file)}
        <button
          type="button"
          class="th"
          aria-label={t('inbox.files.image', { name: f.name })}
          onclick={() => showImages(space, ask, images, i)}
        >
          <AskImg {space} {ask} file={f} />
        </button>
      {/each}
    </div>
  {/if}
  {#each others as f (f.file)}
    <button type="button" class="file" onclick={() => open(f)}>
      <Icon name={isPdf(f) ? 'page' : isText(f) ? 'file' : 'download'} size={18} />
      <span class="nm">{f.name}</span>
      <span class="sz">{fSize(f.size)}</span>
      <span class="go">{isPdf(f) || isText(f) ? t('mobile.files.read') : t('mobile.files.open')}</span>
    </button>
  {/each}
  {#if said}<p class="note" role="status">{said}</p>{/if}
{/if}

<style>
  .gal {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .th {
    flex: none;
    width: 120px;
    height: 80px;
    padding: 0;
    border: 0;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: transparent;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--target);
    padding: 6px 12px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font: inherit;
    text-align: left;
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-weight: 600;
    font-size: 14px;
  }
  .sz {
    font-size: var(--small);
    color: var(--muted);
  }
  .go {
    font-size: var(--small);
    font-weight: 650;
    color: var(--teal-ink);
  }
  .note {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
</style>

<script lang="ts">
  import { fileCategory } from '../../lib/attachments';
  import { fSize } from '../../lib/format';
  import type { ViewedImage } from '../../lib/gallery';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { inbox } from '../../lib/inbox/inbox.svelte';
  import { api } from '../../lib/ipc';
  import { opensOutside } from '../../lib/runnable';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { AskFile, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import ImageViewer from '../conv/ImageViewer.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import AskThumb from './AskThumb.svelte';

  // The files of a request (docs/INBOX.md § 4.3, `b-pdf.png`): a PDF « Lire » in the reader beside the text (`reading`), an
  // image as a thumbnail that opens the viewer (the request's images in a gallery, its zoom, « Copier », « Enregistrer
  // sous… »), a Markdown file drawn as the conversation draws one (sanitized, nothing from the web without a click), any
  // other file (a Word document included, on this line) with its name, its size, « Ouvrir » (never a program, a script
  // nor a page: « Afficher dans le dossier » then) and « Enregistrer sous… ». A file is data: never run, never read
  // from another path than the core's copy.
  let { ask, files, reading = $bindable<string | null>(null) }: { ask: WindowAsk; files: AskFile[]; reading?: string | null } = $props();

  /** What « Ouvrir » would hand to a program: the core refuses it, the line offers the folder instead. */
  const runs = (f: AskFile) => opensOutside(f.name);

  const ICON: Record<string, IconName> = { archive: 'archive', sheet: 'sheet', document: 'file', code: 'code', generic: 'file' };
  const iconOf = (f: AskFile): IconName => (f.kind === 'pdf' ? 'file' : (ICON[fileCategory(f.name)] ?? 'file'));

  const images = $derived(files.filter((f) => f.kind === 'image'));
  const markdowns = $derived(files.filter((f) => f.kind === 'markdown'));
  let viewing = $state<{ images: ViewedImage[]; index: number } | null>(null);
  let dims = $state<Record<string, string>>({});
  let texts = $state<Record<string, string>>({});

  // The size of each image, read once it is loaded (a line of its chip: « 1280 × 560 · 84 Ko »).
  $effect(() => {
    for (const f of images) {
      if (dims[f.file]) continue;
      void inbox.imageUrl(ask, f).then((url) => {
        const img = new Image();
        img.onload = () => (dims = { ...dims, [f.file]: t('inbox.files.dims', { w: img.naturalWidth, h: img.naturalHeight }) });
        img.src = url;
      });
    }
  });

  // A Markdown file's text, read from the core's copy.
  $effect(() => {
    for (const f of markdowns) {
      if (f.file in texts) continue;
      void inbox
        .fileBytes(ask, f)
        .then((b) => (texts = { ...texts, [f.file]: new TextDecoder().decode(b) }))
        .catch(() => {});
    }
  });

  /** A file looked at, for the usage statistics: its sort only. */
  const looked = (f: AskFile) => api.inboxTrack('attachment', { kind: f.kind }).catch(() => {});

  async function view(f: AskFile) {
    void looked(f);
    const list = await Promise.all(
      images.map(async (x) => ({
        name: x.name,
        url: await inbox.imageUrl(ask, x),
        source: { kind: 'ask' as const, id: ask.id, file: x.file },
      })),
    );
    viewing = { images: list, index: Math.max(0, images.indexOf(f)) };
  }

  async function saveAs(f: AskFile) {
    if (await app.run(api.inboxSaveFile(ask.id, f.file))) app.toast(t('inbox.files.saved', { name: f.name }), 'ok');
  }
  function open(f: AskFile, reveal: boolean) {
    if (reveal) return void app.run(api.inboxOpenFile(ask.id, f.file, true));
    void looked(f);
    void app.openFile(api.inboxOpenFile(ask.id, f.file, false), () => api.inboxOpenFile(ask.id, f.file, true));
  }
</script>

{#if ask.filesErasedAt !== null && files.length}
  <p class="erased">{t('inbox.files.erased')}</p>
{:else if files.length}
  <ul class="atts" aria-label={t('inbox.files.label')} data-testid="ask-files">
    {#each files as f (f.file)}
      <li class="att" class:on={reading === f.file} data-kind={f.kind}>
        {#if f.kind === 'image'}
          <button class="th img" aria-label={t('inbox.files.image', { name: f.name })} onclick={() => view(f)} data-testid="ask-image">
            <AskThumb {ask} file={f} alt="" big />
          </button>
        {:else}
          <span class="th"><Icon name={iconOf(f)} /></span>
        {/if}
        <span class="nm ell" use:tip={f.name}>{f.name}</span>
        <span class="mt">
          {#if f.kind === 'image' && dims[f.file]}<span>{dims[f.file]}</span><span aria-hidden="true">·</span>{/if}
          {#if f.kind === 'pdf' && inbox.pages[`${ask.id}/${f.file}`]}<span
              >{t('inbox.files.pages', { count: inbox.pages[`${ask.id}/${f.file}`] })}</span
            ><span aria-hidden="true">·</span>{/if}
          <span>{fSize(f.size)}</span>
          {#if f.kind === 'pdf'}
            <span aria-hidden="true">·</span>
            <button class="a" aria-pressed={reading === f.file} onclick={() => ((reading = f.file), looked(f))} data-testid="ask-pdf-read"
              >{t('inbox.files.read')}</button
            >
          {:else if f.kind === 'markdown' || f.kind === 'image'}
            <!-- Drawn in place: nothing to open. -->
          {:else if runs(f)}
            <span aria-hidden="true">·</span>
            <button class="a" onclick={() => open(f, true)}>{t('inbox.files.reveal')}</button>
          {:else}
            <span aria-hidden="true">·</span>
            <button class="a" onclick={() => open(f, false)} data-testid="ask-open">{t('inbox.files.open')}</button>
          {/if}
          <span aria-hidden="true">·</span>
          <button class="a" onclick={() => saveAs(f)} data-testid="ask-save">{t('inbox.files.saveAs')}</button>
        </span>
      </li>
    {/each}
  </ul>
  {#each markdowns as f (f.file)}
    {#if texts[f.file] !== undefined}
      <section class="md-file" aria-label={f.name}>
        <h3 class="mono">{f.name}</h3>
        <Markdown text={texts[f.file]} />
      </section>
    {/if}
  {/each}
{/if}

{#if viewing}
  <ImageViewer images={viewing.images} bind:index={viewing.index} onclose={() => (viewing = null)} />
{/if}

<style>
  .atts {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    min-width: 0;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .att {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 2px 10px;
    min-width: 0;
    max-width: 100%;
    padding: 7px 10px 7px 7px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .att.on {
    background: var(--sel);
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
  }
  .th {
    grid-row: 1 / 3;
    display: grid;
    place-items: center;
    width: 44px;
    height: 34px;
    border-radius: 5px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .th.img {
    padding: 0;
    border: none;
    background: none;
    box-shadow: none;
    cursor: zoom-in;
  }
  .th.img:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .nm {
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }
  .mt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 8px;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .a {
    padding: 0;
    border: none;
    background: none;
    color: var(--soft);
    font: 550 11.5px/1.4 var(--font);
    cursor: pointer;
  }
  .a:hover,
  .a[aria-pressed='true'] {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .md-file {
    display: grid;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
  }
  /* A file's own title, set to the size of the box's text: the file is shown, not the page's heading. */
  .md-file :global(h1),
  .md-file :global(h2) {
    margin: 4px 0;
    font-size: 14px;
  }
  .md-file h3 {
    margin: 0;
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
  }
  .erased {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .ell {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>

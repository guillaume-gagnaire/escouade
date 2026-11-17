<script lang="ts">
  import { untrack } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';
  import { inbox } from '../../lib/inbox/inbox.svelte';
  import type { PdfDoc } from '../../lib/previews/pdf';
  import { tip } from '../../lib/tip';
  import type { AskFile, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';

  // A PDF of a request read in place (direction B, `b-pdf.png`): its bar (its name, « 3 sur 12 » where a page typed
  // leads, the zoom, the search, « Plein écran »), the thumbnails of its pages, the page drawn white in both themes.
  // pdf.js is loaded with the first PDF (`lib/previews/pdf.ts`): no script of the PDF, nothing from the web.
  let { ask, file }: { ask: Pick<WindowAsk, 'id'>; file: AskFile } = $props();

  let doc = $state.raw<PdfDoc | null>(null);
  let error = $state('');
  let page = $state(1);
  let typed = $state('1');
  /** The page's width as a share of the room (1: the room's width, fitted). */
  let zoom = $state(1);
  let full = $state(false);
  let query = $state('');
  let hits = $state<number[] | null>(null);
  let room = $state(0);
  let canvas = $state<HTMLCanvasElement>();
  let root = $state<HTMLElement>();
  let thumbs = $state<HTMLCanvasElement[]>([]);
  /** The thumbnails drawn, the first pages only: a long PDF is paged through by its number. */
  const THUMBS = 30;

  $effect(() => {
    const f = file;
    const a = ask;
    let live = true;
    let opened: PdfDoc | null = null;
    doc = null;
    error = '';
    page = 1;
    typed = '1';
    void (async () => {
      try {
        const [{ openPdf, PdfPasswordError }, bytes] = await Promise.all([import('../../lib/previews/pdf'), inbox.fileBytes(a, f)]);
        try {
          opened = await openPdf(bytes);
        } catch (e) {
          if (e instanceof PdfPasswordError) {
            if (live) error = t('inbox.files.password');
            return;
          }
          throw e;
        }
        if (live) {
          doc = opened;
          inbox.pages[`${a.id}/${f.file}`] = opened.pages;
        } else opened.dispose();
      } catch (e) {
        if (live) error = t('inbox.files.failed', { name: f.name, error: e instanceof Error ? e.message : String(e) });
      }
    })();
    return () => {
      live = false;
      opened?.dispose();
    };
  });

  // The page drawn at the room's width, as the room, the zoom or the page change.
  $effect(() => {
    const d = doc;
    const c = canvas;
    const n = page;
    const w = Math.max(120, Math.floor((room - 28) * zoom));
    if (!d || !c || !room) return;
    untrack(() => d.draw(n, c, w).catch(() => {}));
  });

  // The thumbnails, one after the other.
  $effect(() => {
    const d = doc;
    const list = thumbs;
    if (!d) return;
    let live = true;
    void (async () => {
      for (let i = 0; i < Math.min(d.pages, THUMBS); i++) {
        const c = list[i];
        if (!live) return;
        if (c) await d.draw(i + 1, c, 48).catch(() => {});
      }
    })();
    return () => {
      live = false;
    };
  });

  function go(n: number) {
    if (!doc) return;
    page = Math.max(1, Math.min(doc.pages, n));
    typed = String(page);
  }

  function onTyped() {
    const n = Math.round(Number(typed));
    if (doc && n >= 1 && n <= doc.pages) go(n);
    else typed = String(page);
  }

  async function search() {
    if (!doc || !query.trim()) {
      hits = null;
      return;
    }
    hits = await doc.find(query);
    const next = hits.find((n) => n > page) ?? hits[0];
    if (next) go(next);
  }

  function onKey(e: KeyboardEvent) {
    const field = (e.target as HTMLElement).closest('input');
    if (e.key === 'Escape' && full) {
      e.preventDefault();
      e.stopPropagation();
      full = false;
      return;
    }
    if (field) return;
    if (e.key === 'ArrowRight' || e.key === 'PageDown') go(page + 1);
    else if (e.key === 'ArrowLeft' || e.key === 'PageUp') go(page - 1);
    else if (e.key === 'Home') go(1);
    else if (e.key === 'End' && doc) go(doc.pages);
    else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<!-- Full screen, it holds the focus until Échap or « Plein écran » again. -->
<section
  class="pdfv"
  class:full
  aria-label={t('inbox.pdf.reader', { name: file.name })}
  bind:this={root}
  onkeydown={onKey}
  data-testid="pdf-reader"
  {@attach (el) => {
    if (!full) return;
    const held = trapFocus(el);
    return () => held.destroy();
  }}
  role={full ? 'dialog' : 'region'}
  aria-modal={full ? 'true' : undefined}
>
  <div class="bar">
    <Icon name="file" size={13} />
    <b class="ell">{file.name}</b>
    <span class="sp"></span>
    {#if doc}
      <button class="ib" aria-label={t('inbox.pdf.prev')} use:tip={t('inbox.pdf.prev')} disabled={page <= 1} onclick={() => go(page - 1)}
        ><Icon name="left" size={13} /></button
      >
      <label class="pg">
        <span class="vh">{t('inbox.pdf.page')}</span>
        <input class="pgn" inputmode="numeric" bind:value={typed} onchange={onTyped} data-testid="pdf-page" />
        <span>{t('inbox.pdf.of', { count: doc.pages })}</span>
      </label>
      <button
        class="ib"
        aria-label={t('inbox.pdf.next')}
        use:tip={t('inbox.pdf.next')}
        disabled={page >= doc.pages}
        onclick={() => go(page + 1)}
        data-testid="pdf-next"><Icon name="right" size={13} /></button
      >
      <span class="sep"></span>
      <button
        class="ib"
        aria-label={t('inbox.pdf.zoomOut')}
        use:tip={t('inbox.pdf.zoomOut')}
        disabled={zoom <= 0.5}
        onclick={() => (zoom = Math.max(0.5, zoom / 1.25))}>−</button
      >
      <button
        class="ib"
        aria-label={t('inbox.pdf.zoomIn')}
        use:tip={t('inbox.pdf.zoomIn')}
        disabled={zoom >= 3}
        onclick={() => (zoom = Math.min(3, zoom * 1.25))}><Icon name="zoom" size={13} /></button
      >
      <input
        class="find"
        type="search"
        placeholder={t('inbox.pdf.findShort')}
        aria-label={t('inbox.pdf.find')}
        bind:value={query}
        onkeydown={(e) => e.key === 'Enter' && search()}
      />
      {#if hits}<span class="found" aria-live="polite"
          >{hits.length ? t('inbox.pdf.found', { count: hits.length }) : t('inbox.pdf.notFound')}</span
        >{/if}
    {/if}
    <button
      class="ib"
      aria-label={full ? t('inbox.pdf.close') : t('inbox.pdf.full')}
      aria-pressed={full}
      use:tip={full ? t('inbox.pdf.close') : t('inbox.pdf.full')}
      onclick={() => (full = !full)}
      data-testid="pdf-full"><Icon name={full ? 'close' : 'expand'} size={13} /></button
    >
  </div>
  <div class="in" class:nothumbs={!doc || doc.pages < 2}>
    {#if doc && doc.pages > 1}
      <nav class="thumbs" aria-label={t('inbox.pdf.thumbs')}>
        {#each { length: Math.min(doc.pages, THUMBS) } as _, i (i)}
          <button class="th" aria-current={page === i + 1} aria-label={t('inbox.pdf.thumb', { n: i + 1 })} onclick={() => go(i + 1)}>
            <canvas bind:this={thumbs[i]}></canvas>
            <span>{i + 1}</span>
          </button>
        {/each}
      </nav>
    {/if}
    <div class="canvas" bind:clientWidth={room} tabindex="-1">
      {#if error}
        <p class="err" role="alert">{error}</p>
      {:else if !doc}
        <p class="wait" role="status">{t('inbox.files.loading', { name: file.name })}</p>
      {:else}
        <canvas class="page" bind:this={canvas} data-testid="pdf-canvas" data-page={page}></canvas>
      {/if}
    </div>
  </div>
</section>

<style>
  .pdfv {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
  }
  .pdfv.full {
    position: fixed;
    inset: 16px;
    z-index: 60;
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 6px 8px 6px 12px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
  }
  .bar b {
    color: var(--text);
    font-weight: 600;
  }
  .sp {
    flex: 1;
  }
  .sep {
    width: 1px;
    height: 16px;
    background: var(--line-2);
  }
  .ib {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    flex: none;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--soft);
    font: 600 14px/1 var(--font);
    cursor: pointer;
  }
  .ib:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .ib:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .pg {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .pgn {
    width: 34px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: 600 12px/1 var(--font);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .find {
    width: 120px;
    height: 24px;
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: 12px var(--font);
  }
  .found {
    white-space: nowrap;
  }
  .in {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr);
    min-height: 0;
  }
  .in.nothumbs {
    grid-template-columns: minmax(0, 1fr);
  }
  .thumbs {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 10px 8px;
    border-right: 1px solid var(--line);
    overflow-y: auto;
  }
  .th {
    display: grid;
    gap: 3px;
    padding: 0;
    border: none;
    background: none;
    color: var(--dim);
    font: 10.5px var(--font);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .th canvas {
    width: 100%;
    border-radius: 3px;
    background: #fff;
    box-shadow: 0 0 0 1px var(--line-2);
  }
  .th[aria-current='true'] canvas {
    box-shadow: 0 0 0 2px var(--p-ink);
  }
  .th[aria-current='true'] {
    color: var(--text);
  }
  .canvas {
    display: grid;
    justify-items: center;
    align-content: start;
    min-width: 0;
    padding: 14px;
    overflow: auto;
    outline: none;
  }
  .page {
    max-width: none;
    border-radius: 2px;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
  }
  .err,
  .wait {
    margin: 24px 0;
    color: var(--muted);
    font-size: 12.5px;
    text-align: center;
  }
  .err {
    color: var(--del);
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  @media (max-width: 1100px) {
    .find,
    .found {
      display: none;
    }
  }
</style>

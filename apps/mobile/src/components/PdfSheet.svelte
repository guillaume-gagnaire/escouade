<script lang="ts">
  // A PDF of a request, read page by page (MO-5, `docs/MOBILE.md` § 3.6): pdf.js in the WebView, without the PDF's
  // scripts nor anything from the web (`src/lib/previews/pdf.ts`, the desktop's reader), loaded when the first PDF
  // opens. Each page is drawn at the screen's width when it comes near the view; the bytes stay in memory alone.
  import { onDestroy } from 'svelte';
  import { t } from '$shared/lib/i18n';
  import type { PdfDoc } from '$shared/lib/previews/pdf';
  import { failText, fileBytes } from '../lib/act';
  import { overlay } from '../lib/feed.svelte';
  import { closeSheet } from '../lib/nav.svelte';
  import Screen from './Screen.svelte';

  const p = overlay.pdf!;
  let doc = $state<PdfDoc | null>(null);
  let error = $state<string | null>(null);
  let pages = $state<HTMLElement>();

  $effect(() => {
    let live = true;
    Promise.all([import('$shared/lib/previews/pdf'), fileBytes(p.space, p.ask, p.file)])
      .then(async ([{ openPdf, PdfPasswordError }, bytes]) => {
        try {
          const d = await openPdf(bytes);
          if (live) doc = d;
          else d.dispose();
        } catch (e) {
          if (live) error = e instanceof PdfPasswordError ? t('inbox.files.password') : failText(e);
        }
      })
      .catch((e) => live && (error = failText(e)));
    return () => (live = false);
  });

  // Each page drawn when it nears the view, at the width it has.
  $effect(() => {
    if (!doc || !pages) return;
    const d = doc;
    const near = new IntersectionObserver(
      (entries) => {
        for (const e of entries) {
          if (!e.isIntersecting) continue;
          const c = e.target as HTMLCanvasElement;
          near.unobserve(c);
          void d.draw(Number(c.dataset.page), c, c.parentElement?.clientWidth ?? 320).catch(() => {});
        }
      },
      { rootMargin: '600px 0px' },
    );
    for (const c of pages.querySelectorAll('canvas')) near.observe(c);
    return () => near.disconnect();
  });

  onDestroy(() => doc?.dispose());
</script>

<Screen
  title={p.file.name}
  sub={doc ? t('mobile.pdf.pages', { count: doc.pages }) : ''}
  label={t('inbox.pdf.reader', { name: p.file.name })}
  onClose={closeSheet}
>
  {#if error}
    <p class="note" role="alert">{t('inbox.files.failed', { name: p.file.name, error })}</p>
  {:else if !doc}
    <p class="note" role="status">{t('mobile.pdf.loading')}</p>
  {:else}
    <div class="pages" bind:this={pages} data-testid="pdf-pages">
      {#each Array.from({ length: doc.pages }, (_, i) => i + 1) as n (n)}
        <div class="pg"><canvas data-page={n} aria-label={`${t('inbox.pdf.page')} ${n}`}></canvas></div>
      {/each}
    </div>
  {/if}
</Screen>

<style>
  .pages {
    display: grid;
    gap: 12px;
    padding-top: 6px;
  }
  .pg {
    min-height: 120px;
    border-radius: 6px;
    overflow: hidden;
    background: #fff;
    box-shadow: 0 0 0 1px var(--line-2);
  }
  canvas {
    display: block;
    width: 100%;
  }
  .note {
    font-size: 14px;
    color: var(--muted);
  }
</style>

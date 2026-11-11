<script lang="ts">
  import { galleryOf } from '../../lib/gallery';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { handleMarkdownClick, highlightWithin, renderMarkdown } from '../../lib/markdown';
  import ImageViewer from './ImageViewer.svelte';

  // `agentId`: the agent whose message it is, which reads the images it shows by a local path (ESC-171); without one,
  // such an image shows its text alone.
  let { text, streaming = false, agentId = null }: { text: string; streaming?: boolean; agentId?: string | null } = $props();

  let el = $state<HTMLDivElement>();
  let html = $state('');
  let scheduled = false;
  let viewing = $state<ReturnType<typeof galleryOf> | null>(null);

  // While streaming, re-render at most once per frame; highlight code once the text is final.
  $effect(() => {
    const t = text;
    if (!streaming) {
      html = renderMarkdown(t);
      return;
    }
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      html = renderMarkdown(text, false);
    });
  });

  $effect(() => {
    void html;
    if (!streaming && el) queueMicrotask(() => el && highlightWithin(el));
  });

  // The images of a local path: read by the core (the agent's fence, a format of image, a size at most), once written.
  $effect(() => {
    void html;
    const id = agentId;
    if (streaming || !el) return;
    queueMicrotask(() => {
      for (const off of el?.querySelectorAll<HTMLElement>('[data-local-src]:not([data-read])') ?? []) {
        off.dataset.read = '1';
        const src = off.dataset.localSrc ?? '';
        const missing = () => (off.title = t('conv.sent.imageMissing', { src }));
        if (!id) {
          missing();
          continue;
        }
        api.agentLocalImage(id, src).then((url) => {
          if (!url) return missing();
          // The image in place of its text: an `<img>` of the `data:` URL the core read.
          const img = document.createElement('img');
          img.src = url;
          img.alt = off.textContent ?? '';
          img.dataset.localSrc = src;
          img.dataset.agent = id;
          img.dataset.view = '';
          off.replaceWith(img);
        }, missing);
      }
    });
  });

  /** « Afficher l'image » of an image of the web: the core downloads it, now only, and it takes the place of its label. */
  async function showRemote(button: HTMLElement) {
    const label = button.closest<HTMLElement>('[data-remote-src]');
    const src = label?.dataset.remoteSrc;
    if (!label || !src || button.hasAttribute('disabled')) return;
    button.setAttribute('disabled', '');
    const url = await app.run(api.remoteImage(src));
    if (!url) return button.removeAttribute('disabled');
    const img = document.createElement('img');
    img.src = url;
    img.alt = label.firstChild?.textContent?.split(' · ')[0] ?? '';
    img.dataset.remoteSrc = src;
    img.dataset.view = '';
    label.replaceWith(img);
  }

  /** A click on an image shown opens it full size, among the others of its conversation; the rest is the copy buttons'. */
  function onclick(e: MouseEvent) {
    const show = (e.target as HTMLElement).closest<HTMLElement>('[data-show-img]');
    if (show) {
      e.preventDefault();
      void showRemote(show);
      return;
    }
    const img = (e.target as HTMLElement).closest('img');
    if (img?.src && img.complete && img.naturalWidth > 0) {
      viewing = galleryOf(img);
      return;
    }
    handleMarkdownClick(e);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="md" bind:this={el} {onclick}>{@html html}</div>

{#if viewing}
  <ImageViewer images={viewing.images} bind:index={viewing.index} onclose={() => (viewing = null)} />
{/if}

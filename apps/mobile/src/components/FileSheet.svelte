<script lang="ts">
  // A Markdown or a text of a request, read in a sheet (MO-5): Markdown rendered (sanitized, as the desktop renders
  // it), any other text as it is.
  import { t } from '$shared/lib/i18n';
  import { renderMarkdown } from '$shared/lib/markdown';
  import { failText, fileBytes } from '../lib/act';
  import { overlay } from '../lib/feed.svelte';
  import { closeSheet } from '../lib/nav.svelte';
  import Screen from './Screen.svelte';

  const f = overlay.file!;
  let text = $state<string | null>(null);
  let error = $state<string | null>(null);
  const md = /\.(md|markdown)$/i.test(f.file.name) || f.file.kind === 'markdown';

  $effect(() => {
    fileBytes(f.space, f.ask, f.file)
      .then((b) => (text = new TextDecoder().decode(b)))
      .catch((e) => (error = failText(e)));
  });
</script>

<Screen title={f.file.name} label={f.file.name} onClose={closeSheet}>
  {#if error}
    <p class="note" role="alert">{t('inbox.files.failed', { name: f.file.name, error })}</p>
  {:else if text === null}
    <p class="note" role="status">{t('inbox.files.loading', { name: f.file.name })}</p>
  {:else if md}
    <div class="md doc">
      <!-- eslint-disable-next-line svelte/no-at-html-tags -- renderMarkdown sanitizes (DOMPurify) -->
      {@html renderMarkdown(text)}
    </div>
  {:else}
    <pre class="doc">{text}</pre>
  {/if}
</Screen>

<style>
  .doc {
    font-size: 15px;
    overflow-wrap: anywhere;
  }
  pre.doc {
    white-space: pre-wrap;
    font-family: var(--mono);
    font-size: 13px;
  }
  .note {
    font-size: 14px;
    color: var(--muted);
  }
</style>

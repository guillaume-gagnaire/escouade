<script lang="ts">
  import { t } from '../../lib/i18n';

  // A tool's output drawn a slice at a time (ESC-290): a text of megabytes (a file read whole, a line without breaks)
  // is laid out in one go by the page, which the window waits for, and which WebKitGTK may not survive.
  let { text, class: cls = '' }: { text: string; class?: string } = $props();

  const SLICE = 20_000;
  let shown = $state(SLICE);
  const rest = $derived(Math.max(0, text.length - shown));
</script>

<pre class={cls}>{rest ? text.slice(0, shown) : text}{#if rest}<button class="more" onclick={() => (shown += SLICE)}
      >{t('conv.tool.more', { count: rest })}</button
    >{/if}</pre>

<style>
  .more {
    display: block;
    margin-top: 8px;
    padding: 2px 8px;
    border-radius: 6px;
    font-family: var(--font);
    font-size: 11.5px;
    color: var(--soft);
    background: var(--hover);
  }
  .more:hover {
    color: var(--text);
  }
</style>

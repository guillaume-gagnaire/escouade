<script lang="ts">
  // Un message de la conversation : le rendu Markdown d'Escouade (`src/lib/markdown.ts`, marked + DOMPurify + shiki),
  // ses diffs (`src/lib/diff.ts` et `DiffView.svelte`, repris tels quels), une sortie de commande, une autorisation.
  import { onMount } from 'svelte';
  import { renderMarkdown, highlightWithin, handleMarkdownClick } from '../../../src/lib/markdown';
  import { parseUnifiedDiff, fileLines } from '../../../src/lib/diff';
  import DiffView from '../../../src/components/DiffView.svelte';
  import { t } from '../../../src/lib/i18n';
  import type { MobileItem } from './server';
  import { mobile } from './texts';

  let { item, onAnswer }: { item: MobileItem; onAnswer?: (decision: 'allow' | 'deny') => void } = $props();
  let box = $state<HTMLElement>();

  onMount(() => {
    if (item.kind === 'text' && box) highlightWithin(box);
  });
</script>

{#if item.kind === 'user'}
  <div class="msg user">{item.text}</div>
{:else if item.kind === 'text'}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="msg md" bind:this={box} onclick={handleMarkdownClick}>{@html renderMarkdown(item.text)}</div>
{:else if item.kind === 'tool' && item.name === 'Bash'}
  <div class="msg tool">
    <div class="tool-head"><span class="tool-name">Bash</span> <code>{item.input.command}</code></div>
    <pre class="out">{item.output}</pre>
  </div>
{:else if item.kind === 'tool'}
  <div class="msg tool">
    <div class="tool-head"><span class="tool-name">Edit</span> <code>{item.input.file_path}</code></div>
    {#each parseUnifiedDiff(item.diff ?? '') as file (file.path)}
      <div class="diff"><DiffView lines={fileLines(file)} split={false} /></div>
    {/each}
  </div>
{:else if item.kind === 'permission'}
  <div class="msg perm">
    <div class="perm-title">{t('composer.permission.title', { engine: 'Claude Code' })}</div>
    <div class="perm-desc">{item.input.description}</div>
    <div class="perm-label">{mobile.wholeCommand}</div>
    <pre class="perm-cmd">{item.input.command}</pre>
    {#if onAnswer}
      <div class="perm-actions">
        <button class="deny" onclick={() => onAnswer('deny')}>{t('composer.permission.deny')}</button>
        <button class="allow" onclick={() => onAnswer('allow')}>{t('composer.permission.allow')}</button>
      </div>
    {/if}
  </div>
{/if}

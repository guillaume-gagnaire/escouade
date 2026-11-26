<script lang="ts">
  // A skill or a plugin of an agent near on the « Workspace », above its card (spec 2026-11-25-vue-workspace-design.md,
  // « L'outillage », and the user's choice: above the card, not west of it). Squarer than an agent's card, a coloured
  // edge on top: « de la doc, pas un agent ». While one of its calls runs, a green ring and the word.
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import { toolName, type ToolCard } from '../../lib/workspace-layout';
  import Icon from '../Icon.svelte';

  let { tool }: { tool: ToolCard } = $props();

  const calls = $derived(t('workspace.tool.calls', { count: tool.calls }));
  const name = $derived(toolName(tool));
  const label = $derived(
    [t('workspace.tool.label', { kind: t(`workspace.tool.${tool.kind}`), name, calls }), tool.running ? t('workspace.tool.running') : '']
      .filter(Boolean)
      .join(', '),
  );
</script>

<!-- Reached by Tab, as every card of the canvas: a screen reader reads it. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<article class="tool {tool.kind}" class:running={tool.running} data-side="west" aria-label={label} tabindex="0">
  <header>
    <span class="ico"><Icon name={tool.kind === 'skill' ? 'scroll' : 'box'} size={14} /></span>
    <b class="name" use:tip={tool.name}>{name}</b>
  </header>
  <p>
    <span class="kind">{t(`workspace.tool.${tool.kind}`)}</span> · <span class="num">{calls}</span>
    {#if tool.running}<span class="st">{t('workspace.tool.running')}</span>{/if}
  </p>
</article>

<style>
  .tool {
    --edge: var(--skill);
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 6px 10px;
    border-top: 2px solid var(--edge);
    border-radius: 3px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12.5px;
  }
  .tool.plugin {
    --edge: var(--info);
  }
  .tool.running {
    box-shadow:
      inset 0 0 0 1px var(--run),
      0 0 0 3px color-mix(in oklch, var(--run) 18%, transparent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .ico {
    color: var(--edge);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    font-weight: 590;
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  p {
    display: flex;
    gap: 4px;
    margin: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .st {
    margin-left: auto;
    color: var(--run);
    font-weight: 550;
  }
</style>

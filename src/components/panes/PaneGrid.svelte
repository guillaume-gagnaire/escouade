<script lang="ts">
  // The split screen (ESC-392), drawn from its tree: a leaf is a pane, a split two parts and the handle between them.
  // The handle sets the split's ratio: dragged, with the arrows when it has the focus, back to half and half on a
  // double click.
  import { t } from '../../lib/i18n';
  import { isLeaf, itemKey, MIN_RATIO, type PaneNode } from '../../lib/panes';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Project } from '../../lib/types';
  import Pane from './Pane.svelte';
  import PaneGrid from './PaneGrid.svelte';

  let { node, path = '', project }: { node: PaneNode; path?: string; project: Project } = $props();

  let box = $state<HTMLDivElement>();
  let dragging = $state(false);

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  function move(e: PointerEvent) {
    if (!dragging || !box || isLeaf(node)) return;
    const r = box.getBoundingClientRect();
    app.setPaneRatio(path, node.dir === 'row' ? (e.clientX - r.left) / r.width : (e.clientY - r.top) / r.height);
  }

  function key(e: KeyboardEvent) {
    if (isLeaf(node)) return;
    const step = e.shiftKey ? 0.1 : 0.02;
    const back = node.dir === 'row' ? 'ArrowLeft' : 'ArrowUp';
    const forth = node.dir === 'row' ? 'ArrowRight' : 'ArrowDown';
    if (e.key === back) app.setPaneRatio(path, node.ratio - step);
    else if (e.key === forth) app.setPaneRatio(path, node.ratio + step);
    else if (e.key === 'Home') app.setPaneRatio(path, MIN_RATIO);
    else if (e.key === 'End') app.setPaneRatio(path, 1 - MIN_RATIO);
    else if (e.key === 'Enter') app.setPaneRatio(path, 0.5);
    else return;
    e.preventDefault();
  }
</script>

{#if isLeaf(node)}
  {#key itemKey(node.leaf)}
    <Pane item={node.leaf} {project} />
  {/key}
{:else}
  <div class="split {node.dir}" class:dragging bind:this={box} style:--r={node.ratio}>
    <div class="part a"><PaneGrid node={node.a} path="{path}a" {project} /></div>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="gap"
      role="separator"
      tabindex="0"
      aria-orientation={node.dir === 'row' ? 'vertical' : 'horizontal'}
      aria-valuenow={Math.round(node.ratio * 100)}
      aria-valuemin={Math.round(MIN_RATIO * 100)}
      aria-valuemax={Math.round((1 - MIN_RATIO) * 100)}
      aria-label={t('shell.panes.handle')}
      use:tip={{ text: t('shell.panes.handleTip'), aria: false }}
      data-testid="pane-handle"
      onpointerdown={down}
      onpointermove={move}
      onpointerup={() => (dragging = false)}
      onpointercancel={() => (dragging = false)}
      ondblclick={() => app.setPaneRatio(path, 0.5)}
      onkeydown={key}
    ></div>
    <div class="part b"><PaneGrid node={node.b} path="{path}b" {project} /></div>
  </div>
{/if}

<style>
  .split {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .split.col {
    flex-direction: column;
  }
  .part {
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .part.a {
    flex: var(--r) 1 0%;
  }
  .part.b {
    flex: calc(1 - var(--r)) 1 0%;
  }
  /* The frame shows between two panes (direction B): 8 px, the handle's grip in its middle. */
  .gap {
    position: relative;
    flex: 0 0 8px;
    outline: none;
  }
  .row > .gap {
    cursor: col-resize;
  }
  .col > .gap {
    cursor: row-resize;
  }
  .gap::after {
    content: '';
    position: absolute;
    border-radius: 2px;
    background: var(--line);
    transition: background 0.12s;
  }
  .row > .gap::after {
    left: 2.5px;
    top: 50%;
    width: 3px;
    height: 36px;
    margin-top: -18px;
  }
  .col > .gap::after {
    top: 2.5px;
    left: 50%;
    height: 3px;
    width: 36px;
    margin-left: -18px;
  }
  .gap:hover::after,
  .gap:focus-visible::after,
  .dragging > .gap::after {
    background: var(--p-ink);
  }
  .gap:focus-visible {
    box-shadow: inset 0 0 0 1px var(--ring);
    border-radius: 4px;
  }
  .dragging {
    user-select: none;
  }
</style>

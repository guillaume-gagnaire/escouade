<script lang="ts">
  // The plan of an agent near, north of its card (spec 2026-11-25-vue-workspace-design.md, « Au nord », and « Choix
  // validés ») : the conversation's own banner, as it is there, in a box of the canvas.
  import { observeHeight } from '../../lib/resize';
  import type { Agent } from '../../lib/types';
  import PlanBanner from '../PlanBanner.svelte';

  // `mini`: at mid-height, its head and its gauge alone; a click on it unfolds it (`onunfold`). `onsize`: its height, the
  // list as the reader sized it.
  let {
    agent,
    mini = false,
    onunfold,
    onsize = () => {},
  }: { agent: Agent; mini?: boolean; onunfold?: () => void; onsize?: (h: number) => void } = $props();
</script>

<section class="north" data-side="north" use:observeHeight={onsize}>
  <PlanBanner {agent} folded={mini} {onunfold} />
</section>

<style>
  /* A surface of the work area: the banner's own column, list and grip inside, its list scrolling past its height (232 px,
     or the reader's); its own height, measured. */
  .north {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    overflow: hidden;
    border-radius: var(--r);
    background: var(--panel);
    box-shadow:
      0 0 0 1px var(--line),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  /* The banner's rule under it is the conversation's, where messages follow it. */
  .north > :global(.plan) {
    border-bottom: none;
  }
</style>

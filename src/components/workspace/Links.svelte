<script lang="ts">
  // The links of the compasses, one `<svg>` under the cards (spec 2026-11-25-vue-workspace-design.md, « Construction »).
  // Only what runs right now moves: its dashes flow and a dot goes from the agent to it. Ended: grey; failed: red.
  import { t } from '../../lib/i18n';
  import type { Link } from '../../lib/workspace-layout';

  let { links, w, h }: { links: Link[]; w: number; h: number } = $props();

  // Unique in the document: a launched link names its path for its word.
  const uid = Math.random().toString(36).slice(2);
</script>

<svg class="links" width={w} height={h} overflow="visible" aria-hidden="true">
  {#each links as l, i (l.id)}
    <path id="link-{uid}-{i}" class="link {l.state}" d={l.d} />
    {#if l.state === 'run'}
      <circle class="flow" r="3.5"><animateMotion dur="1.6s" repeatCount="indefinite" path={l.d} /></circle>
    {:else if l.state === 'launched'}
      <text class="said" dy="-6"
        ><textPath href="#link-{uid}-{i}" startOffset="50%" text-anchor="middle">{t('workspace.launched')}</textPath></text
      >
    {/if}
  {/each}
</svg>

<style>
  .links {
    position: absolute;
    left: 0;
    top: 0;
    pointer-events: none;
  }
  .link {
    fill: none;
    stroke-width: 1.5;
  }
  .link.run {
    stroke: var(--run);
    stroke-dasharray: 6 5;
    animation: flow 0.8s linear infinite;
  }
  .link.done {
    stroke: var(--line-3);
  }
  .link.failed {
    stroke: var(--del);
  }
  .link.plan {
    stroke: var(--line-2);
  }
  .link.launched {
    stroke: var(--muted);
    stroke-dasharray: 2 5;
  }
  .flow {
    fill: var(--run);
  }
  .said {
    fill: var(--muted);
    font: 500 11px var(--font);
  }
  @keyframes flow {
    to {
      stroke-dashoffset: -11;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .link.run {
      animation: none;
    }
    .flow {
      display: none;
    }
  }
</style>

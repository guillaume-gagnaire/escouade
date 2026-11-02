<script lang="ts">
  import { ciPill, ciTitle, ciTone } from '../lib/branch-ci';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { BranchCi, CiOf } from '../lib/types';
  import Icon from './Icon.svelte';

  // The pill of a branch's CI (ESC-63): « main ✗ depuis 2 h » in a project's board's header, the same on the card of a
  // ticket that pushed its branch. A click opens its jobs (CiModal); a reading that failed is said in its tooltip, and the
  // pill keeps the last state it read.
  let { ci, of, projectId }: { ci: BranchCi; of: CiOf; projectId: string } = $props();

  const tone = $derived(ciTone(ci));
  const label = $derived(ciPill(ci, app.now));
  const title = $derived(ciTitle(ci, app.now));
  /** Red and green say it in their words (✗, ✓): a glyph only while it waits, or has no job. */
  const icon = $derived(tone === 'pending' ? 'clock' : tone === 'none' ? 'circle' : null);
</script>

<button
  class="ci-pill {tone}"
  class:stale={!!ci.error}
  data-ci-pill={of.kind}
  aria-label={title}
  use:tip={{ text: title, aria: false }}
  onclick={(e) => {
    e.stopPropagation();
    app.modal = { kind: 'ci', of, projectId };
  }}
>
  {#if icon}<Icon name={icon} size={12} />{/if}
  <span class="l">{label}</span>
  {#if ci.error}<Icon name="info" size={12} />{/if}
</button>

<style>
  .ci-pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    max-width: 100%;
    height: 22px;
    padding: 0 8px;
    border: none;
    border-radius: 11px;
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    font-size: 11.5px;
    font-weight: 550;
    cursor: pointer;
  }
  .ci-pill:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .l {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .failure {
    background: color-mix(in oklch, var(--del) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent);
    color: var(--del);
  }
  .success {
    color: var(--run);
  }
  .pending {
    color: var(--wait);
  }
  .stale {
    opacity: 0.75;
  }
</style>

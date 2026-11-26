<script lang="ts">
  // A side of a compass folded at mid-height (spec 2026-11-25-vue-workspace-design.md, « À mi-hauteur : des piles ») :
  // a button that says its side and its count, and unfolds it (its name says so: it is gone once unfolded); ringed while one of its items runs.
  import { t } from '../../lib/i18n';
  import type { Pile, PileSide } from '../../lib/workspace-layout';

  let { pile, onunfold }: { pile: Pile; onunfold: (side: PileSide) => void } = $props();
</script>

<button
  type="button"
  class="pile"
  class:running={pile.running}
  data-side={pile.side}
  aria-label={t(`workspace.pile.unfold.${pile.side}`, { count: pile.count })}
  onclick={() => onunfold(pile.side)}
>
  {t(`workspace.pile.${pile.side}`, { count: pile.count })}
</button>

<style>
  /* Squared as the tool cards, stacked as a pile: a second edge drawn under it. */
  .pile {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 3px;
    background: var(--ws-card);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      4px 4px 0 -1px var(--panel),
      4px 4px 0 0 var(--line-2);
    color: var(--text);
    font: inherit;
    font-size: 13px;
    font-weight: 590;
    cursor: pointer;
  }
  .pile:hover {
    background: var(--hover);
  }
  .pile.running {
    box-shadow:
      inset 0 0 0 1px var(--run),
      0 0 0 3px color-mix(in oklch, var(--run) 18%, transparent);
  }
</style>

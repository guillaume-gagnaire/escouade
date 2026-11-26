<script lang="ts">
  // The landing zone, south of a compass (spec 2026-11-25-vue-workspace-design.md, « Au sud : la zone d'atterrissage »):
  // always drawn, a dotted edge, hatched, corner marks. What is not finished hovers above its slot, dotted; finished, it
  // lands. The pages published, the remote CI, the deployments; never the local tests (under their card).
  import { t } from '../../lib/i18n';
  import { observeHeight } from '../../lib/resize';
  import { landed, type Landing } from '../../lib/workspace-landing';
  import ArtifactCard from './ArtifactCard.svelte';
  import RibbonCard from './RibbonCard.svelte';

  let { agentId, items, onsize }: { agentId: string; items: Landing[]; onsize: (h: number) => void } = $props();

  const down = $derived(items.filter(landed));
  const coming = $derived(items.filter((l) => !landed(l)));
  const title = $derived(
    t('workspace.land.title', {
      landed: t('workspace.land.landed', { count: down.length }),
      coming: t('workspace.land.coming', { count: coming.length }),
    }),
  );
</script>

{#snippet card(l: Landing)}
  <div class="slot" class:artifact={l.kind === 'artifact'}>
    {#if l.kind === 'artifact'}<ArtifactCard item={l} />{:else}<RibbonCard {agentId} item={l} side="south" />{/if}
  </div>
{/snippet}

<section class="zone" data-side="south" aria-label={title} use:observeHeight={onsize}>
  <i class="corner tl"></i><i class="corner tr"></i><i class="corner bl"></i><i class="corner br"></i>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <h3 class:above={coming.length > 0} tabindex="0">{title}</h3>
  {#if !items.length}
    <p class="empty">{t('workspace.land.empty')}</p>
  {/if}
  {#if coming.length}
    <div class="row coming">
      {#each coming as l (l.id)}{@render card(l)}{/each}
    </div>
  {/if}
  {#if down.length}
    <div class="row landed">
      {#each down as l (l.id)}{@render card(l)}{/each}
    </div>
  {/if}
</section>

<style>
  .zone {
    position: relative;
    box-sizing: border-box;
    min-height: 132px;
    padding: 10px 14px 14px;
    border: 1px dashed var(--line-2);
    border-radius: 9px;
    background: repeating-linear-gradient(-45deg, var(--line-2) 0 1px, transparent 1px 9px);
  }
  .corner {
    position: absolute;
    width: 10px;
    height: 10px;
    border: 0 solid var(--line-3);
  }
  .tl {
    top: -1px;
    left: -1px;
    border-width: 2px 0 0 2px;
    border-top-left-radius: 9px;
  }
  .tr {
    top: -1px;
    right: -1px;
    border-width: 2px 2px 0 0;
    border-top-right-radius: 9px;
  }
  .bl {
    bottom: -1px;
    left: -1px;
    border-width: 0 0 2px 2px;
    border-bottom-left-radius: 9px;
  }
  .br {
    right: -1px;
    bottom: -1px;
    border-width: 0 2px 2px 0;
    border-bottom-right-radius: 9px;
  }
  h3 {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
  }
  /* Room for the row on approach: it hovers 14 px above its slot, its outline 3 px out. */
  h3.above {
    margin-bottom: 24px;
  }
  .empty {
    margin: 22px 0 0;
    color: var(--soft);
    font-size: 12.5px;
    text-align: center;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .row + .row {
    margin-top: 12px;
  }
  .slot {
    width: 240px;
    height: 104px;
  }
  .slot.artifact {
    height: 92px;
  }
  /* Not finished: hovers above its slot, dotted. */
  .coming .slot {
    transform: translateY(-14px);
    margin-bottom: -14px;
    border-radius: 9px;
    outline: 1px dashed var(--line-3);
    outline-offset: 2px;
  }
</style>

<script lang="ts">
  // « Nouveau dans la 2.3 » (ESC-506, docs/ONBOARDING.md § 5.5, choice O9): a device set up before the release sees it
  // once, never the first launch. « Voir » opens the map on what is new; × closes it for this release. A minor version
  // shows nothing.
  import { act, discover, openMap } from '../../lib/discover.svelte';
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  const release = $derived(discover.view?.band ?? null);

  function see() {
    if (!release) return;
    openMap(true, release);
    void act({ kind: 'bandSeen' });
  }
</script>

{#if release}
  <div class="new-band" role="status" data-testid="discover-band">
    <Icon name="gift" size={14} />
    <span class="msg">{t('discover.band.text', { release })}</span>
    <button class="btn small" onclick={see}>{t('discover.band.see')}</button>
    <button
      class="icon-btn"
      aria-label={t('discover.band.close')}
      use:tip={t('discover.band.close')}
      onclick={() => void act({ kind: 'bandSeen' })}><Icon name="close" size={13} /></button
    >
  </div>
{/if}

<style>
  .new-band {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
    padding: 6px 8px 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--p-ink);
    background: var(--p-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p-ink) 25%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
</style>

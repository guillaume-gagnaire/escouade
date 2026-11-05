<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { SeedOffer } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import PluginTile from './PluginTile.svelte';

  // What the app's seed holds that is not installed (ESC-338): a plugin the user uninstalled, which the seed does not put
  // back, and one it refused, its reason said. Installed on a click only, in the usual window.
  const why = (o: SeedOffer) =>
    o.refused
      ? t('plugins.installed.seedRefused', { reason: o.refused })
      : o.removed !== null
        ? t('plugins.installed.seedRemoved', { version: o.removed })
        : t('plugins.installed.seedMissing');
</script>

{#if plugins.offers.length}
  <section class="seed" aria-label={t('plugins.installed.seedTitle')}>
    <div class="group">
      {t('plugins.installed.seedTitle')} · {plugins.offers.length}<span>— {t('plugins.installed.seedNote')}</span>
    </div>
    {#each plugins.offers as o (o.id)}
      <div class="line">
        <PluginTile id={o.id} name={o.name} />
        <div class="txt">
          <b>{o.name}</b>
          <span class:bad={o.refused}>{why(o)}</span>
        </div>
        <span class="tag mono">{t('plugins.version', { version: o.version })}</span>
        <button class="btn" onclick={() => plugins.installOffer(o)}>{t('plugins.installed.seedInstall')}</button>
      </div>
    {/each}
  </section>
{/if}

<style>
  .seed {
    display: grid;
    margin-top: 14px;
  }
  .group {
    color: var(--muted);
    font-size: 11.5px;
    padding: 12px 0 6px;
    border-bottom: 1px solid var(--line);
  }
  .group span {
    margin-left: 6px;
  }
  .line {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .txt b {
    display: block;
    font-weight: 600;
  }
  .txt span {
    color: var(--muted);
    font-size: 12px;
  }
  .txt span.bad {
    color: var(--danger, var(--muted));
  }
  .tag {
    color: var(--muted);
    font-size: 11.5px;
  }
</style>

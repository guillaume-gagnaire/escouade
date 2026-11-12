<script lang="ts">
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import Icon from './Icon.svelte';

  // SEC-07: while « Ignorer la vérification des certificats TLS » is on, a band across the window says so, always. It
  // holds through the proxy alone (`src-tauri/src/tls.rs`): with none, the band says the setting does nothing. Its
  // button opens Réglages › Avancé › Réseau, where the file of authorities is chosen in its place.
  const proxy = $derived(app.settings.proxyUrl.trim());
  const shown = $derived(app.settings.insecureTls);
  /** The proxy as the band names it: its host and port, never its user nor its password. */
  const where = $derived.by(() => {
    try {
      return new URL(proxy.includes('://') ? proxy : `http://${proxy}`).host;
    } catch {
      return '';
    }
  });
</script>

{#if shown}
  <div class="tls-band" role="status" data-testid="tls-band">
    <Icon name="shield" size={14} />
    <span class="msg">{where ? t('settings.network.insecureBanner', { proxy: where }) : t('settings.network.insecureBannerNoProxy')}</span>
    <button class="small" onclick={() => (app.modal = { kind: 'settings', page: 'advanced', at: 'network' })}
      >{t('settings.network.insecureBannerAction')}</button
    >
  </div>
{/if}

<style>
  .tls-band {
    grid-column: 2;
    grid-row: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 8px 6px 0;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--warn-ink);
    background: color-mix(in oklch, var(--warn-ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--warn-ink) 35%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
  }
</style>

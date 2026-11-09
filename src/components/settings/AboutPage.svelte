<script lang="ts">
  import { CREDITS_URL } from '../../lib/conversation';
  import { found as foundEngines } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { DISCORD_URL, siteUrl, supportUrl } from '../../lib/links';
  import { openUrl } from '../../lib/opener';
  import { app } from '../../lib/state.svelte';
  import { shownVersion } from '../../lib/updater';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // The page « À propos », at the foot of the bar (PROPOSITION.md §2): who it is (the logo, the version, the harnesses
  // found), the credits, the site, Discord, the support. Its updates and its statistics went to « Général », its local
  // data to « Avancé ».
  const found = $derived(foundEngines());
  const open = (url: string) => openUrl(url).catch((e) => app.toast(String(e), 'error'));
</script>

<!-- Who it is (`.about-id` of the 2.0 design): the logo, the version, the harnesses found. -->
<div class="about-id">
  <img src="/logo.svg" alt="" draggable="false" />
  <div class="who">
    <b data-testid="app-version">Escouade {shownVersion(app.version)}</b>
    <span
      >{found.length ? t('settings.about.enginesFound', { list: found.map((e) => e.name).join(', ') }) : t('settings.about.noEngine')}</span
    >
  </div>
</div>
<Group aria-label="Escouade">
  <Row label={t('settings.about.credits')} desc={t('settings.about.creditsDesc')}>
    <button class="btn" onclick={() => open(CREDITS_URL)}>{t('settings.about.creditsOpen')}</button>
  </Row>
  <Row label={t('settings.about.site')} desc={t('settings.about.siteDesc')}>
    <button class="btn" aria-label={t('settings.about.siteOpenLabel')} onclick={() => open(siteUrl())}
      ><Icon name="ext" size={14} />{t('settings.about.siteOpen')}</button
    >
  </Row>
  <Row label={t('settings.about.discord')} desc={t('settings.about.discordDesc')}>
    <button class="btn" aria-label={t('settings.about.discordJoinLabel')} onclick={() => open(DISCORD_URL)}
      ><Icon name="ext" size={14} />{t('settings.about.discordJoin')}</button
    >
  </Row>
  <!-- Support, said once here and never pushed: no window, no reminder (ESC-185). -->
  <Row label={t('settings.about.support')} desc={t('settings.about.supportDesc')}>
    <button class="btn" onclick={() => open(supportUrl())}><Icon name="ext" size={14} />{t('settings.about.supportOpen')}</button>
  </Row>
</Group>

<style>
  .about-id {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 22px 0 18px;
  }
  .about-id img {
    width: 44px;
    height: 44px;
  }
  .who {
    display: grid;
    min-width: 0;
  }
  .who b {
    font-size: 16px;
    font-weight: 640;
  }
  .who span {
    color: var(--muted);
    font-size: 12.5px;
  }
</style>

<script lang="ts">
  // A proposal of a feature switched off, at the right moment (F-3, ESC-558, docs/FEATURES.md § 2): a small window at
  // the bottom right, in the language of the bubbles of the discovery (ESC-506) and of the box « À toi ». It says what
  // Escouade noticed and what the feature would do, with « Activer », « Pas maintenant » and « Ne plus proposer » (given
  // back in Réglages › Fonctionnalités). A dialog that is not modal: it takes no focus, is told politely to a screen
  // reader, and Échap in it is « Pas maintenant ». It never comes while the user types (`offers.svelte.ts`), and stays
  // in its corner, out of the way, until it is answered.
  import { FEATURE_ICONS } from '../lib/features';
  import { t } from '../lib/i18n';
  import { answer, offers } from '../lib/offers.svelte';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  const o = $derived(offers.shown);
  const name = $derived(o ? t(`features.${o.feature}.name`) : '');

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    void answer('later');
  }
</script>

{#if o}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions: Échap in it is « Pas maintenant » -->
  <div
    class="offer"
    role="dialog"
    aria-modal="false"
    tabindex="-1"
    aria-labelledby="feature-offer-title"
    aria-describedby="feature-offer-noticed feature-offer-does"
    aria-live="polite"
    data-testid="feature-offer"
    data-feature={o.feature}
    data-signal={o.signal}
    {onkeydown}
  >
    <header>
      <span class="ic" aria-hidden="true"><Icon name={FEATURE_ICONS[o.feature]} size={16} /></span>
      <div class="hd">
        <span class="kicker">{t('features.offer.kicker')}</span>
        <h2 id="feature-offer-title">{name}</h2>
      </div>
    </header>
    <div class="part">
      <span class="lab">{t('features.offer.noticed')}</span>
      <p id="feature-offer-noticed">
        {t(`features.offer.signals.${o.signal}`, { count: o.count, project: o.project ?? '' })}
      </p>
    </div>
    <div class="part">
      <span class="lab">{t('features.offer.would')}</span>
      <p id="feature-offer-does">{t(`features.offer.does.${o.feature}`)}</p>
    </div>
    <p class="private"><Icon name="lock" size={12} />{t('features.offer.private')}</p>
    <div class="acts">
      <button class="btn small primary" onclick={() => void answer('enable')}>{t('features.offer.enable')}</button>
      <button class="btn small ghost" onclick={() => void answer('later')}>{t('features.offer.later')}</button>
      <button class="btn small ghost" use:tip={t('features.offer.neverHint')} onclick={() => void answer('never')}
        >{t('features.offer.never')}</button
      >
    </div>
  </div>
{/if}

<style>
  /* The bubble of the discovery's card (DiscoverTip), set in the corner above the status bar. */
  .offer {
    position: fixed;
    right: 16px;
    bottom: 40px;
    z-index: 95;
    width: 368px;
    max-width: calc(100vw - 32px);
    display: grid;
    gap: 10px;
    padding: 14px 16px 14px;
    border-radius: 12px;
    background: var(--panel-2, var(--panel));
    color: var(--text);
    box-shadow:
      0 0 0 1px var(--line-2),
      0 16px 36px rgb(0 0 0 / 0.3);
    font-size: 12.5px;
    line-height: 1.5;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .ic {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 1px solid color-mix(in oklch, var(--teal) 45%, var(--line));
    border-radius: 8px;
    background: color-mix(in oklch, var(--teal) 12%, transparent);
    color: var(--teal);
  }
  .hd {
    display: grid;
    min-width: 0;
  }
  .kicker {
    color: var(--muted);
    font-size: 11px;
    letter-spacing: 0.02em;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 640;
  }
  .part {
    display: grid;
    gap: 2px;
  }
  .lab {
    color: var(--muted);
    font-size: 11px;
    font-weight: 560;
  }
  p {
    margin: 0;
    color: var(--soft);
  }
  .private {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding-top: 2px;
  }
  @media (prefers-reduced-motion: no-preference) {
    .offer {
      animation: rise 240ms var(--ease);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .offer {
      animation: fade 120ms linear;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
</style>

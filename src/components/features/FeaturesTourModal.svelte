<script lang="ts">
  import { FEATURES } from '../../lib/features';
  import { endTour, tour } from '../../lib/features-tour.svelte';
  import { t } from '../../lib/i18n';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';
  import FeatureTour from './FeatureTour.svelte';

  // The assistant of the features on its own (F-2, ESC-559): opened by itself at the first window of the version that
  // brings the choice, on an installation updated to it (« Passer : garder l'essentiel » keeps what it uses: the update
  // switched off the rest), or by « Revoir la présentation » (Réglages › Fonctionnalités: « Fermer »). The same screens as
  // the station of the first launch; Échap passes, as « Passer » does.
  const review = $derived(tour.mode === 'review');
  let at = $state(0);
  const last = $derived(at >= FEATURES.length - 1);
</script>

<Modal title={review ? t('features.tour.reviewTitle') : t('features.tour.updateTitle')} width={1120} onclose={endTour}>
  {#snippet lead()}<span class="lead"><Icon name="sparkle" /></span>{/snippet}
  <div class="body" data-testid="features-tour-window">
    {#if !review}<p class="intro">{t('features.tour.updateLead')}</p>{/if}
    <FeatureTour list={FEATURES} bind:at mode={tour.mode} used={tour.view?.used} />
  </div>
  {#snippet footer()}
    <span class="foot">{t('features.tour.foot')}</span>
    <span class="sp"></span>
    <button class="btn ghost" onclick={endTour}>{review ? t('features.tour.close') : t('features.tour.skip')}</button>
    <button class="btn" disabled={at === 0} onclick={() => (at -= 1)}>{t('features.tour.previous')}</button>
    {#if last}
      <button class="btn primary" onclick={endTour}>{t('features.tour.finish')}</button>
    {:else}
      <button class="btn primary" onclick={() => (at += 1)}>{t('features.tour.next')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .body {
    display: grid;
    gap: 16px;
  }
  .intro {
    margin: 0;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.55;
  }
  .foot {
    color: var(--dim);
    font-size: 12.5px;
  }
  .sp {
    flex: 1;
  }
  .lead {
    display: grid;
    place-items: center;
    color: var(--brand);
  }
</style>

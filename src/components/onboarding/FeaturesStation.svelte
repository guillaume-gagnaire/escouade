<script lang="ts">
  import { FEATURES } from '../../lib/features';
  import { endTour, loadTour, tour } from '../../lib/features-tour.svelte';
  import { t } from '../../lib/i18n';
  import { onboarding, statusOf, type Primary } from '../../lib/onboarding.svelte';
  import Icon from '../Icon.svelte';
  import FeatureTour from '../features/FeatureTour.svelte';
  import StationHead from './StationHead.svelte';

  // The station of the features (F-2, ESC-559), after the first agent and before the statistics: the user has seen the
  // app at work, and chooses what it shows around the agents. The assistant, one feature per screen: « Suivant » goes from
  // one to the next, « Continuer » on the last ends the station; « Passer : garder l'essentiel » (its « Plus tard ») passes
  // it with no switch changed, so only the essential and what is used stay on (a new installation has the rest off,
  // `NEW_INSTALL`). « Retour » goes back a screen before it goes back a station. Imposed whole by a policy: shown, crossed
  // in one « Continuer ».
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  let at = $state(0);
  const last = $derived(at >= FEATURES.length - 1);
  const imposed = $derived(statusOf('features') === 'policy');
  const redoing = $derived(onboarding.view?.trigger === 'redo');

  // What is used now (a source of tickets, the voice plugged in at « Ce que tu veux brancher »): said on its screen.
  $effect(() => {
    void loadTour(true);
  });

  $effect(() => {
    primary = {
      label: last || imposed ? t('onboarding.continue') : t('features.tour.next'),
      later: t('features.tour.skip'),
      back: () => {
        if (at === 0 || imposed) return false;
        at -= 1;
        return true;
      },
      run: async () => {
        if (!last && !imposed) {
          at += 1;
          return false;
        }
        await endTour();
        return true;
      },
    };
  });
</script>

<div class="q" data-testid="onboarding-features">
  <StationHead step="features" question={t('features.tour.question')}>
    {#snippet why()}{t('features.tour.why')}{/snippet}
  </StationHead>
  {#if imposed}<p class="lock"><Icon name="lock" size={13} />{t('features.tour.imposed')}</p>{/if}
  <FeatureTour list={FEATURES} bind:at mode={redoing ? 'review' : 'first'} used={tour.view?.used} />
  <p class="foot">{t('features.tour.foot')}</p>
</div>

<style>
  .q {
    display: grid;
    gap: 18px;
    width: min(1040px, 100%);
  }
  .lock,
  .foot {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .foot {
    color: var(--dim);
  }
</style>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { stations } from '../../lib/onboarding.svelte';
  import type { OnboardingStep } from '../../lib/types';

  // The head of a station (docs/ONBOARDING.md § 4 and § 8): its number on the line, its question (the station's `h1`,
  // focused as it arrives) and why it is asked, the question's description.
  let { step, question, why }: { step: OnboardingStep; question: string; why?: Snippet } = $props();
  const n = $derived(stations().indexOf(step) + 1);
  let h1 = $state<HTMLElement>();
  $effect(() => {
    void step;
    h1?.focus({ preventScroll: true });
  });
</script>

<div class="ord">
  <span class="ln" aria-hidden="true"></span>{t('onboarding.order', { n, name: t(`onboarding.stations.${step}.inline`) })}
</div>
<h1 id="onb-q" tabindex="-1" aria-describedby={why ? 'onb-why' : undefined} bind:this={h1}>{question}</h1>
{#if why}<p class="why" id="onb-why">{@render why()}</p>{/if}

<style>
  .ord {
    display: flex;
    gap: 10px;
    align-items: center;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
  }
  .ln {
    width: 22px;
    height: 3px;
    border-radius: 2px;
    background: var(--brand);
  }
  h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.2;
  }
  /* The question takes the focus at each station to be read first: it is no control, it shows no ring. */
  h1:focus,
  h1:focus-visible {
    outline: none;
    box-shadow: none;
  }
  .why {
    margin: 0;
    max-width: 60ch;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.6;
  }
  @media (max-width: 1100px) {
    h1 {
      font-size: 22px;
    }
  }
</style>

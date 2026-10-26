<script lang="ts">
  import {
    fProjection,
    meterLabel,
    meterName,
    projectedPct,
    projectionInBar,
    projectionLevel,
    projectionMode,
    projectionTip,
    quotaTip,
  } from '../lib/accounts';
  import { fCountdown, fPct } from '../lib/format';
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { RateWindow } from '../lib/types';

  // One meter of an account's quota: a window labelled by its length (« 5 h », « 7 j », « 30 j »), or the month's credits.
  // In the status bar: the label, a thin bar, the percentage, and the projection at the reset only past 80 % (choice 21
  // of the 2.0 design); the reset in its tooltip. In the panel of the accounts (`detail`): the label and the percentage
  // over a long bar, the reset under it, and the projection always. The bar is a meter that says its value to the
  // keyboard and to screen readers.
  let {
    minutes,
    usage,
    now,
    detail = false,
    focusable = true,
  }: {
    /** The window's length (300: 5 hours); null for the month's credits. */
    minutes: number | null;
    /** The meter as last read; null while it is not known. */
    usage: RateWindow | null;
    /** The clock (ms), to count down to the reset. */
    now: number;
    /** The panel's form: the reset written under the bar, every projection, no tooltip. */
    detail?: boolean;
    /** The bar takes the focus. Off when a button around it does (the keyboard then opens the panel, which writes the values). */
    focusable?: boolean;
  } = $props();

  const label = $derived(meterLabel(minutes));
  const name = $derived(meterName(minutes));
  const pct = $derived(usage ? Math.min(100, Math.max(0, usage.pct)) : 0);
  /** Where the window would end at the pace used so far (`projectedPct`); null when too early, or « Projection des quotas » on « Jamais ». */
  const projected = $derived(projectionMode() === 'off' ? null : projectedPct(minutes, usage, now));
  const level = $derived(projected === null ? null : projectionLevel(projected));
  /** In the bar, past 80 % or always, as « Projection des quotas » says (`projectionInBar`); in the panel, every one. */
  const shownProjection = $derived(level !== null && (detail || projectionInBar(level)));
  /**
   * « 42 % · remise à zéro le 10/10 à 18:00 · projection à la remise à zéro : 64 % »: the tooltip and the bar's value as
   * the screen reader reads it, the projection in it whatever the bar shows (under 1240 pixels, it shows none).
   */
  const text = $derived(quotaTip(usage) + projectionTip(minutes, usage, now));
</script>

{#snippet bar()}
  <!-- The bar is a meter, and reached by Tab: its value is the only way to read it without a pointer. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <span
    class="bar gauge"
    class:hot={pct > 80}
    role="meter"
    tabindex={focusable && !detail ? 0 : undefined}
    aria-label={name}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(pct)}
    aria-valuetext={text}
  >
    <span class="fill" style:width="{pct}%"></span>
  </span>
{/snippet}

{#snippet projection()}
  {#if shownProjection && projected !== null}
    <span class="p {level}"
      ><span class="sr">{t('accounts.quota.projectedSr')}</span>{t('accounts.quota.projected', { pct: fProjection(projected) })}</span
    >
  {/if}
{/snippet}

{#if detail}
  <span class="q detail">
    <span class="top">
      <span class="lbl" aria-hidden="true">{label}</span>
      <span><span class="v">{usage ? fPct(usage.pct) : '—'}</span>{@render projection()}</span>
    </span>
    {@render bar()}
    {#if usage?.resetsAt}<span class="d">{t('accounts.quota.reset', { countdown: fCountdown(usage.resetsAt, now) })}</span>{/if}
  </span>
{:else}
  <!-- The tooltip over the label, the bar and the percentage, a bigger target than the bar; the bar says it to a screen reader. -->
  <span class="q" use:tip={{ text, aria: false }}>
    <span class="lbl" aria-hidden="true">{label}</span>
    {@render bar()}
    <span class="v pct" aria-hidden="true">{usage ? fPct(usage.pct) : '—'}</span>
    {@render projection()}
  </span>
{/if}

<style>
  .q {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .bar {
    width: 30px;
  }
  .v {
    color: var(--soft);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  /* In the panel: the label and the values over a long bar, the reset under it. */
  .detail {
    display: grid;
    gap: 4px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .top {
    display: flex;
    justify-content: space-between;
    gap: 8px;
  }
  .detail .bar {
    width: 100%;
  }
  .d {
    color: var(--muted);
  }
  /* The projection at the reset: amber past 80 %, red past 100 % (the window runs out before its end), green below. */
  .p {
    margin-left: 4px;
    font-weight: 550;
  }
  .p.ok {
    color: var(--run);
  }
  .p.warn {
    color: var(--wait);
  }
  .p.over {
    color: var(--del);
  }
  /* Read by screen readers only. */
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>

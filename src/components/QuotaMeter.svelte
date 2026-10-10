<script lang="ts">
  import { fCountdown, fDayMonth, fPct, fTime } from '../lib/format';
  import { t } from '../lib/i18n';
  import type { RateWindow } from '../lib/types';

  // One quota window of an account: « 5h <bar> reset 3h01 ». The bar is a meter that says its value to the keyboard
  // and to screen readers; the percentage is not written (a sighted pointer finds it in a tooltip over the bar, a
  // keyboard on the bar's focus) unless `detail`: the panel of the accounts writes it next to the bar.
  let {
    kind,
    usage,
    now,
    detail = false,
    focusable = true,
  }: {
    kind: 'fiveHour' | 'sevenDay';
    /** The window as last read; null while it is not known. */
    usage: RateWindow | null;
    /** The clock (ms), to count down to the reset. */
    now: number;
    /** The panel's form: the percentage written, no tooltip. */
    detail?: boolean;
    /** The bar takes the focus. Off when a button around it does (the keyboard then opens the panel, which writes the values). */
    focusable?: boolean;
  } = $props();

  const label = $derived(kind === 'fiveHour' ? t('accounts.quota.fiveHour.label') : t('accounts.quota.sevenDay.label'));
  const name = $derived(kind === 'fiveHour' ? t('accounts.quota.fiveHour.name') : t('accounts.quota.sevenDay.name'));
  const pct = $derived(usage ? Math.min(100, Math.max(0, usage.pct)) : 0);
  /** « 42 % · remise à zéro le 10/10 à 18:00 »: the tooltip and the bar's value as the screen reader reads it. */
  const text = $derived.by(() => {
    if (!usage) return t('accounts.quota.unavailable');
    if (!usage.resetsAt) return fPct(usage.pct);
    return t('accounts.quota.tip', { pct: fPct(usage.pct), date: fDayMonth(usage.resetsAt), time: fTime(usage.resetsAt) });
  });

  let hover = $state(false);
  let focused = $state(false);
  /** Escape put the tooltip away; the next hover or focus brings it back. */
  let dismissed = $state(false);
  let bar = $state<HTMLElement>();
  /** Where the tooltip stands: above the bar, in the window's coordinates (the status bar clips what overflows it). */
  let spot = $state({ left: 8, bottom: 38 });
  const shown = $derived(!detail && (hover || focused) && !dismissed);

  function reveal() {
    dismissed = false;
    if (!bar) return;
    const r = bar.getBoundingClientRect();
    spot = { left: Math.max(8, r.left - 6), bottom: Math.max(8, window.innerHeight - r.top + 8) };
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !shown) return;
    e.stopPropagation();
    dismissed = true;
  }
</script>

<span
  class="q"
  class:detail
  onmouseenter={() => {
    hover = true;
    reveal();
  }}
  onmouseleave={() => (hover = false)}
  role="presentation"
>
  <span class="lbl" aria-hidden="true">{label}</span>
  <!-- The bar is a meter, and reached by Tab: its value is the only way to read it without a pointer. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <span
    class="bar"
    class:warn={pct > 80}
    bind:this={bar}
    role="meter"
    tabindex={focusable && !detail ? 0 : undefined}
    aria-label={name}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(pct)}
    aria-valuetext={text}
    onfocus={() => {
      focused = true;
      reveal();
    }}
    onblur={() => (focused = false)}
    onkeydown={onKeydown}
  >
    <span class="fill" style:width="{pct}%"></span>
  </span>
  {#if detail}<span class="v">{usage ? fPct(usage.pct) : '—'}</span>{/if}
  {#if usage?.resetsAt}
    <span class="d">{t('accounts.quota.reset', { countdown: fCountdown(usage.resetsAt, now) })}</span>
  {:else if !usage && !detail}
    <span class="d">—</span>
  {/if}
  {#if shown}
    <span class="tip" role="tooltip" style:left="{spot.left}px" style:bottom="{spot.bottom}px">{text}</span>
  {/if}
</span>

<style>
  .q {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .bar {
    width: 48px;
    height: 5px;
    border-radius: 3px;
    background: var(--elev2);
    overflow: hidden;
    margin-left: 2px;
    flex: none;
  }
  .bar:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--accent);
    transition: width 0.4s;
  }
  .warn .fill {
    background: var(--wait);
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
  .v {
    color: var(--text);
  }
  /* The rows of the panel line up: the same width for the label, the bar and the percentage. */
  .detail .lbl {
    width: 22px;
    color: var(--muted);
  }
  .detail .bar {
    width: 120px;
    margin-left: 0;
  }
  .detail .v {
    width: 44px;
    text-align: right;
  }
  .d {
    color: var(--dim);
  }
  .tip {
    position: fixed;
    z-index: 60;
    padding: 4px 8px;
    background: var(--elev);
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.4);
    color: var(--text);
    font-size: 11.5px;
    line-height: 1.3;
    white-space: nowrap;
    pointer-events: none;
  }
</style>

<script lang="ts" module>
  export interface DropdownOption {
    value: string;
    label: string;
    detail?: string;
    title?: string;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  import { t } from '../lib/i18n';

  // A compact picker whose menu opens upwards (the composer sits at the bottom of the window).
  let {
    caption,
    value,
    options,
    open,
    onToggle,
    onPick,
    showCaption = true,
    disabled = false,
    danger = false,
    title,
  }: {
    caption: string;
    value: string;
    options: DropdownOption[];
    open: boolean;
    onToggle: () => void;
    onPick: (value: string) => void;
    showCaption?: boolean;
    disabled?: boolean;
    danger?: boolean;
    title?: string;
  } = $props();

  const shown = $derived(options.find((o) => o.value === value)?.label ?? (value || '—'));
  const label = $derived(t('nav.dropdown.label', { caption, shown }));

  // Escape closes the menu first, before it can reach the composer (where it interrupts Claude).
  function onKeydownCapture(e: KeyboardEvent) {
    if (!open || e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    onToggle();
  }
</script>

<svelte:window onkeydowncapture={onKeydownCapture} />

<div class="dd">
  <button
    class="trigger"
    class:danger
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    title={title ?? label}
    {disabled}
    onclick={onToggle}
  >
    {#if showCaption}<span class="cap">{caption}</span>{/if}<span class="val">{shown}</span><span class="chev">▾</span>
  </button>
  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="backdrop" onclick={onToggle}></div>
    <div class="menu" role="menu" aria-label={caption}>
      {#each options as o (o.value)}
        <button role="menuitemradio" aria-checked={o.value === value} disabled={o.disabled} title={o.title} onclick={() => onPick(o.value)}>
          <span class="check" aria-hidden="true">{o.value === value ? '✓' : ''}</span>
          <span class="ml">{o.label}</span>
          {#if o.detail}<span class="md">{o.detail}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dd {
    position: relative;
    min-width: 0;
    flex: 0 1 auto;
  }
  .trigger {
    height: 30px;
    max-width: 100%;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--panel);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  .trigger:hover:not(:disabled) {
    border-color: var(--line2);
  }
  .trigger:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .trigger.danger {
    border-color: color-mix(in oklch, var(--del) 55%, transparent);
    color: var(--del);
  }
  .cap {
    font-family: var(--ui);
    color: var(--dim);
  }
  .val {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chev {
    flex: none;
    color: var(--dim);
    font-size: 9px;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 29;
  }
  .menu {
    position: absolute;
    left: 0;
    bottom: calc(100% + 6px);
    z-index: 30;
    width: max-content;
    min-width: 170px;
    max-width: 330px;
    display: flex;
    flex-direction: column;
    padding: 5px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
  }
  .menu button {
    display: grid;
    grid-template-columns: 16px auto;
    column-gap: 6px;
    padding: 7px 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .menu button:hover:not(:disabled) {
    background: var(--elev2);
  }
  .menu button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .check {
    grid-row: span 2;
    color: var(--accent);
    font-size: 12px;
  }
  .ml {
    font-size: 13px;
    font-weight: 600;
  }
  .md {
    font-size: 11.5px;
    color: var(--dim);
  }
</style>

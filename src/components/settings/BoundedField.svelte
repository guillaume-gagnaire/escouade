<script lang="ts">
  import { boundedInput } from '../../lib/fetch';
  import { t } from '../../lib/i18n';

  // A whole number between two bounds, typed freely or taken from the values offered (ESC-561): out of its bounds, it
  // is brought back to the nearest and the field says so under it. `empty`: the value an emptied field is saved as (a
  // project's « the general one », shown as its placeholder); without it, an emptied field keeps the value it had.
  let {
    value = $bindable(),
    min,
    max,
    presets,
    format,
    unit,
    label,
    id,
    empty = null,
    placeholder = '',
  }: {
    value: number;
    min: number;
    max: number;
    presets: number[];
    format: (n: number) => string;
    unit: string;
    label: string;
    id: string;
    empty?: number | null;
    placeholder?: string;
  } = $props();

  /** What the field shows: the value, or nothing for the empty one. */
  const shown = (v: number) => (empty !== null && v === empty ? '' : String(v));
  let text = $state('');
  let note = $state('');
  // Follows the value from outside (a preset, the draft read again), never a text being typed.
  let editing = false;
  $effect(() => {
    const v = value;
    if (!editing) text = shown(v);
  });

  function commit() {
    editing = false;
    const typed = text.trim();
    // Nothing typed since (the change after Enter, the blur after it): what it said stays.
    if (typed === shown(value)) return;
    const got = boundedInput(text, min, max, empty);
    if (!got) {
      text = shown(value);
      return;
    }
    note = got.clamped ? t('fetch.settings.clamped', { typed, min: format(min), max: format(max), value: format(got.value) }) : '';
    value = got.value;
    text = shown(got.value);
  }

  function take(n: number) {
    note = '';
    value = n;
    text = shown(n);
  }
</script>

<div class="bounded">
  <div class="line">
    <input
      class="field input num"
      type="text"
      inputmode="numeric"
      {id}
      aria-label={label}
      aria-describedby={note ? `${id}-note` : undefined}
      {placeholder}
      bind:value={text}
      oninput={() => (editing = true)}
      onchange={commit}
      onblur={commit}
      onkeydown={(e) => e.key === 'Enter' && commit()}
    />
    <span class="unit">{unit}</span>
    <div class="segmented chips" role="group" aria-label={label}>
      {#each presets as p (p)}
        <button class="chip mono" class:on={value === p} aria-pressed={value === p} onclick={() => take(p)}>{format(p)}</button>
      {/each}
    </div>
  </div>
  {#if note}<p class="note" id="{id}-note" role="status">{note}</p>{/if}
</div>

<style>
  .bounded {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    max-width: 100%;
  }
  .line {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .num {
    width: 64px;
    flex: 0 0 auto;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .unit {
    color: var(--muted);
    font-size: 12px;
  }
  .chips {
    flex: 0 1 auto;
    flex-wrap: wrap;
    justify-content: flex-start;
    max-width: 100%;
  }
  .chip.mono {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
</style>

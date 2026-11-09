<script lang="ts">
  import { settingsForm, type NumberSetting } from '../../lib/settings.svelte';

  // A number of the app's settings (minutes, Mo, days): typed in the draft, saved as it changes; one that is not a
  // number is said by its row (`numberError`, ESC-380), the one saved staying. Its unit beside it.
  let { key, label, hint }: { key: NumberSetting; label: string; hint: string } = $props();

  const s = $derived(settingsForm.settings);
  const bad = $derived(settingsForm.badNumbers.has(key));
</script>

<input
  class="field input num"
  type="number"
  min="0"
  aria-label={label}
  aria-invalid={bad}
  aria-describedby={bad ? `${key}-error` : undefined}
  bind:value={s[key]}
/>
<span class="hint">{hint}</span>

<style>
  .num {
    width: 64px;
    max-width: 100%;
    flex: 0 1 auto;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
</style>

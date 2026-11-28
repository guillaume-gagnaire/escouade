<script lang="ts">
  // « Un mot » (MO-5): a short text for the agent, typed or dictated (M9), kept as a draft on the phone until it goes
  // (M13: a draft, never what the agent said).
  import { onMount } from 'svelte';
  import { t } from '$shared/lib/i18n';
  import { draftOf, keepDraft } from '../lib/kept';
  import Mic from './Mic.svelte';

  let {
    value = $bindable(''),
    label,
    draft,
    placeholder = t('mobile.answer.wordPlaceholder'),
    rows = 2,
  }: { value?: string; label: string; draft: string; placeholder?: string; rows?: number } = $props();

  let field = $state<HTMLTextAreaElement>();

  onMount(() => {
    if (!value) value = draftOf(draft);
    field?.focus({ preventScroll: true });
  });

  function set(v: string) {
    value = v;
    keepDraft(draft, v);
  }
</script>

<div class="word">
  <span class="lb">{label}</span>
  <span class="row">
    <textarea bind:this={field} aria-label={label} {rows} {placeholder} {value} oninput={(e) => set(e.currentTarget.value)}></textarea>
    <Mic onHeard={(h) => set(value ? `${value.trimEnd()} ${h}` : h)} />
  </span>
</div>

<style>
  .word {
    display: grid;
    gap: 6px;
  }
  .lb {
    font-size: var(--small);
    color: var(--muted);
  }
  .row {
    display: flex;
    align-items: flex-start;
    gap: 4px;
    padding: 4px 4px 4px 12px;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  textarea {
    flex: 1;
    min-width: 0;
    min-height: var(--target);
    padding: 10px 0;
    border: 0;
    outline: none;
    resize: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--body);
  }
</style>

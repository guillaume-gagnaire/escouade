<script lang="ts" generics="T extends string | number">
  // One choice among a few, each a chip.
  let {
    value = $bindable(),
    options,
    label,
    mono = false,
  }: { value: T; options: { value: T; label: string; title?: string }[]; label: string; mono?: boolean } = $props();
</script>

<div class="chips" role="group" aria-label={label}>
  {#each options as o (o.value)}
    <button
      class="chip"
      class:mono
      class:on={value === o.value}
      aria-pressed={value === o.value}
      title={o.title}
      onclick={() => (value = o.value)}>{o.label}</button
    >
  {/each}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 4px;
    max-width: 60%;
  }
  .chip {
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  .chip:hover:not(.on) {
    color: var(--text);
  }
  .chip.mono {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
</style>

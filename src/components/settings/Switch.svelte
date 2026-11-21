<script lang="ts">
  // `describedby`: the id of the text that says more of the switch (why it cannot be changed, for instance);
  // `onchange`: told the new value, for a switch whose change goes to the core rather than to a draft.
  let {
    on = $bindable(),
    label,
    disabled = false,
    describedby,
    onchange,
  }: { on: boolean; label: string; disabled?: boolean; describedby?: string; onchange?: (on: boolean) => void } = $props();
</script>

<button
  class="switch"
  class:on
  role="switch"
  aria-checked={on}
  aria-label={label}
  aria-describedby={describedby}
  {disabled}
  onclick={() => {
    on = !on;
    onchange?.(on);
  }}
></button>

<style>
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>

<script lang="ts">
  let { on = $bindable(), label, disabled = false }: { on: boolean; label: string; disabled?: boolean } = $props();
</script>

<button class="switch" class:on role="switch" aria-checked={on} aria-label={label} {disabled} onclick={() => (on = !on)}></button>

<style>
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>

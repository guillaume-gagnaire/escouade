<script lang="ts">
  import { SERVICES } from '../../lib/integrations';
  import type { Service } from '../../lib/types';

  // A ticket source's letter, in its square: on the accounts of « Services connectés » and the sources of
  // « Tickets importés ».
  let { id, small = false }: { id: Service; small?: boolean } = $props();
</script>

<span class="badge" class:small aria-hidden="true">{SERVICES[id].letter}</span>

<style>
  .badge {
    min-width: 24px;
    height: 24px;
    padding: 0;
    flex: none;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.07);
    color: var(--text);
    font-size: 11px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .badge.small {
    min-width: 20px;
    height: 20px;
    border-radius: 5px;
    font-size: 9px;
  }
</style>

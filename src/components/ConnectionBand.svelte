<script lang="ts">
  // The band of the connection to the core (ESC-495): on the remote API, « Connexion au cœur perdue · nouvel essai… »
  // while the window tries again after a cut, or why the tries stopped (a token refused, another version). Never on
  // Tauri's IPC, which loses nothing.
  import { connection } from '../lib/connection.svelte';
  import { t } from '../lib/i18n';
  import Icon from './Icon.svelte';
</script>

{#if connection.state === 'lost' || connection.state === 'refused'}
  <div class="connection-band" class:stopped={connection.state === 'refused'} role="status" data-testid="connection-band">
    <Icon name="remote" size={14} />
    <span class="msg">
      {connection.state === 'lost' ? t('remote.band.lost') : t('remote.band.stopped', { reason: connection.reason })}
    </span>
  </div>
{/if}

<style>
  .connection-band {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--warn-ink);
    background: color-mix(in oklch, var(--warn-ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--warn-ink) 35%, transparent);
  }
  .connection-band.stopped {
    color: var(--del);
    background: color-mix(in oklch, var(--del) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
  }
</style>

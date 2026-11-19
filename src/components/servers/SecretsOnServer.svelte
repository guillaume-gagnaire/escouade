<script lang="ts">
  // Where a secret given on a server is kept (ESC-538, docs/SERVER-UX.md § 4, design `x-compte`): « Gardée dans un fichier
  // sur Travail » — `~/.escouade/secrets.json` of the server, readable by its service's account alone (a server has no
  // keychain, ESC-514 says so in its `welcome`: `connection.secretsInFile`); not in this computer's keychain. Nothing on
  // this computer, or on a server that keeps them otherwise.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import Icon from '../Icon.svelte';

  /** `one`: said of one secret (« Gardée »), else of several (« Gardés »). */
  let { one = false }: { one?: boolean } = $props();
</script>

{#if connection.server && connection.secretsInFile}
  <p class="kept" data-testid="secrets-in-file">
    <Icon name="lock" size={12} />
    <span
      ><b>{t(one ? 'moves.secrets.inFileOne' : 'moves.secrets.inFile', { name: connection.server.name })}</b>
      · {t('moves.secrets.detail')}</span
    >
  </p>
{/if}

<style>
  .kept {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }
  .kept :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .kept b {
    color: var(--soft);
    font-weight: 600;
  }
</style>

<script lang="ts">
  // The status bar's item of a server (ESC-537, docs/SERVER-UX.md § 4): « Travail · ssh acme-escouade · 38 ms », or in
  // amber « Travail · reconnexion dans 4 s » while the connection is cut. Nothing on this computer.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  const server = $derived(connection.server);
  let now = $state(Date.now());
  $effect(() => {
    if (connection.state === 'open') return;
    now = Date.now();
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });
  const seconds = $derived(connection.retryAt ? Math.max(0, Math.ceil((connection.retryAt - now) / 1000)) : 0);
  const ms = $derived(connection.latency ?? 0);
</script>

{#if server}
  <span class="vsep"></span>
  {#if connection.state === 'open'}
    <span class="it tunnel" data-testid="status-tunnel" use:tip={t('servers.status.title', { name: server.name, ssh: server.ssh, ms })}
      ><Icon name="tunnel" size={13} />{t('servers.status.tunnel', {
        name: server.name,
        ssh: server.ssh,
      })}{#if connection.latency !== null}{' · '}<b>{t('servers.status.latency', { ms })}</b>{/if}</span
    >
  {:else}
    <span class="it tunnel cut" data-testid="status-tunnel" role="status" class:refused={connection.state === 'refused'}
      ><Icon name={connection.state === 'refused' ? 'warn' : 'refresh'} size={13} />{connection.state === 'refused'
        ? t('servers.status.refused', { name: server.name })
        : connection.retryAt
          ? t('servers.status.reconnecting', { name: server.name, seconds })
          : connection.state === 'connecting' && connection.tries === 0
            ? t('servers.status.connecting', { name: server.name })
            : t('servers.status.reconnectingNow', { name: server.name })}</span
    >
  {/if}
{/if}

<style>
  .vsep {
    width: 1px;
    height: 12px;
    background: var(--line-2);
  }
  .it {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  b {
    color: var(--soft);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .cut {
    color: var(--wait);
  }
  .cut.refused {
    color: var(--del);
  }
</style>

<script lang="ts">
  // The band of the connection to the core (ESC-495): on the remote API, « Connexion au cœur perdue · nouvel essai… »
  // while the window tries again after a cut, or why the tries stopped (a token refused, another version). Never on
  // Tauri's IPC, which loses nothing.
  //
  // On a server (ESC-537, docs/SERVER-UX.md § 3, design `*-coupure`): it names the space, says when it was cut, when the
  // next try comes and which one, that the agents go on on the server and how old the screen is; « Réessayer » and the
  // way back to this computer. Once back: « Reconnecté à Travail », briefly.
  import { connection } from '../lib/connection.svelte';
  import { fTime } from '../lib/format';
  import { t } from '../lib/i18n';
  import { goHome, nth, retryNow } from '../lib/servers.svelte';
  import { app } from '../lib/state.svelte';
  import Icon from './Icon.svelte';

  const server = $derived(connection.server);
  let now = $state(Date.now());
  $effect(() => {
    if (connection.state !== 'lost') return;
    now = Date.now();
    const timer = setInterval(() => (now = Date.now()), 500);
    return () => clearInterval(timer);
  });
  const seconds = $derived(connection.retryAt ? Math.max(0, Math.ceil((connection.retryAt - now) / 1000)) : 0);
  const at = $derived(connection.lostAt ? fTime(connection.lostAt) : '');

  // « Reconnecté à Travail » once a cut is over.
  let was = connection.state;
  $effect(() => {
    const state = connection.state;
    if (server && was === 'lost' && state === 'open') app.toast(t('servers.band.reconnected', { name: server.name }), 'ok');
    was = state;
  });
</script>

{#if server && connection.state === 'lost'}
  <div class="lostbar" role="alert" data-testid="connection-band">
    <Icon name="warn" size={16} />
    <div class="t">
      <b>{t('servers.band.lost', { name: server.name, time: at })}</b>
      <span
        >{connection.retryAt ? t('servers.band.retryIn', { seconds, nth: nth(connection.tries) }) : t('servers.band.retrying')}
        {t('servers.band.agentsGoOn', { time: at })}</span
      >
    </div>
    <span class="sp"></span>
    <button class="btn" onclick={() => void retryNow()}><Icon name="refresh" size={12} />{t('servers.band.retry')}</button>
    <button class="btn ghost" onclick={() => void goHome()}>{t('servers.backHome')}</button>
  </div>
{:else if !server && (connection.state === 'lost' || connection.state === 'refused')}
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
  .lostbar {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    margin-bottom: 6px;
    padding: 10px 14px 10px 16px;
    border-radius: 8px;
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .lostbar .t {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .lostbar b {
    font-size: 13px;
    font-weight: 640;
    color: var(--wait);
  }
  .lostbar .t span {
    font-size: 12.5px;
    color: var(--soft);
  }
  .sp {
    flex: 1;
  }
</style>

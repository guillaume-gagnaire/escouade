<script lang="ts">
  // Réglages de l'espace › Appareils (ESC-539, docs/SERVER-UX.md § 2 step 8, S5, design `b-appareils`): the devices
  // paired to the server the window is on — this computer marked, its « Se déconnecter » being « Retirer » of
  // Réglages › Serveurs (S15) —, removing another one (its connections close within two seconds), and a pairing code
  // created from here, ten minutes for one device (the phone's way). Read from the server, through the window's own
  // connection: nothing of it is this computer's.
  import { connection } from '../../lib/connection.svelte';
  import { fDate, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { invoke } from '../../lib/transport';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';

  interface Device {
    id: string;
    name: string;
    createdAt: number;
    lastSeen?: number | null;
  }

  const server = $derived(connection.server);
  let devices = $state<Device[] | null>(null);
  let failed = $state('');
  let confirm = $state<Device | null>(null);
  let code = $state<{ code: string; expiresAt: number } | null>(null);
  let busy = $state(false);

  async function load() {
    try {
      devices = await invoke<Device[]>('remote_devices');
      failed = '';
    } catch (e) {
      failed = String(e);
    }
  }

  $effect(() => {
    if (server) void load();
  });

  /** This computer, as the server knows it: by the name it was paired under. */
  const mine = (d: Device) => !!server && d.name === server.device;

  async function revoke(d: Device) {
    confirm = null;
    busy = true;
    try {
      await invoke<boolean>('remote_device_revoke', { id: d.id, by: server?.device ?? null });
      app.toast(t('spaces.devices.revoked', { device: d.name, name: server?.name ?? '' }), 'ok');
      await load();
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function create() {
    busy = true;
    try {
      code = await invoke<{ code: string; expiresAt: number }>('remote_pair_code');
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }
</script>

{#if !server}
  <p class="empty">{t('spaces.devices.unavailable')}</p>
{:else}
  <p class="intro">{t('spaces.devices.intro', { name: server.name, host: server.host })}</p>
  {#if failed}<p class="err" role="alert">{failed}</p>{/if}
  {#if devices}
    <ul class="list" aria-label={t('spaces.devices.title')}>
      {#each devices as d (d.id)}
        <li class="drow" data-testid="device-row">
          <span class="ic" aria-hidden="true"
            ><Icon name={/pixel|phone|iphone|android/i.test(d.name) ? 'phone' : 'monitor'} size={15} /></span
          >
          <div class="main">
            <div class="l1">
              <b>{d.name}</b>
              {#if mine(d)}<span class="mine">{t('spaces.devices.thisOne')}</span>{/if}
            </div>
            <span class="l2"
              >{d.lastSeen ? t('spaces.devices.seen', { date: fDate(d.lastSeen) }) : t('spaces.devices.never')} · {t(
                'spaces.devices.added',
                {
                  date: fDate(d.createdAt),
                },
              )}</span
            >
          </div>
          {#if mine(d)}
            <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'servers' })}>{t('spaces.devices.disconnect')}</button
            >
          {:else}
            <button class="btn ghost danger" disabled={busy} onclick={() => (confirm = d)}>{t('spaces.devices.revoke')}</button>
          {/if}
        </li>
      {:else}
        <li class="empty">{t('spaces.devices.none')}</li>
      {/each}
    </ul>
  {/if}
  {#if confirm}
    {@const c = confirm}
    <div class="confirm" role="alertdialog" aria-label={t('spaces.devices.revokeTitle', { device: c.name, name: server.name })}>
      <b>{t('spaces.devices.revokeTitle', { device: c.name, name: server.name })}</b>
      <p>{t('spaces.devices.revokeText')}</p>
      <div class="acts">
        <button class="btn" onclick={() => (confirm = null)}>{t('common.cancel')}</button>
        <button class="btn primary danger-bg" onclick={() => void revoke(c)}>{t('spaces.devices.revoke')}</button>
      </div>
    </div>
  {/if}
  <div class="pair">
    <div>
      <b>{t('spaces.devices.pairTitle')}</b>
      <p>{t('spaces.devices.pairText')}</p>
      {#if code}
        <p class="code" data-testid="device-code" aria-live="polite">
          {t('spaces.devices.code', { code: code.code })} · {t('spaces.devices.expires', { time: fTime(code.expiresAt) })}
        </p>
      {/if}
    </div>
    <button class="btn" disabled={busy} onclick={() => void create()}><Icon name="key" size={12} />{t('spaces.devices.create')}</button>
  </div>
{/if}

<style>
  .intro,
  .empty {
    margin: 0 0 8px;
    color: var(--soft);
    line-height: 1.5;
  }
  .list {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .drow {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px 0;
    border-top: 1px solid var(--line);
  }
  .ic {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .main {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .l1 {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .l1 b {
    font-weight: 620;
  }
  .l2 {
    color: var(--muted);
    font-size: 12px;
  }
  .mine {
    padding: 1px 7px;
    border-radius: 6px;
    color: var(--run);
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 40%, transparent);
  }
  .danger {
    color: var(--del);
  }
  .danger-bg {
    background: var(--del);
  }
  .err {
    color: var(--del);
  }
  .confirm {
    display: grid;
    gap: 8px;
    margin-top: 12px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .confirm p,
  .pair p {
    margin: 0;
    color: var(--soft);
  }
  .acts {
    display: flex;
    gap: 8px;
  }
  .pair {
    display: flex;
    gap: 16px;
    align-items: center;
    justify-content: space-between;
    padding: 14px 0;
    border-top: 1px solid var(--line);
  }
  .pair > div {
    display: grid;
    gap: 4px;
  }
  .code {
    color: var(--text) !important;
    font-weight: 600;
  }
</style>

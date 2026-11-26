<script lang="ts">
  // Réglages › Téléphone (ESC-567, MO-2 of docs/MOBILE.md; docs/REMOTE.md « Le téléphone »): the phone's access to the
  // core the window is on — this computer's, or a server's (the same page in the settings of a server's space, read
  // through the window's connection). On or off, the UDP port, the tunnel's state, what to do in the box (or the
  // server's firewall), a CGNAT and its ways out, the public address and when it changed, Tailscale, the phones. A policy
  // of the device (`servers.phone`, `servers.remoteApi`) greys the switch with who imposes it. The QR code that pairs a
  // phone is MO-4's.
  import { fDate, fList, fWhen } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { invoke } from '../../lib/transport';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  interface Opened {
    by: 'upnp' | 'natpmp';
    externalPort: number;
    lease: number;
  }
  interface Net {
    local: string | null;
    ipv6: string | null;
    gateway: string | null;
    opened: Opened | null;
    openError: string | null;
    wan: string | null;
    public: string | null;
    cgnat: boolean;
    direct: boolean;
    probed: boolean;
  }
  interface PeerStatus {
    id: string;
    name: string;
    address: string;
    device: string | null;
    addedAt: number;
    sinceHandshake: number | null;
  }
  interface PhoneStatus {
    enabled: boolean;
    state: 'off' | 'starting' | 'on' | 'error';
    error: string | null;
    denied: string | null;
    port: number;
    defaultPort: number;
    bound: string | null;
    tunnelAddress: string;
    api: string;
    publicKey: string | null;
    server: boolean;
    net: Net | null;
    checking: boolean;
    checkedAt: number | null;
    public: { address: string; since: number; previous?: string | null; changedAt?: number | null } | null;
    tailscale: { address: string; name: string | null } | null;
    tailscaleOn: boolean;
    tailscaleServed: string | null;
    stunOn: boolean;
    stunServers: string[];
    peers: PeerStatus[];
    handshakes: number;
    unknown: number;
  }

  let status = $state<PhoneStatus | null>(null);
  let failed = $state('');
  let busy = $state(false);
  let port = $state('');
  let portError = $state('');
  let confirmRotate = $state(false);
  let now = $state(Date.now());

  async function load() {
    try {
      const s = await invoke<PhoneStatus>('phone_status');
      // The port typed and not applied stays as typed.
      if (!status || port === String(status.port)) port = String(s.port);
      status = s;
      failed = '';
    } catch (e) {
      failed = String(e);
    }
    now = Date.now();
  }

  // Read now, then every 3 s while the page is open: the network is looked at in the background.
  $effect(() => {
    void load();
    const timer = setInterval(() => void load(), 3000);
    return () => clearInterval(timer);
  });

  async function change(args: { enabled?: boolean; port?: number; tailscale?: boolean; stun?: boolean }) {
    busy = true;
    try {
      status = await invoke<PhoneStatus>('phone_set', args);
      port = String(status.port);
      failed = '';
    } catch (e) {
      failed = String(e);
      await load();
    } finally {
      busy = false;
    }
  }

  function applyPort() {
    const n = Number(port);
    if (!Number.isInteger(n) || n < 1 || n > 65535) {
      portError = t('phone.port.invalid');
      return;
    }
    portError = '';
    void change({ port: n });
  }

  async function check() {
    busy = true;
    try {
      status = await invoke<PhoneStatus>('phone_check');
    } catch (e) {
      failed = String(e);
    } finally {
      busy = false;
    }
  }

  async function rotate() {
    confirmRotate = false;
    busy = true;
    try {
      status = await invoke<PhoneStatus>('phone_rotate_key');
      app.toast(t('phone.advanced.rotated'), 'ok');
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function remove(p: PeerStatus) {
    try {
      // Its device's token revoked with its peer (SR1-03); a peer never paired as a device had none: said so.
      const r = await invoke<{ removed: boolean; revoked: boolean; neverLinked: boolean }>('phone_remove', { id: p.id });
      app.toast(t(r.neverLinked ? 'phone.phones.removedUnlinked' : 'phone.phones.removed', { name: p.name }), 'ok');
      await load();
    } catch (e) {
      app.toast(String(e), 'error');
    }
  }

  const where = $derived(status?.server ? t('phone.whereServer') : t('phone.whereLocal'));
  const on = $derived(status?.enabled ?? false);
  const stateText = $derived(
    !status
      ? ''
      : status.state === 'on'
        ? t('phone.state.on', { port: status.port })
        : status.state === 'starting'
          ? t('phone.state.starting')
          : status.state === 'error'
            ? t('phone.state.error')
            : t('phone.state.off'),
  );
  const net = $derived(status?.net ?? null);
  /** How long ago the last handshake was, said as « il y a 3 min ». */
  const ago = (ms: number) => {
    const s = Math.floor(ms / 1000);
    return s < 60
      ? t('format.ago.now')
      : s < 3600
        ? t('format.ago.minutes', { n: Math.floor(s / 60) })
        : t('format.ago.hours', { n: Math.floor(s / 3600) });
  };
</script>

<p class="intro">{t('phone.intro', { where })}</p>
{#if failed}<p class="err" role="alert">{failed}</p>{/if}

{#if status}
  <Group title={t('phone.access.title')}>
    <Row label={t('phone.access.label')} desc={t('phone.access.labelDesc')} row="phoneAccess">
      <Switch
        label={t('phone.access.label')}
        {on}
        disabled={busy || (!!status.denied && !on)}
        onchange={(v: boolean) => void change({ enabled: v })}
      />
    </Row>
    <Row label={t('phone.port.label')} desc={t('phone.port.labelDesc')} row="phonePort" error={portError} errorId="phone-port-error">
      <input
        class="field input num"
        type="number"
        min="1"
        max="65535"
        aria-label={t('phone.port.label')}
        aria-invalid={!!portError}
        aria-describedby={portError ? 'phone-port-error' : undefined}
        bind:value={port}
        onkeydown={(e) => e.key === 'Enter' && applyPort()}
      />
      <button class="btn" disabled={busy || port === String(status.port)} onclick={applyPort}>{t('phone.port.apply')}</button>
    </Row>
    <!-- SR1-11: STUN said (who, when, what) and its switch. -->
    <Row
      label={t('phone.stun.label')}
      desc={t('phone.stun.labelDesc', { where, servers: fList(status.stunServers.map((s) => s.replace(/:\d+$/, ''))) })}
      row="phoneStun"
    >
      <Switch label={t('phone.stun.label')} on={status.stunOn} disabled={busy} onchange={(v: boolean) => void change({ stun: v })} />
    </Row>
  </Group>

  <Group title={t('phone.state.title')}>
    <div class="state" data-testid="phone-state" data-state={status.state}>
      <span class="dot {status.state}" aria-hidden="true"></span>
      <div class="lines">
        <b>{stateText}</b>
        {#if status.state === 'error' && (status.error || status.denied)}
          <span class="bad">{status.error ?? status.denied}</span>
        {/if}
        {#if status.state === 'on'}
          <span class="soft mono">{t('phone.state.api', { api: status.api })}</span>
          <span class="soft">{t('phone.state.phones', { count: status.peers.length })}</span>
        {/if}
      </div>
    </div>
  </Group>

  {#if on}
    <Group title={t('phone.box.title')}>
      {#snippet action()}
        <button class="btn ghost" disabled={busy || status?.checking} onclick={() => void check()}
          ><Icon name="refresh" size={12} />{t('phone.public.check')}</button
        >
      {/snippet}
      <div class="box" data-testid="phone-box">
        {#if status.server}
          <p class="tip"><Icon name="server" size={14} />{t('phone.box.server', { port: status.port })}</p>
        {:else if !net}
          <p class="soft">{status.checking ? t('phone.box.checking') : t('phone.box.unchecked')}</p>
        {:else if !net.probed}
          <p class="soft">{t('phone.box.probesOff')}</p>
        {:else if net.direct}
          <p class="tip"><Icon name="globe" size={14} />{t('phone.box.direct', { port: status.port })}</p>
        {:else if net.opened}
          <p class="ok">
            <Icon name="check" size={14} />{t('phone.box.opened', {
              port: net.opened.externalPort,
              by: net.opened.by === 'upnp' ? 'UPnP' : 'NAT-PMP',
            })}
          </p>
        {:else}
          <p class="tip" data-testid="phone-box-todo">
            <Icon name="info" size={14} />{net.local
              ? t('phone.box.todo', { port: status.port, local: net.local })
              : t('phone.box.todoNoLocal', { port: status.port })}
          </p>
          {#if net.openError}<p class="soft small">{t('phone.box.why', { error: net.openError })}</p>{/if}
        {/if}
        {#if net?.cgnat}
          <div class="warn" role="note" data-testid="phone-cgnat">
            <b>{t('phone.box.cgnat')}</b>
            <span>{t('phone.box.cgnatWays')}</span>
          </div>
        {/if}
        {#if net?.ipv6 && !status.server}
          <p class="soft small">{t('phone.box.ipv6', { ip: net.ipv6, port: status.port })}</p>
        {/if}
        <dl class="facts">
          <dt>{t('phone.public.label')}</dt>
          <dd class="mono" data-testid="phone-public">{status.public?.address ?? net?.public ?? t('phone.public.unknown')}</dd>
          {#if net?.wan && net.wan !== net.public}
            <dt>{t('phone.box.wanLabel')}</dt>
            <dd class="mono">{net.wan}</dd>
          {/if}
        </dl>
        {#if status.public?.changedAt && status.public.previous}
          <p class="tip small" data-testid="phone-changed">
            <Icon name="info" size={13} />{t('phone.public.changed', {
              date: fDate(status.public.changedAt),
              before: status.public.previous,
            })}
          </p>
        {/if}
        {#if status.checkedAt}<p class="soft small">{t('phone.public.checked', { when: fWhen(status.checkedAt, now) })}</p>{/if}
      </div>
    </Group>

    <Group title={t('phone.tailscale.title')}>
      <p class="soft para">
        {status.tailscale
          ? t('phone.tailscale.found', {
              address: status.tailscale.name ? `${status.tailscale.address} · ${status.tailscale.name}` : status.tailscale.address,
            })
          : t('phone.tailscale.absent')}
      </p>
      {#if status.tailscale}
        <Row label={t('phone.tailscale.label')} desc={t('phone.tailscale.labelDesc')} row="phoneTailscale">
          <Switch
            label={t('phone.tailscale.label')}
            on={status.tailscaleOn}
            disabled={busy}
            onchange={(v: boolean) => void change({ tailscale: v })}
          />
        </Row>
        {#if status.tailscaleServed}<p class="soft small mono">{t('phone.tailscale.served', { addr: status.tailscaleServed })}</p>{/if}
      {/if}
    </Group>
  {/if}

  <Group title={t('phone.phones.title')}>
    <ul class="list" aria-label={t('phone.phones.title')}>
      {#each status.peers as p (p.id)}
        <li class="drow" data-testid="phone-row">
          <span class="ic" aria-hidden="true"><Icon name="phone" size={15} /></span>
          <div class="main">
            <b>{p.name}</b>
            <span class="l2"
              >{p.sinceHandshake != null ? t('phone.phones.up', { ago: ago(p.sinceHandshake) }) : t('phone.phones.idle')} · {t(
                'phone.phones.address',
                { address: p.address },
              )}</span
            >
          </div>
          <button class="btn ghost danger" onclick={() => void remove(p)}>{t('phone.phones.remove')}</button>
        </li>
      {:else}
        <li class="empty">{t('phone.phones.none')}</li>
      {/each}
    </ul>
  </Group>

  <Group advanced="phone">
    <Row label={t('phone.advanced.rotate')} desc={t('phone.advanced.rotateDesc')} row="phoneKey">
      <button class="btn" disabled={busy} onclick={() => (confirmRotate = true)}
        ><Icon name="key" size={12} />{t('phone.advanced.rotateButton')}</button
      >
    </Row>
    {#if status.publicKey}
      <p class="soft small mono">{t('phone.advanced.key', { key: status.publicKey })}</p>
    {/if}
    {#if confirmRotate}
      <div class="confirm" role="alertdialog" aria-label={t('phone.advanced.rotateTitle')}>
        <b>{t('phone.advanced.rotateTitle')}</b>
        <p>{t('phone.advanced.rotateText', { where })}</p>
        <div class="acts">
          <button class="btn" onclick={() => (confirmRotate = false)}>{t('common.cancel')}</button>
          <button class="btn primary danger-bg" onclick={() => void rotate()}>{t('phone.advanced.rotateButton')}</button>
        </div>
      </div>
    {/if}
  </Group>
{/if}

<style>
  .intro {
    margin: 0 0 8px;
    color: var(--soft);
    line-height: 1.5;
  }
  .err,
  .bad {
    color: var(--del);
  }
  .num {
    width: 88px;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .state {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    margin: 8px 0 14px;
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    margin-top: 5px;
    border-radius: 50%;
    background: var(--muted);
  }
  .dot.on {
    background: var(--run);
  }
  .dot.starting {
    background: var(--wait, var(--muted));
  }
  .dot.error {
    background: var(--del);
  }
  .lines {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .lines b {
    font-weight: 620;
  }
  .soft {
    color: var(--soft);
  }
  .small {
    font-size: 12px;
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .box {
    display: grid;
    gap: 8px;
    margin: 8px 0 14px;
  }
  .box p,
  .para {
    margin: 0;
    line-height: 1.5;
  }
  .para {
    margin: 8px 0 4px;
  }
  .tip,
  .ok {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }
  .tip :global(svg),
  .ok :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .ok {
    color: var(--text);
  }
  .ok :global(svg) {
    color: var(--run);
  }
  .warn {
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border-radius: 9px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 30%, transparent);
    line-height: 1.5;
  }
  .warn span {
    color: var(--soft);
  }
  .facts {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 4px 16px;
    margin: 4px 0 0;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
  }
  .list {
    display: grid;
    margin: 4px 0 8px;
    padding: 0;
    list-style: none;
  }
  .drow {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 10px 0;
    border-top: 1px solid var(--line);
  }
  .drow:first-child {
    border-top: 0;
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
  .main b {
    font-weight: 620;
  }
  .l2 {
    color: var(--muted);
    font-size: 12px;
  }
  .empty {
    padding: 8px 0;
    color: var(--soft);
  }
  .danger {
    color: var(--del);
  }
  .danger-bg {
    background: var(--del);
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
  .confirm p {
    margin: 0;
    color: var(--soft);
  }
  .acts {
    display: flex;
    gap: 8px;
  }
</style>

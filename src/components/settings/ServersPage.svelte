<script lang="ts">
  // Réglages › Serveurs (ESC-537, docs/SERVER-UX.md § 2 step 8, design `x-etats`): the servers this computer knows, each
  // with its state and its gesture — open, try again, copy the ssh command of an unknown key, pair again, remove (S15:
  // revoked on the server when it answers) — and « Se connecter à un serveur ». Nothing here is a server's setting.
  import { connection, type ServerInfo } from '../../lib/connection.svelte';
  import { fDate } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { forgetServer, goHome, goToServer, loadServers, openConnect, retryNow, servers } from '../../lib/servers.svelte';
  import { app } from '../../lib/state.svelte';
  import { lockLine } from '../../lib/locks.svelte';
  import { ownLocks, spacesState } from '../../lib/spaces.svelte';
  import type { LockedBy } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import SpacePlate from '../servers/SpacePlate.svelte';

  $effect(() => {
    void loadServers();
  });

  let busy = $state<string | null>(null);
  let confirm = $state<ServerInfo | null>(null);

  type Pill = { text: string; tone: 'ok' | 'wait' | 'err' | '' };

  function stateOf(s: ServerInfo): Pill {
    if (s.active) {
      if (connection.state === 'open') return { text: t('servers.settings.connected'), tone: 'ok' };
      if (connection.state === 'refused') return pillOf(s) ?? { text: t('servers.settings.refused'), tone: 'err' };
      return { text: t('servers.settings.reconnecting'), tone: 'wait' };
    }
    // Another space (ESC-539): its own connection, kept up for its pile and « Partout ».
    const own = spacesState.watched[s.id]?.conn.state;
    if (own === 'open') return { text: t('servers.settings.connected'), tone: 'ok' };
    if (own === 'lost' || (own === 'connecting' && !s.problem)) return { text: t('servers.settings.reconnecting'), tone: 'wait' };
    return pillOf(s) ?? { text: t('servers.settings.notConnected'), tone: '' };
  }

  /**
   * The policies of this computer on the servers (ESC-539), each greyed with its padlock and who imposes it: what this
   * page cannot change.
   */
  const policies = $derived(
    (
      [
        ['servers.allowed', 'allowed'],
        ['servers.personalSpace', 'personal'],
        ['servers.aggregateView', 'aggregate'],
        ['servers.contentlessNotifications', 'contentless'],
      ] as const
    )
      .map(([key, name]) => {
        const lock = ownLocks().locks[key];
        const by: LockedBy | null | undefined = lock?.by ?? lock?.bounds?.[0]?.by;
        if (!lock || !by) return null;
        const list = [...(Array.isArray(lock.value) ? (lock.value as string[]) : []), ...(lock.bounds ?? []).flatMap((b) => b.allowed)];
        if (name === 'personal' && lock.value !== false) return null;
        if (name === 'aggregate' && lock.value !== false) return null;
        if (name === 'contentless' && lock.value !== true) return null;
        return { key, by, title: t(`spaces.policies.${name}.title`), text: t(`spaces.policies.${name}.text`, { list: list.join(', ') }) };
      })
      .filter((r) => r !== null),
  );

  function pillOf(s: ServerInfo): Pill | null {
    const p = s.problem;
    if (!p) return null;
    switch (p.kind) {
      case 'unreachable':
        return { text: t('servers.settings.unreachable'), tone: 'err' };
      case 'hostKey':
        return { text: t('servers.settings.hostKey'), tone: 'err' };
      case 'noEscouade':
        return { text: t('servers.settings.noEscouade'), tone: 'err' };
      case 'version':
        return { text: t('servers.settings.version'), tone: 'wait' };
      case 'revoked':
        return { text: t('servers.settings.revoked'), tone: 'err' };
      case 'policy':
        return { text: t('servers.settings.policy'), tone: 'err' };
      default:
        return { text: t('servers.settings.refused'), tone: 'err' };
    }
  }

  /** The line under its name: what its state says, or its version and when it was seen. */
  function lineOf(s: ServerInfo): string {
    const p = s.problem;
    if (p?.kind === 'hostKey') return t('servers.problem.hostKey.text', { command: `ssh ${s.ssh}` });
    if (p?.kind === 'unreachable' || p?.kind === 'noEscouade') return p.output.split('\n').at(-1) ?? '';
    if (p?.kind === 'version')
      return p.update === 'client'
        ? t('servers.problem.version.updateClient', { server: p.server })
        : t('servers.problem.version.updateServer', { client: p.client });
    if (p?.kind === 'revoked')
      return p.revoked?.by
        ? t('servers.problem.revoked.when', { date: fDate(p.revoked.at), by: p.revoked.by })
        : p.revoked
          ? t('servers.problem.revoked.whenServer', { date: fDate(p.revoked.at) })
          : t('servers.problem.revoked.unknown');
    if (p?.kind === 'policy' || p?.kind === 'refused') return p.message;
    const version = s.version ?? '?';
    const when = s.lastSeen ? t('servers.settings.seen', { version, date: fDate(s.lastSeen) }) : `Escouade ${version}`;
    const latency = s.active && connection.latency !== null ? ` · ${t('servers.status.latency', { ms: connection.latency })}` : '';
    return `${when}${latency} · ${t('servers.settings.deviceName', { device: s.device })}`;
  }

  async function open(s: ServerInfo) {
    busy = s.id;
    try {
      const problem = await goToServer(s.id);
      if (problem) app.toast(lineOf({ ...s, problem }) || t('servers.problem.other.title'), 'error');
    } finally {
      busy = null;
    }
  }

  async function copy(text: string) {
    await navigator.clipboard.writeText(text).catch(() => {});
    app.toast(t('servers.dialog.pair.copied'), 'ok');
  }

  async function remove(s: ServerInfo) {
    confirm = null;
    busy = s.id;
    try {
      const revoked = await forgetServer(s.id);
      app.toast(
        revoked ? t('servers.settings.removed', { name: s.name }) : t('servers.settings.removedNotRevoked', { name: s.name }),
        'ok',
      );
    } finally {
      busy = null;
    }
  }
</script>

<div class="head">
  <span class="intro"></span>
  <button class="btn primary" onclick={() => openConnect()} data-testid="servers-connect"
    ><Icon name="plus" size={13} />{t('servers.connect')}</button
  >
</div>

{#if servers.view && servers.view.servers.length}
  <ul class="list" aria-label={t('servers.settings.title')}>
    {#each servers.view.servers as s (s.id)}
      {@const pill = stateOf(s)}
      <li class="srow" data-testid="server-row" data-state={s.problem?.kind ?? (s.active ? connection.state : 'idle')}>
        <SpacePlate name={s.name} size={28} cut={pill.tone === 'wait'} />
        <div class="main">
          <div class="l1">
            <b>{s.name}</b>
            <span class="mono host">{s.ssh}{s.host && s.host !== s.ssh ? ` → ${s.host}` : ''}</span>
            <span class="pill {pill.tone}"
              >{#if pill.tone === 'wait'}<Icon name="refresh" size={11} />{/if}{pill.text}</span
            >
          </div>
          <span class="l2">{lineOf(s)}</span>
        </div>
        <div class="acts">
          {#if s.active}
            {#if connection.state === 'open'}
              <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
            {:else}
              <button class="btn" onclick={() => void retryNow()}>{t('servers.problem.retry')}</button>
            {/if}
          {:else if s.problem?.kind === 'hostKey'}
            <button class="btn" onclick={() => void copy(`ssh ${s.ssh}`)}
              ><Icon name="copy" size={12} />{t('servers.problem.hostKey.copy')}</button
            >
          {:else if s.problem?.kind === 'revoked'}
            <button class="btn" onclick={() => openConnect(s.id)}>{t('servers.problem.revoked.again')}</button>
          {:else}
            <button class="btn" disabled={busy === s.id} onclick={() => void open(s)}
              >{s.problem ? t('servers.problem.retry') : t('servers.settings.open')}</button
            >
          {/if}
          <button class="btn ghost danger" disabled={busy === s.id} onclick={() => (confirm = s)}>{t('servers.settings.remove')}</button>
        </div>
      </li>
    {/each}
  </ul>
{:else if servers.view}
  <p class="empty">{t('servers.settings.empty')}</p>
{/if}

{#if policies.length}
  <h3 class="sub">{t('spaces.policies.title')}</h3>
  <ul class="list">
    {#each policies as r (r.key)}
      <li class="lockrow" data-testid="server-policy" data-key={r.key}>
        <b>{r.title}</b>
        <span class="lk"><Icon name="lock" size={12} />{lockLine(r.by)}</span>
        <span class="d">{r.text}</span>
      </li>
    {/each}
  </ul>
{/if}

{#if servers.view}
  <p class="foot">
    <Icon name="key" size={12} />{servers.view.tokensInFile ? t('servers.settings.tokensInFile') : t('servers.settings.tokensInKeychain')}
  </p>
{/if}

{#if confirm}
  {@const c = confirm}
  <div class="confirm" role="alertdialog" aria-label={t('servers.settings.removeTitle', { name: c.name })}>
    <b>{t('servers.settings.removeTitle', { name: c.name })}</b>
    <p>{t('servers.settings.removeText')}</p>
    <div class="acts">
      <button class="btn" onclick={() => (confirm = null)}>{t('common.cancel')}</button>
      <button class="btn primary danger-bg" onclick={() => void remove(c)}>{t('servers.settings.remove')}</button>
    </div>
  </div>
{/if}

<style>
  .head {
    display: flex;
    gap: 16px;
    align-items: flex-start;
    margin-bottom: 8px;
  }
  .intro {
    flex: 1;
    margin: 0;
    color: var(--soft);
    line-height: 1.5;
  }
  .list {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .srow {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 12px 0;
    border-top: 1px solid var(--line);
  }
  .main {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .l1 {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .l1 b {
    font-weight: 620;
  }
  .host {
    color: var(--muted);
    font-size: 12px;
  }
  .l2 {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    padding: 0 7px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    color: var(--soft);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .pill.ok {
    color: var(--run);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 40%, transparent);
  }
  .pill.wait {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .pill.err {
    color: var(--del);
    background: color-mix(in oklch, var(--del) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 40%, transparent);
  }
  .acts {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .danger {
    color: var(--del);
  }
  .danger-bg {
    background: var(--del);
  }
  .empty,
  .foot {
    color: var(--muted);
    font-size: 12.5px;
  }
  .foot {
    display: flex;
    gap: 6px;
    align-items: center;
    margin-top: 12px;
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
  .sub {
    margin: 18px 0 4px;
    font-size: 13px;
    font-weight: 640;
  }
  .lockrow {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    align-items: baseline;
    padding: 10px 0;
    border-top: 1px solid var(--line);
    color: var(--muted);
  }
  .lockrow b {
    color: var(--soft);
    font-weight: 620;
  }
  .lk {
    display: inline-flex;
    gap: 5px;
    align-items: center;
    font-size: 12px;
    font-weight: 600;
  }
  .d {
    font-size: 12px;
  }
</style>

<script lang="ts">
  // Réglages › Général of a server's space (ESC-702): the server's own updates, not this computer's — its version, the
  // version it found and where it stands (put in place, waiting for rest, a new image), « Mettre à jour le serveur »
  // (`server_update_start`, a computer's alone, after a confirmation), and its two switches, which are the server's
  // settings (`autoUpdate`, `betaUpdates`), within its policies. Read from the server every few seconds while shown;
  // an error is said once, never asked again by itself.
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';
  import { invoke } from '../../lib/transport';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  interface State {
    version: string;
    phase: 'waiting' | 'checking' | 'upToDate' | 'available' | 'image' | 'downloading' | 'ready' | 'restarting' | 'failed';
    available?: string;
    automatic: boolean;
    channel: 'stable' | 'beta';
    image: boolean;
    installs: boolean;
    command?: string;
    error?: string;
    busy?: string;
  }

  let { name }: { name: string } = $props();
  const s = $derived(settingsForm.settings);
  let info = $state<State | null>(null);
  let confirming = $state(false);
  let asking = $state(false);
  let asked = $state('');
  let failed = $state('');
  let copied = $state(false);
  let timer: ReturnType<typeof setInterval> | undefined;

  async function load() {
    try {
      info = await invoke<State | null>('server_update_state');
    } catch {
      info = null;
    }
  }

  onMount(() => {
    void load();
    timer = setInterval(() => void load(), 5000);
  });
  onDestroy(() => clearInterval(timer));

  const BUSY = ['turn', 'question', 'setup', 'validation', 'work', 'background', 'launch', 'terminal'] as const;
  const busyText = (why: string) =>
    (BUSY as readonly string[]).includes(why) ? t(`spaces.update.busy.${why as (typeof BUSY)[number]}`) : t('spaces.update.busy.other');

  const channel = $derived(info?.channel === 'beta' ? t('spaces.update.beta') : t('spaces.update.stable'));

  /** The line under the version: where the update stands. */
  const line = $derived.by(() => {
    if (!info) return '';
    switch (info.phase) {
      case 'waiting':
        return t('spaces.update.waiting');
      case 'checking':
        return t('spaces.update.checking');
      case 'upToDate':
        return t('spaces.update.upToDate', { channel });
      case 'downloading':
        return t('spaces.update.downloading', { version: info.available ?? '' });
      case 'restarting':
        return t('spaces.update.restarting', { version: info.available ?? '' });
      default:
        return '';
    }
  });

  async function start(version: string) {
    confirming = false;
    asking = true;
    failed = '';
    try {
      asked = await invoke<string>('server_update_start', { version });
      await load();
    } catch (e) {
      failed = String(e);
    } finally {
      asking = false;
    }
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      // The command stays shown, to select.
    }
  }
</script>

{#snippet command(text: string)}
  <div class="code">
    <code>{text}</code>
    <button class="btn ghost" onclick={() => void copy(text)}
      ><Icon name={copied ? 'check' : 'copy'} size={12} />{copied
        ? t('servers.problem.version.copied')
        : t('servers.problem.version.copy')}</button
    >
  </div>
{/snippet}

<Group title={t('spaces.update.title', { name })} data-testid="server-updates" data-phase={info?.phase ?? ''}>
  <Row label={t('spaces.update.version', { version: info?.version ?? '…' })} desc={line} />
  {#if info?.phase === 'image' && info.available}
    <Row label={t('spaces.update.image', { version: info.available })} desc={t('spaces.update.imageDesc')}>
      {#snippet extra()}{#if info?.command}{@render command(info.command)}{/if}{/snippet}
    </Row>
  {:else if info?.phase === 'ready' && info.available}
    <Row
      label={t('spaces.update.ready', { version: info.available })}
      desc={info.busy ? t('spaces.update.readyWaits', { why: busyText(info.busy) }) : t('spaces.update.readyDesc')}
    />
  {:else if info && (info.phase === 'available' || info.phase === 'failed') && info.available}
    {@const version = info.available}
    <Row
      label={t('spaces.update.available', { version })}
      desc={info.phase === 'failed'
        ? t('spaces.update.failed', { error: info.error ?? '' })
        : info.installs
          ? t('spaces.update.off')
          : t('spaces.update.noSystemd')}
    >
      {#snippet extra()}
        {#if !info?.installs && info?.command}{@render command(info.command)}{/if}
        {#if confirming}
          <div class="confirm" role="group" aria-label={t('servers.problem.version.confirmTitle', { name, version })}>
            <b>{t('servers.problem.version.confirmTitle', { name, version })}</b>
            <span>{t('servers.problem.version.confirmText', { version })}</span>
            <div class="acts">
              <button class="btn primary" onclick={() => void start(version)}>{t('servers.problem.version.confirm')}</button>
              <button class="btn" onclick={() => (confirming = false)}>{t('common.cancel')}</button>
            </div>
          </div>
        {/if}
        {#if asked}<span class="ok" role="status">{t('spaces.update.asked', { version: asked })}</span>{/if}
        {#if failed}<span class="err" role="alert" data-testid="server-update-failed">{failed}</span>{/if}
      {/snippet}
      {#if info.installs && !confirming}
        <button class="btn primary" disabled={asking} onclick={() => (confirming = true)}
          ><Icon name="download" size={12} />{asking
            ? t('servers.problem.version.asking')
            : t('servers.problem.version.updateServerButton')}</button
        >
      {/if}
    </Row>
  {:else if info?.phase === 'failed' && info.error}
    <Row label={t('spaces.update.failed', { error: info.error })} />
  {/if}
  <Row label={t('spaces.update.auto')} desc={t('spaces.update.autoDesc')} row="autoUpdate">
    <Switch label={t('spaces.update.auto')} bind:on={s.autoUpdate} />
  </Row>
  <Row label={t('spaces.update.betaChannel')} desc={t('spaces.update.betaChannelDesc')} row="betaUpdates">
    <Switch label={t('spaces.update.betaChannel')} bind:on={s.betaUpdates} />
  </Row>
</Group>

<style>
  .code {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
    margin-top: 6px;
    padding: 6px 6px 6px 12px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .code code {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--code-fg);
    overflow-wrap: anywhere;
  }
  .confirm {
    display: grid;
    gap: 8px;
    margin-top: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    line-height: 1.5;
  }
  .confirm b {
    color: var(--text);
    font-weight: 620;
  }
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .ok {
    display: block;
    margin-top: 6px;
    color: var(--soft);
  }
  .err {
    display: block;
    margin-top: 6px;
    color: var(--del);
  }
</style>

<script lang="ts">
  // « Mettre à jour le serveur » (ESC-702) on the refusal of the versions, when the server is the one to update: the
  // button asks it of the server by this computer's token (`server_update_ask`, a computer's alone, never a phone's),
  // after a confirmation that says what follows; in the image, the `docker pull` to copy. An error is said once, with
  // the button to try again: never a loop. The server's answer is shown as text, never run.
  import { connection, type ServerUpdate } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { goHome, goToServer } from '../../lib/servers.svelte';
  import { local } from '../../lib/transport';
  import Icon from '../Icon.svelte';

  let {
    id,
    name,
    version,
    how,
  }: {
    /** The server's id on this computer. */
    id: string;
    name: string;
    /** This computer's version: the one the server is asked to take. */
    version: string;
    /** What the refusal says of the server's update (`serverUpdate`). */
    how?: ServerUpdate;
  } = $props();

  type Step = 'idle' | 'confirm' | 'asking' | 'asked' | 'failed';
  let step = $state<Step>('idle');
  let error = $state('');
  let target = $state('');
  let copied = $state(false);

  const image = $derived(how?.how === 'image');

  async function ask() {
    step = 'asking';
    try {
      target = await local<string>('server_update_ask', { id, version });
      step = 'asked';
    } catch (e) {
      error = String(e);
      step = 'failed';
    }
  }

  /** The server tried again (its tunnel, a handshake): the window loads on it once it answers; still refused, the new refusal is shown (another version, the image). */
  async function reconnect() {
    const p = await goToServer(id).catch(() => null);
    if (p) connection.problem = p;
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

  // Asked: the window tries the server again every 30 s, until it is back on the new version (a handshake, never the
  // update asked again).
  $effect(() => {
    if (step !== 'asked') return;
    const timer = setInterval(() => void reconnect(), 30_000);
    return () => clearInterval(timer);
  });
</script>

<div class="upd" data-testid="update-server" data-step={image ? 'image' : step}>
  {#if image}
    <p>{t('servers.problem.version.image')}</p>
  {:else if step === 'confirm'}
    <div class="confirm" role="group" aria-label={t('servers.problem.version.confirmTitle', { name, version })}>
      <b>{t('servers.problem.version.confirmTitle', { name, version })}</b>
      <p>{t('servers.problem.version.confirmText', { version })}</p>
      <div class="acts">
        <button class="btn primary" onclick={() => void ask()}>{t('servers.problem.version.confirm')}</button>
        <button class="btn" onclick={() => (step = 'idle')}>{t('common.cancel')}</button>
      </div>
    </div>
  {:else if step === 'asked'}
    <p role="status" data-testid="update-asked">
      <Icon name="check" size={12} />{t('servers.problem.version.asked', { name, version: target || version })}
    </p>
  {:else if step === 'failed'}
    <p class="err" role="alert" data-testid="update-failed">{t('servers.problem.version.failed', { error })}</p>
  {/if}

  {#if how?.command && (image || step !== 'confirm')}
    {#if !image}<span class="lbl">{t('servers.problem.version.byHand')}</span>{/if}
    <div class="code">
      <code>{how.command}</code>
      <button class="btn ghost" onclick={() => void copy(how.command)}
        ><Icon name={copied ? 'check' : 'copy'} size={12} />{copied
          ? t('servers.problem.version.copied')
          : t('servers.problem.version.copy')}</button
      >
    </div>
  {/if}

  {#if step !== 'confirm'}
    <div class="acts">
      {#if image}
        <button class="btn primary" onclick={() => void reconnect()}><Icon name="refresh" size={12} />{t('servers.problem.retry')}</button>
      {:else if step === 'asked'}
        <button class="btn primary" onclick={() => void reconnect()}
          ><Icon name="refresh" size={12} />{t('servers.problem.version.reconnect')}</button
        >
      {:else}
        <button class="btn primary" disabled={step === 'asking'} onclick={() => (step = 'confirm')}
          ><Icon name="download" size={12} />{step === 'asking'
            ? t('servers.problem.version.asking')
            : t('servers.problem.version.updateServerButton')}</button
        >
        <!-- Updated by hand on the server: the window tries it again. -->
        <button class="btn" onclick={() => void reconnect()}><Icon name="refresh" size={12} />{t('servers.problem.retry')}</button>
      {/if}
      <button class="btn" onclick={() => void goHome()}>{t('servers.backHome')}</button>
    </div>
  {/if}
</div>

<style>
  .upd {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  p {
    display: flex;
    gap: 6px;
    align-items: baseline;
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
    max-width: 62ch;
  }
  .err {
    color: var(--del);
  }
  .confirm {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .confirm b {
    color: var(--text);
    font-weight: 620;
  }
  .lbl {
    font-size: 12px;
    font-weight: 600;
    color: var(--soft);
  }
  .code {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
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
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }
</style>

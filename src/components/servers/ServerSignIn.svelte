<script lang="ts">
  // The sign-in of an account on a server (ESC-538, S13, docs/SERVER-UX.md § 4, design `x-compte`): the harness runs on
  // the server, its browser cannot. The address its terminal gives opens in this computer's browser, at the user's click
  // (nothing the server sends opens by itself); when the harness waits for the page's return on `localhost`, this
  // computer listens on that same port and relays it to the server (`server_relay`, same port); else the code the page
  // shows is pasted in the terminal, or a device's code typed on the page. The account is kept on the server.
  import { t } from '../../lib/i18n';
  import { loginLink, relay, spaceName } from '../../lib/server-moves.svelte';
  import { local } from '../../lib/transport';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  let { text }: { text: string } = $props();

  const link = $derived(loginLink(text));
  let opened = $state<string | null>(null);
  /** The return relayed (its port), refused (the port taken here), or not asked. */
  let back = $state<{ port: number; ok: boolean } | null>(null);
  let copied = $state(false);
  const name = $derived(spaceName());

  async function open() {
    const l = link;
    if (!l) return;
    if (l.port !== null && back?.port !== l.port) {
      try {
        await relay(l.port, true);
        back = { port: l.port, ok: true };
      } catch {
        back = { port: l.port, ok: false };
      }
    }
    await local('open_url', { url: l.url });
    opened = l.url;
  }

  async function copy() {
    if (!link) return;
    await navigator.clipboard.writeText(link.url).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="signin" data-testid="server-signin">
  <p class="step" class:done={!!opened}>
    <span class="n"
      >{#if opened}<Icon name="check" size={11} />{:else}1{/if}</span
    >
    <span>
      {#if opened}{t('moves.login.opened')}{:else if link}{t('moves.login.ready')}{:else}{t('moves.login.waiting')}{/if}
    </span>
  </p>
  {#if link}
    <p class="step">
      <span class="n">2</span>
      <span>
        {#if back?.ok}{t('moves.login.relayed', { port: back.port, name })}
        {:else if back && !back.ok}{t('moves.login.relayBusy', { port: back.port })}
        {:else if link.device}{t('moves.login.device')}
        {:else if link.port === null}{t('moves.login.paste')}
        {:else}{t('moves.login.relayed', { port: link.port, name })}{/if}
        <b>{t('moves.login.kept', { name })}</b>
      </span>
    </p>
    <div class="acts">
      <button class="btn primary" onclick={open}
        ><Icon name="ext" size={13} />{opened ? t('moves.login.again') : t('moves.login.open')}</button
      >
      <button class="btn ghost" onclick={copy}
        ><Icon name="copy" size={13} />{copied ? t('moves.login.copied') : t('moves.login.copy')}</button
      >
      <span class="url mono" use:tip={{ text: link.url, cut: true }}>{link.url}</span>
    </div>
  {/if}
</div>

<style>
  .signin {
    display: grid;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12.5px;
  }
  .step {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 0;
    line-height: 1.45;
    color: var(--soft);
  }
  .step b {
    font-weight: 600;
    color: var(--text);
  }
  .n {
    flex: none;
    display: grid;
    place-items: center;
    min-width: 20px;
    height: 20px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--wait);
    font-size: 11px;
    font-weight: 600;
  }
  .done .n {
    box-shadow: inset 0 0 0 1px var(--run);
    color: var(--run);
  }
  .acts {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .url {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 11px;
  }
</style>

<script lang="ts">
  // « Appairer un téléphone » (ESC-570, MO-4 of docs/MOBILE.md § 3.1): the QR code the core the window is on makes —
  // this computer's (Réglages › Téléphone) or a server's (Réglages de l'espace › Appareils) —, valid ten minutes for one
  // phone, the 8-character code beside it for an app that cannot scan, how long it still holds and where the phone will
  // reach it. It is a secret: never copied, never logged; it leaves the screen once a phone paired through it, when it
  // ends, or when the page is left. A new one takes the place of the one before.
  import { t } from '../../lib/i18n';
  import { invoke } from '../../lib/transport';
  import Icon from '../Icon.svelte';

  interface PairQr {
    svg: string;
    code: string;
    expiresAt: number;
    peer: string;
    endpoint: string;
    endpointUnknown: boolean;
  }

  let { onpaired }: { onpaired?: () => void } = $props();

  let qr = $state<PairQr | null>(null);
  let failed = $state('');
  let busy = $state(false);
  let done = $state(false);
  let expired = $state(false);
  let now = $state(Date.now());

  async function make() {
    busy = true;
    failed = '';
    done = false;
    expired = false;
    try {
      qr = await invoke<PairQr>('phone_pair_qr');
    } catch (e) {
      failed = String(e);
      qr = null;
    } finally {
      busy = false;
    }
  }

  // While it is shown: its time, and whether a phone paired through it (its peer no longer waiting).
  $effect(() => {
    if (!qr) return;
    const shown = qr;
    const tick = setInterval(() => {
      now = Date.now();
      if (now >= shown.expiresAt) {
        qr = null;
        expired = true;
      }
    }, 1000);
    const watch = setInterval(async () => {
      try {
        const s = await invoke<{ peers: { id: string; pending: boolean; device: string | null }[] }>('phone_status');
        const peer = s.peers.find((p) => p.id === shown.peer);
        if (peer && !peer.pending && peer.device && qr === shown) {
          qr = null;
          done = true;
          onpaired?.();
        }
      } catch {
        // Read again at the next turn.
      }
    }, 2000);
    return () => {
      clearInterval(tick);
      clearInterval(watch);
    };
  });

  const left = $derived(qr ? Math.max(0, Math.ceil((qr.expiresAt - now) / 1000)) : 0);
  const leftText = $derived(`${Math.floor(left / 60)} min ${String(left % 60).padStart(2, '0')} s`);
</script>

<div class="pairqr" data-testid="pair-qr">
  {#if qr}
    <div class="shown" role="region" aria-label={t('phone.qr.title')}>
      <!-- The SVG the core drew (qrcode): dark on light, whatever the theme, as cameras read it. -->
      <div class="code-img" aria-hidden="true">{@html qr.svg}</div>
      <div class="side">
        <b>{t('phone.qr.scan')}</b>
        <p class="soft">{t('phone.qr.orCode')}</p>
        <p class="letters" data-testid="pair-qr-code" aria-label={t('phone.qr.codeLabel')}>{qr.code}</p>
        <p class="soft small" data-testid="pair-qr-left">{t('phone.qr.left', { left: leftText })}</p>
        <p class="soft small mono">{t('phone.qr.endpoint', { endpoint: qr.endpoint })}</p>
        {#if qr.endpointUnknown}<p class="warn small">{t('phone.qr.endpointUnknown')}</p>{/if}
        <p class="secret small"><Icon name="lock" size={12} />{t('phone.qr.secret')}</p>
        <div class="acts">
          <button class="btn ghost" onclick={() => (qr = null)}>{t('phone.qr.hide')}</button>
        </div>
      </div>
    </div>
  {:else}
    {#if done}<p class="ok" role="status"><Icon name="check" size={14} />{t('phone.qr.paired')}</p>{/if}
    {#if expired}<p class="soft" role="status">{t('phone.qr.expired')}</p>{/if}
    {#if failed}<p class="err" role="alert">{failed}</p>{/if}
    <button class="btn primary" disabled={busy} onclick={() => void make()}
      ><Icon name="phone" size={13} />{expired || done ? t('phone.qr.again') : t('phone.qr.make')}</button
    >
  {/if}
</div>

<style>
  .pairqr {
    display: grid;
    gap: 10px;
    justify-items: start;
  }
  .shown {
    display: flex;
    gap: 18px;
    align-items: flex-start;
    flex-wrap: wrap;
    padding: 14px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .code-img {
    width: 220px;
    height: 220px;
    flex: none;
    border-radius: 8px;
    overflow: hidden;
    background: #fff;
  }
  .code-img :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .side {
    display: grid;
    gap: 6px;
    min-width: 200px;
    flex: 1;
  }
  .side p {
    margin: 0;
  }
  .letters {
    font-family: var(--mono);
    font-size: 26px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: var(--text);
    user-select: none;
  }
  .soft {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
  .mono {
    font-family: var(--mono);
  }
  .warn {
    color: var(--warn-ink);
  }
  .secret {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    color: var(--soft);
  }
  .ok {
    display: inline-flex;
    gap: 6px;
    align-items: center;
    color: var(--done);
    margin: 0;
  }
  .err {
    color: var(--del);
    margin: 0;
  }
  .acts {
    margin-top: 6px;
  }
</style>

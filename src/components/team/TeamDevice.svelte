<script lang="ts">
  // « Ajouter un de tes appareils » (TS-3, §3.4): a QR code (the link of one's own device, an hour), or, without a camera,
  // a short code of 10 characters for a PAKE through the server (one try, 10 minutes). The device in the team checks the
  // try, and sends the invitation sealed under the PAKE's key; it then lets the new device in.
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { errorText, teams, teamsApi, type InviteView, type PairView } from '../../lib/teams.svelte';
  import InviteLink from './InviteLink.svelte';
  import TeamAdmissions from './TeamAdmissions.svelte';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  let tab = $state<'qr' | 'code'>('qr');
  let invite = $state<InviteView | null>(null);
  let pair = $state<PairView | null>(null);
  let failure = $state('');
  /** The team as the backend last said it: the new device's request comes in it, with its six words (SR2-08). */
  const current = $derived(teams.byId(teamId));
  let timer: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    try {
      invite = await teamsApi.ownDevice(teamId);
    } catch (e) {
      failure = errorText(e);
    }
  });

  function stop() {
    if (timer) clearInterval(timer);
    timer = null;
  }

  onDestroy(() => {
    stop();
    if (pair && pair.state !== 'sent') void teamsApi.pairCancel(teamId);
  });

  async function startCode() {
    stop();
    failure = '';
    try {
      pair = await teamsApi.pairStart(teamId);
      timer = setInterval(step, 1000);
    } catch (e) {
      failure = errorText(e);
    }
  }

  let stepping = false;
  async function step() {
    if (stepping || !pair) return;
    stepping = true;
    try {
      pair = await teamsApi.pairStep(teamId);
      if (pair.state === 'sent' || pair.state === 'wrongCode') stop();
    } catch (e) {
      failure = errorText(e);
      stop();
    } finally {
      stepping = false;
    }
  }

  const said = $derived(
    !pair
      ? ''
      : pair.state === 'sent'
        ? t('team.device.sent')
        : pair.state === 'wrongCode'
          ? t('team.device.wrongCode')
          : pair.state === 'waitingForToken'
            ? t('team.device.waitingForToken')
            : t('team.device.waiting'),
  );
</script>

<div class="segmented tabs" role="tablist">
  <button role="tab" aria-selected={tab === 'qr'} class:on={tab === 'qr'} onclick={() => (tab = 'qr')}>{t('team.device.qrTab')}</button>
  <button role="tab" aria-selected={tab === 'code'} class:on={tab === 'code'} onclick={() => (tab = 'code')}
    >{t('team.device.codeTab')}</button
  >
</div>

{#if tab === 'qr'}
  <p class="lead">{t('team.device.qrLead')}</p>
  {#if invite}
    <InviteLink {invite} label={t('team.invite.link')} qrLabel={t('team.invite.qr')} />
    {#if !invite.ready}<p class="wait">{t('team.invite.notReady')}</p>{/if}
  {/if}
{:else}
  <p class="lead">{t('team.device.codeLead')}</p>
  {#if pair}
    <div class="code" aria-live="polite">
      <span class="lbl">{t('team.device.code')}</span>
      <strong class="mono" data-testid="pair-code">{pair.code}</strong>
      <span class="hint">{t('team.device.codeServer', { server: pair.server })}</span>
    </div>
    <p class="state" class:bad={pair.state === 'wrongCode'} class:ok={pair.state === 'sent'} role="status">{said}</p>
    {#if pair.state === 'wrongCode'}
      <div><button class="btn" onclick={startCode}>{t('team.device.again')}</button></div>
    {/if}
  {:else}
    <div><button class="btn primary" onclick={startCode}>{t('team.device.make')}</button></div>
  {/if}
  <p class="hint">{t('team.device.oneTry')}</p>
{/if}

{#if current}<TeamAdmissions team={current} />{/if}

{#if failure}<p class="error" role="alert">{failure}</p>{/if}
<div class="foot">
  <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
</div>

<style>
  .tabs {
    align-self: flex-start;
  }
  .lead,
  .hint,
  .error,
  .state,
  .wait {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .hint {
    color: var(--dim);
  }
  .error,
  .state.bad {
    color: var(--del);
  }
  .state,
  .wait {
    color: var(--wait);
  }
  .state.ok {
    color: var(--add);
  }
  .code {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 18px;
    border-radius: 12px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .code strong {
    color: var(--text);
    font-size: 30px;
    font-weight: 600;
    letter-spacing: 0.12em;
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>

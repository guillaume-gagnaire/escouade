<script lang="ts">
  // « Inscrire une machine de l'équipe » (OE-1): an administrator's short code, for a machine under the role of service
  // (an hour, one try). It is typed on the machine, `escouade-server team pair <serveur> <CODE>` (or « Ajouter cet
  // appareil par un code » in the window connected to it); the machine's request then waits for its six words.
  import { onDestroy, onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { errorText, teams, teamsApi, type PairView } from '../../lib/teams.svelte';
  import TeamAdmissions from './TeamAdmissions.svelte';

  let { teamId }: { teamId: string } = $props();

  let pair = $state<PairView | null>(null);
  let failure = $state('');
  const current = $derived(teams.byId(teamId));
  let timer: ReturnType<typeof setInterval> | null = null;

  function stop() {
    if (timer) clearInterval(timer);
    timer = null;
  }

  async function start() {
    stop();
    failure = '';
    try {
      pair = await teamsApi.pairStart(teamId, true);
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

  onMount(start);
  onDestroy(() => {
    stop();
    if (pair && pair.state !== 'sent') void teamsApi.pairCancel(teamId);
  });

  const said = $derived(
    !pair
      ? ''
      : pair.state === 'sent'
        ? t('team.machines.sent')
        : pair.state === 'wrongCode'
          ? t('team.device.wrongCode')
          : pair.state === 'waitingForToken'
            ? t('team.device.waitingForToken')
            : t('team.device.waiting'),
  );
</script>

<p class="lead">{t('team.machines.role')}</p>
<p class="lead">{t('team.machines.lead2')}</p>
{#if pair}
  <div class="code" aria-live="polite">
    <span class="lbl">{t('team.device.code')}</span>
    <strong class="mono" data-testid="machine-code">{pair.code}</strong>
    <code class="cmd mono" data-testid="machine-command">{t('team.machines.command', { server: pair.server, code: pair.code })}</code>
  </div>
  <p class="hint">{t('team.machines.window')}</p>
  <p class="state" class:bad={pair.state === 'wrongCode'} class:ok={pair.state === 'sent'} role="status">{said}</p>
  {#if pair.state === 'wrongCode'}
    <div><button class="btn" onclick={start}>{t('team.device.again')}</button></div>
  {/if}
{/if}
<p class="hint">{t('team.device.oneTry')}</p>
{#if failure}<p class="error" role="alert">{failure}</p>{/if}

{#if current}<TeamAdmissions team={current} />{/if}

<style>
  .lead,
  .hint {
    margin: 0 0 8px;
    max-width: 64ch;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
  }
  .lead {
    color: var(--text);
  }
  .code {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 4px 0 10px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  strong {
    font-size: 22px;
    letter-spacing: 0.08em;
    color: var(--text);
  }
  .cmd {
    font-size: 12px;
    color: var(--soft);
    user-select: all;
    overflow-wrap: anywhere;
  }
  .state {
    margin: 0 0 8px;
    font-size: 12.5px;
    color: var(--soft);
  }
  .state.bad,
  .error {
    color: var(--del);
  }
  .error {
    font-size: 12px;
  }
</style>

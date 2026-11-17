<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { lockedBy, lockLine } from '../../lib/locks.svelte';
  import { leave, statusOf, type Primary } from '../../lib/onboarding.svelte';
  import { openUrl } from '../../lib/opener';
  import { app } from '../../lib/state.svelte';
  import { privacyUrl } from '../../lib/telemetry';
  import Icon from '../Icon.svelte';
  import StationHead from './StationHead.svelte';

  // The last station, the statistics (docs/ONBOARDING.md § 4.6, choice O7): the same question as the window of the first
  // start had (`shell.telemetry.*`), the same setting (`telemetry_set`). Neither answer is put forward; what stays on the
  // machine is said in a line. Imposed by a policy: said, locked, and passed in one « Continuer ».
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  const imposed = $derived(statusOf('telemetry') === 'policy' ? lockedBy('telemetry') : null);
  let busy = $state(false);

  /** The answer, kept at once, then the end. */
  async function answer(consent: boolean) {
    busy = true;
    try {
      await api.telemetrySet(consent);
      app.settings = { ...app.settings, telemetry: consent };
      await leave('telemetry', 'done');
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    // The two answers are the station's own buttons: a main button only once there is nothing to ask.
    primary = imposed ? { label: t('onboarding.continue'), run: async () => true } : null;
  });

  const more = () => openUrl(privacyUrl()).catch((e) => app.toast(String(e), 'error'));
</script>

<div class="q" data-testid="onboarding-telemetry">
  <StationHead step="telemetry" question={t('shell.telemetry.title')}>
    {#snippet why()}{t('shell.telemetry.body')}{/snippet}
  </StationHead>
  <p class="p">{t('shell.telemetry.detail')}</p>
  <p class="p stays"><Icon name="lock" size={12} />{t('onboarding.telemetry.stays')}</p>
  <button class="link" onclick={more}><Icon name="ext" size={14} />{t('shell.telemetry.more')}</button>
  {#if imposed}
    <p class="p lock">
      <Icon name="lock" size={12} />{app.settings.telemetry ? t('onboarding.telemetry.imposedOn') : t('onboarding.telemetry.imposedOff')}
      · {lockLine(imposed)}
    </p>
  {:else}
    {#if app.settings.telemetry !== null && app.settings.telemetry !== undefined}
      <p class="p">
        {t('onboarding.telemetry.answered', { answer: app.settings.telemetry ? t('shell.telemetry.yes') : t('shell.telemetry.no') })}
      </p>
    {/if}
    <!-- Neither answer put forward: refusing looks as easy as accepting. -->
    <div class="answers">
      <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('shell.telemetry.no')}</button>
      <button class="btn" disabled={busy} onclick={() => answer(true)}>{t('shell.telemetry.yes')}</button>
    </div>
  {/if}
</div>

<style>
  .q {
    display: grid;
    gap: 18px;
    width: min(640px, 100%);
  }
  .p {
    margin: 0;
    max-width: 62ch;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
  }
  .stays,
  .lock {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--soft);
  }
  .link {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    gap: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--p-ink);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .answers {
    display: flex;
    gap: 10px;
  }
</style>

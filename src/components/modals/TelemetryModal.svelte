<script lang="ts">
  import { openUrl } from '../../lib/opener';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { privacyUrl } from '../../lib/telemetry';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  let busy = $state(false);

  /** The answer, kept at once; the question stays while it cannot be. */
  async function answer(consent: boolean) {
    busy = true;
    try {
      await api.telemetrySet(consent);
      app.settings = { ...app.settings, telemetry: consent };
      // The user's name for the team waits for the first share (ESC-190, choice O8 of docs/ONBOARDING.md).
      app.modal = null;
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  const more = () => openUrl(privacyUrl()).catch((e) => app.toast(String(e), 'error'));
</script>

<!-- Closed without an answer: asked again at the next start. -->
<Modal title={t('shell.telemetry.title')} width={520} onclose={() => (app.modal = null)}>
  {#snippet lead()}<span class="lead"><Icon name="stats" /></span>{/snippet}
  <p class="p">{t('shell.telemetry.body')}</p>
  <p class="p detail">{t('shell.telemetry.detail')}</p>
  <button class="link" onclick={more}><Icon name="ext" size={14} />{t('shell.telemetry.more')}</button>
  {#snippet footer()}
    <!-- Neither answer put forward: refusing looks as easy as accepting. -->
    <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('shell.telemetry.no')}</button>
    <button class="btn" disabled={busy} onclick={() => answer(true)}>{t('shell.telemetry.yes')}</button>
  {/snippet}
</Modal>

<style>
  .lead {
    display: inline-flex;
    color: var(--brand);
  }
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .detail {
    color: var(--muted);
    font-size: 12.5px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
    border: none;
    background: none;
    padding: 0;
    color: var(--p-ink);
    font-size: 13px;
    font-weight: 550;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
</style>

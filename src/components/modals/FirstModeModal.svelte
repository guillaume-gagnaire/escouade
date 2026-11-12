<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';

  import { FIRST_MODES, RECOMMENDED_MODE, type FirstMode } from '../../lib/first-mode';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  // The first start's first question (SEC-09): the mode of the new agents, the recommended one chosen. Closed without
  // an answer: asked again at the next start, the recommended mode in force meanwhile. Then the statistics' question.
  let chosen = $state<FirstMode>(RECOMMENDED_MODE);
  let busy = $state(false);
  const id = $props.id();

  /** What comes after it at the first start. */
  function next() {
    if (app.settings.telemetry === null) app.modal = { kind: 'telemetry' };
    else if (!app.settings.nameAsked) app.modal = { kind: 'teamName' };
    else app.modal = null;
  }

  async function keep() {
    busy = true;
    const settings = { ...$state.snapshot(app.settings), agentMode: chosen };
    const saved = await app.run(api.saveSettings(settings).then(() => true));
    busy = false;
    if (!saved) return;
    app.settings = settings;
    next();
  }
</script>

<Modal title={t('settings.permissions.firstTitle')} width={560} onclose={next}>
  {#snippet lead()}<span class="lead"><Icon name="shield" /></span>{/snippet}
  <p class="p">{t('settings.permissions.firstBody')}</p>
  <div class="modes" role="radiogroup" aria-labelledby="{id}-title" data-testid="first-mode">
    <span id="{id}-title" class="sr">{t('settings.permissions.agentMode')}</span>
    {#each FIRST_MODES as m (m)}
      <button role="radio" aria-checked={chosen === m} class="mode" class:on={chosen === m} onclick={() => (chosen = m)}>
        <span class="head"
          ><b>{t(`composer.mode.${m}.label`)}</b>{#if m === RECOMMENDED_MODE}<span class="rec">{t('settings.permissions.recommended')}</span
            >{/if}</span
        >
        <span class="desc">{t(`settings.permissions.first.${m}`)}</span>
      </button>
    {/each}
  </div>
  {#snippet footer()}
    <button class="btn primary" disabled={busy} onclick={keep}>{t('settings.permissions.choose')}</button>
  {/snippet}
</Modal>

<style>
  .lead {
    display: inline-flex;
    color: var(--muted);
  }
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .modes {
    display: grid;
    gap: 8px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
  .mode {
    display: grid;
    gap: 4px;
    text-align: left;
    padding: 10px 12px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    cursor: pointer;
  }
  .mode.on {
    box-shadow: inset 0 0 0 1.5px var(--p);
    background: color-mix(in oklch, var(--p) 8%, transparent);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .rec {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 999px;
    color: var(--p);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 45%, transparent);
  }
  .desc {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
</style>

<script lang="ts">
  import { accountDefaults, accountsOf, defaultEngine, engineOrder, hasAccount } from '../../lib/accounts';
  import { BYPASS, capsOf, engineName } from '../../lib/engines';
  import { FIRST_MODES, RECOMMENDED_MODE } from '../../lib/first-mode';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { denial, lockedBy, lockLine, MODES_KEY } from '../../lib/locks.svelte';
  import { modelLabel } from '../../lib/models';
  import { onboarding, statusOf, type Primary } from '../../lib/onboarding.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // Station 3, the starting settings (docs/ONBOARDING.md § 4.3): the mode of the new agents, each one said in a sentence,
  // the recommended one (Édits auto) selected; Bypass shown off, with why, never offered (SEC-09). The model and the
  // effort are said, not asked: the account's. « Continuer » writes « Mode par défaut des agents », as Réglages › Agents
  // does. A mode the harness lacks, or a policy denies, is greyed with why; a mode a policy imposes is shown locked.
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  const engine = $derived(onboarding.engine ?? engineOrder().find((e) => hasAccount(e)) ?? defaultEngine());
  const harness = $derived(engineName(engine));
  const defaults = $derived(accountDefaults(accountsOf(engine)[0], engine));
  const caps = $derived(capsOf(engine));
  const has = (id: string) => caps?.modes.some((m) => m.id === id) ?? true;
  /** ESC-489: a harness whose only mode is Bypass. */
  const onlyBypass = $derived(!!caps?.modes.length && caps.modes.every((m) => m.id === BYPASS));
  const imposed = $derived(statusOf('defaults') === 'policy' ? lockedBy('agentMode') : null);
  const bypassBy = $derived(lockedBy('allowBypass'));

  /** Why a mode cannot be taken: its harness lacks it, or a policy denies it. */
  function why(m: string): string | null {
    if (!has(m)) return t('composer.mode.missing', { harness, mode: t(`composer.mode.${m as (typeof FIRST_MODES)[number]}.label`) });
    return denial(MODES_KEY, m);
  }
  const items = $derived([
    ...FIRST_MODES.map((m) => ({ id: m as string, why: why(m), disabled: !!why(m) || !!imposed })),
    { id: BYPASS, why: null, disabled: true },
  ]);
  const takeable = $derived(items.filter((i) => !i.disabled).map((i) => i.id));

  let chosen = $state<string | null>(null);
  $effect(() => {
    if (chosen && (takeable.includes(chosen) || imposed)) return;
    const now = app.settings.agentMode?.trim();
    if (imposed) chosen = now || RECOMMENDED_MODE;
    else chosen = [now, RECOMMENDED_MODE, ...takeable].find((m) => !!m && takeable.includes(m)) ?? null;
  });
  const modeLabel = (m: string) => t(`composer.mode.${m as (typeof FIRST_MODES)[number]}.label`);

  async function next(): Promise<boolean> {
    if (imposed) return true;
    if (!chosen) return false;
    if (app.settings.agentMode === chosen) return true;
    const settings = { ...$state.snapshot(app.settings), agentMode: chosen };
    const saved = await app.run(api.saveSettings(settings).then(() => true));
    if (!saved) return false;
    app.settings = settings;
    return true;
  }

  $effect(() => {
    primary = {
      label: chosen && !imposed ? t('onboarding.defaults.keep', { mode: modeLabel(chosen) }) : t('onboarding.continue'),
      disabled: !chosen && !imposed,
      run: next,
    };
  });

  const effort = $derived(t(`composer.effort.${defaults.effort as 'medium'}.label`).toLowerCase());
</script>

<div class="q" data-testid="onboarding-defaults">
  <StationHead step="defaults" question={t('onboarding.defaults.question')}>
    {#snippet why()}{defaults.model
        ? t('onboarding.defaults.why', { model: modelLabel(defaults.model, app.models), effort })
        : t('onboarding.defaults.whyNoModel', { effort })}{/snippet}
  </StationHead>
  {#if imposed}<p class="note lock"><Icon name="lock" size={12} />{lockLine(imposed)}</p>{/if}
  {#if onlyBypass}<p class="note warn" role="status">{t('onboarding.defaults.onlyBypass', { harness })}</p>{/if}
  <Choices {items} bind:value={chosen} label={t('onboarding.defaults.question')}>
    {#snippet head(it)}
      {#if it.id === BYPASS}
        <span class="t"
          ><b>{t('composer.mode.bypassPermissions.label')}</b><span class="state"
            ><Icon name="lock" size={12} />{app.settings.allowBypass
              ? t('onboarding.defaults.allowed')
              : t('onboarding.defaults.off')}</span
          ></span
        >
        <span class="d">{app.settings.allowBypass ? t('onboarding.defaults.bypassAllowed') : t('onboarding.defaults.bypass')}</span>
        {#if bypassBy}<span class="d">{lockLine(bypassBy)}</span>{/if}
      {:else}
        <span class="t"
          ><b>{modeLabel(it.id)}</b>{#if it.id === RECOMMENDED_MODE}<span class="rec">{t('settings.permissions.recommended')}</span
            >{/if}</span
        >
        <span class="d">{t(`settings.permissions.first.${it.id as (typeof FIRST_MODES)[number]}`)}</span>
        {#if it.why}<span class="d why">{it.why}</span>{/if}
      {/if}
    {/snippet}
  </Choices>
</div>

<style>
  .q {
    display: grid;
    gap: 22px;
    width: min(640px, 100%);
  }
  .t {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .t b {
    color: var(--text);
    font-weight: 620;
  }
  .rec {
    padding: 1px 8px;
    border-radius: 999px;
    color: var(--teal-ink);
    font-size: 11.5px;
    font-weight: 600;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 45%, transparent);
    background: color-mix(in oklch, var(--brand) 8%, transparent);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
  }
  .d,
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .why {
    color: var(--soft);
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .note.lock {
    color: var(--soft);
  }
  .note.warn {
    color: var(--warn-ink);
  }
  @media (max-width: 1100px) {
    .q {
      gap: 16px;
    }
  }
</style>

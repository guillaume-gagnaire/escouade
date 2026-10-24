<script lang="ts">
  import { accountName, accountsOf, dedicated, defaultEngine, hasAccount, serves } from '../../lib/accounts';
  import { choosesEngine, engineName, engineOptions } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import AccountModal from '../modals/AccountModal.svelte';
  import { app } from '../../lib/state.svelte';
  import type { CommitMode, EngineId } from '../../lib/types';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Chips from '../settings/Chips.svelte';
  import Row from '../settings/Row.svelte';
  import Switch from '../settings/Switch.svelte';
  import Checks from './Checks.svelte';

  // « Compte et commit »: the harness and the account the project's agents go to, whether it is private and the accounts
  // reserved to it then, who writes the commits, Remote Control.
  let { form }: { form: WizardForm } = $props();

  const ACCOUNTS = $derived([
    { value: '', label: t('accounts.project.auto'), title: t('accounts.project.autoTitle') },
    ...accountsOf(form.config.engine ?? undefined)
      .filter((a) => a.active && serves(a, form.project?.id))
      .map((a) => ({ value: a.id, label: accountName(a) })),
  ]);
  const COMMIT_MODES = $derived<{ value: CommitMode; label: string }[]>(
    (['agent', 'direct'] as const).map((value) => ({ value, label: t(`settings.project.commitModes.${value}`) })),
  );
  const all = $derived(!!app.settings.remoteControlAll);
  /** The project's harness, and whether it has an account (ESC-128): without one, « Ajouter un compte… » over the assistant. */
  const engine = $derived(form.config.engine ?? defaultEngine());
  const missing = $derived(!hasAccount(engine));
  let adding = $state(false);

  /** Another harness: the account preferred and the accounts to reserve were the other one's. */
  function setEngine(e: EngineId) {
    if ((form.config.engine ?? 'claude') === e) return;
    form.config.engine = e;
    form.config.account = '';
    form.config.reserved = [];
  }

  /**
   * The accounts a private project may have to itself (ESC-163): those of its harness, active, that it may use (reserved to
   * no other project, or to it); those it has alone already stay so.
   */
  const reservable = $derived(accountsOf(engine).filter((a) => a.active && serves(a, form.project?.id)));
  const own = (id: string) => !!form.project && reservable.some((a) => a.id === id && dedicated(a, form.project!.id));
  function reserve(id: string, on: boolean) {
    const r = form.config.reserved.filter((x) => x !== id);
    form.config.reserved = on ? [...r, id] : r;
  }
</script>

<Checks {form} step="account" />
{#if missing}
  <div class="missing" role="status">
    <span>{t('wizard.account.noAccount', { harness: engineName(engine) })}</span>
    <button class="btn" onclick={() => (adding = true)}>{t('accounts.tab.add')}</button>
  </div>
{/if}
{#if adding}<AccountModal {engine} onclose={() => (adding = false)} />{/if}
<div class="box">
  <!-- From two harnesses found on: the one of its new agents and tickets (none chosen: Claude Code). -->
  {#if choosesEngine()}
    <Row label={t('settings.project.engine')} desc={t('settings.project.engineDesc')}>
      <Chips
        label={t('settings.project.engine')}
        options={engineOptions(form.config.engine)}
        bind:value={() => form.config.engine ?? 'claude', setEngine}
      />
    </Row>
  {/if}
  <Row label={t('accounts.project.label')} desc={t('accounts.project.desc')}>
    <Chips label={t('accounts.project.label')} options={ACCOUNTS} bind:value={form.config.account} />
  </Row>
  <Row label={t('settings.project.commit')} desc={t('settings.project.commitDesc')}>
    <Chips label={t('settings.project.commit')} options={COMMIT_MODES} bind:value={form.config.commitMode} />
  </Row>
  <Row label={t('settings.project.remote')} desc={all ? t('wizard.account.remoteAll') : t('settings.project.remoteDesc')}>
    <Switch
      label={t('settings.project.remote')}
      disabled={all}
      bind:on={() => all || form.config.remoteControl, (v) => (form.config.remoteControl = v)}
    />
  </Row>
</div>

<!-- « Projet privé » (ESC-140/141): then the accounts reserved to it alone, and whether it goes on shared ones at the limit. -->
<div class="box private">
  <Row label={t('settings.project.private')} desc={t('settings.project.privateDesc')}>
    <Switch label={t('settings.project.private')} bind:on={form.config.private} />
  </Row>
  {#if form.config.private}
    <Row label={t('wizard.account.reserved')} desc={t('wizard.account.reservedDesc', { harness: engineName(engine) })} wide>
      {#if reservable.length}
        <ul class="accounts" aria-label={t('wizard.account.reserved')}>
          {#each reservable as a (a.id)}
            <li>
              <label>
                <input
                  type="checkbox"
                  checked={own(a.id) || form.config.reserved.includes(a.id)}
                  disabled={own(a.id)}
                  onchange={(e) => reserve(a.id, e.currentTarget.checked)}
                />
                <span>{accountName(a)}</span>
                {#if own(a.id)}<span class="tag">{t('wizard.account.reservedAlready')}</span>{/if}
              </label>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="none">{t('wizard.account.reservedNone', { harness: engineName(engine) })}</p>
      {/if}
    </Row>
    <Row label={t('accounts.private.sharedOnLimit')} desc={t('accounts.private.sharedOnLimitDesc')}>
      <Switch label={t('accounts.private.sharedOnLimit')} bind:on={form.config.sharedOnLimit} />
    </Row>
  {/if}
</div>

<style>
  .missing {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--wait);
  }
  .box {
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--bg);
  }
  .private {
    margin-top: 10px;
  }
  .accounts {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .accounts label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    cursor: pointer;
  }
  input[type='checkbox'] {
    margin: 0;
    accent-color: var(--accent);
  }
  .tag {
    font-size: 11px;
    color: var(--dim);
  }
  .none {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
  }
</style>

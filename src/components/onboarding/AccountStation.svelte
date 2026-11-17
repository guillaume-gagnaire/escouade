<script lang="ts">
  import { accountName, accountsOf, engineOrder, hasAccount } from '../../lib/accounts';
  import { engineName } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { onboarding, type Primary } from '../../lib/onboarding.svelte';
  import { app } from '../../lib/state.svelte';
  import type { EngineId, MachineAccount } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // Station 2, the account (docs/ONBOARDING.md § 4.2): the machine's account of the harness chosen (its usual folder, as
  // it is), or a separate one (a folder of its own under ~/.escouade). The same gestures as « Ajouter un compte… »: the
  // machine's account added as it adds it, then signed in by its program in a terminal when it is not yet; a separate one
  // made in that same window, which closes back here. An account added already (here or in Réglages) is said, and the
  // station is done.
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  /** The harness of the account: the one chosen at the station before, else one with an account, else the first found. */
  const engine = $derived<EngineId | null>(onboarding.engine ?? engineOrder().find((e) => hasAccount(e)) ?? app.enginesFound[0] ?? null);
  const added = $derived(engine ? accountsOf(engine) : []);
  const harness = $derived(engine ? engineName(engine) : '');

  let machine = $state<MachineAccount | null>(null);
  $effect(() => {
    const e = engine;
    machine = null;
    if (!e) return;
    let stale = false;
    api
      .accountMachine(e)
      .then((m) => {
        if (!stale) machine = m;
      })
      .catch(() => {});
    return () => {
      stale = true;
    };
  });
  /** Its machine's account may be added: not apart from Escouade's reach (Gemini CLI's Google sign-in), not added yet. */
  const machineOk = $derived(!!machine && !machine.apart && !machine.added);
  const items = $derived([
    { id: 'machine', disabled: !machineOk },
    { id: 'separate', disabled: false },
  ]);
  let chosen = $state<string | null>(null);
  $effect(() => {
    if (machine && !chosen) chosen = machineOk ? 'machine' : 'separate';
  });

  async function next(): Promise<boolean> {
    if (!engine) return false;
    if (added.length) return true;
    if (chosen === 'separate') {
      // The window of « Ajouter un compte… », back here when it closes: the account it makes ticks the station.
      app.modal = { kind: 'account', engine, onboarding: true };
      return false;
    }
    const m = machine;
    const made = await app.run(api.accountAddMachine(engine));
    if (!made) return false;
    if (!app.settings.accounts.some((a) => a.id === made.id)) app.settings.accounts = [...app.settings.accounts, made];
    // Not signed in there yet: its sign-in in a terminal, in the same window as « Se connecter… ».
    if (m && !m.connected && m.signsIn) app.modal = { kind: 'account', accountId: made.id, onboarding: true };
    else void app.run(api.refreshUsage());
    return true;
  }

  $effect(() => {
    primary = { label: t('onboarding.continue'), disabled: !engine || (!added.length && !chosen), run: next };
  });
</script>

<div class="q" data-testid="onboarding-account">
  <StationHead step="account" question={engine ? t('onboarding.account.question', { harness }) : t('onboarding.stations.account.name')}>
    {#snippet why()}{t('onboarding.account.why')}{/snippet}
  </StationHead>
  {#if !engine}
    <p class="note">{t('onboarding.account.noHarness')}</p>
  {:else if added.length}
    <ul class="added">
      {#each added as a (a.id)}
        <li><Icon name="check" size={14} />{t('onboarding.account.added', { name: accountName(a) })}</li>
      {/each}
    </ul>
    <p class="note">{t('onboarding.harness.machineAccount')}</p>
  {:else}
    <Choices {items} bind:value={chosen} label={t('onboarding.account.question', { harness })}>
      {#snippet head(it)}
        {#if it.id === 'machine'}
          <span class="t"
            ><b
              >{machine?.connected && machine.email
                ? t('onboarding.account.machineEmail', { email: machine.email })
                : t('onboarding.account.machine')}</b
            >{#if machineOk && machine?.connected}<span class="rec">{t('settings.permissions.recommended')}</span>{/if}</span
          >
          <span class="d">{t('onboarding.account.machineHint')}</span>
          {#if machine && !machine.connected && machine.signsIn && machineOk}
            <span class="d warn">{t('onboarding.account.notSignedIn', { harness })}</span>
          {/if}
        {:else}
          <span class="t"><b>{t('onboarding.account.separate')}</b></span>
          <span class="d">{t('onboarding.account.separateHint')}</span>
        {/if}
      {/snippet}
    </Choices>
  {/if}
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
  .d,
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .warn {
    color: var(--warn-ink);
  }
  .added {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .added li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
  }
  .added :global(.icon) {
    color: var(--run);
  }
  @media (max-width: 1100px) {
    .q {
      gap: 16px;
    }
  }
</style>

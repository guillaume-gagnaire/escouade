<script lang="ts">
  import { accountName, agentsOn, backToAccounts, PRINCIPAL } from '../../lib/accounts';
  import { tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Account, AccountStatus } from '../../lib/types';
  import Group from './Group.svelte';

  // The Claude accounts. Each change goes to the backend at once (`account_*`), not through the modal's draft: an
  // account is made, signed in and removed on the disk, which « Annuler » could not take back.

  const accounts = $derived(app.settings.accounts ?? []);
  const activeCount = $derived(accounts.filter((a) => a.active).length);
  /** Each account's sign-in, as last read. */
  let statuses = $state<Record<string, AccountStatus>>({});
  /** Why an account was not removed, by account. */
  let refused = $state<Record<string, string>>({});
  /** The accounts whose sign-in was asked for (not reactive: asked once each). */
  const asked = new Set<string>();

  // Each account's sign-in, read when the tab opens, and for an account added since.
  $effect(() => {
    for (const a of accounts) {
      if (asked.has(a.id)) continue;
      asked.add(a.id);
      void load(a.id);
    }
  });

  async function load(id: string) {
    try {
      statuses[id] = await api.accountStatus(id);
    } catch {
      // Removed meanwhile: nothing to show.
    }
  }

  /** The account as plain data, to send. */
  const plain = (a: Account): Account => ({ ...$state.snapshot(a) });

  /** What its quota's last reading says of its sign-in when it is there but out of date. */
  function expired(id: string): string | null {
    const u = app.usage.accounts.find((x) => x.id === id);
    return u?.connected && u.reason ? u.reason : null;
  }

  function stateOf(a: Account, s: AccountStatus | undefined): string {
    if (!a.active) return t('accounts.tab.state.inactive');
    if (!s) return t('accounts.tab.state.checking');
    if (s.connected) return s.email ? t('accounts.tab.state.connected', { email: s.email }) : t('accounts.tab.state.connectedNoEmail');
    return a.id === PRINCIPAL ? t('accounts.tab.state.principalNotConnected') : t('accounts.tab.state.notConnected');
  }

  /** Saved as the backend has it: the accounts it then gives are the window's. */
  async function save(next: Account): Promise<boolean> {
    const list = await app.run(api.accountUpdate(next));
    if (list) app.settings.accounts = list;
    return !!list;
  }

  /** A field left: what it says saved when it changed, else (or refused) as it was. */
  async function commit(a: Account, input: HTMLInputElement, field: 'name' | 'claudePath') {
    const shown = field === 'name' ? accountName(a) : a.claudePath;
    if (input.value === shown) return;
    if (!(await save({ ...plain(a), [field]: input.value }))) input.value = shown;
  }

  async function move(i: number, by: number) {
    const ids = accounts.map((a) => a.id);
    [ids[i], ids[i + by]] = [ids[i + by], ids[i]];
    const list = await app.run(api.accountReorder(ids));
    if (list) app.settings.accounts = list;
  }

  /** Asks before removing it, unless agents still run on it; back to this tab either way. */
  function remove(a: Account) {
    const n = agentsOn(a.id);
    if (n) {
      refused[a.id] = t('accounts.tab.stillUsed', { count: n });
      return;
    }
    delete refused[a.id];
    const { id, configDir } = a;
    app.modal = {
      kind: 'confirm',
      title: t('accounts.tab.removeConfirm.title', { name: accountName(a) }),
      body: t('accounts.tab.removeConfirm.body', { path: tildify(configDir) }),
      confirm: t('accounts.tab.removeConfirm.confirm'),
      danger: true,
      onConfirm: async () => {
        const list = await app.run(api.accountRemove(id));
        if (list) app.settings.accounts = list;
        backToAccounts();
      },
      onCancel: backToAccounts,
    };
  }

  const blurOnEnter = (e: KeyboardEvent) => {
    if (e.key === 'Enter') (e.currentTarget as HTMLInputElement).blur();
  };
</script>

<Group title={t('accounts.tab.list')} note={t('accounts.tab.order')} plain>
  <ul class="list" aria-label={t('accounts.tab.list')}>
    {#each accounts as a, i (a.id)}
      {@const name = accountName(a)}
      {@const status = statuses[a.id]}
      {@const late = expired(a.id)}
      {@const last = a.active && activeCount === 1}
      <li class="acct" aria-label={name}>
        <div class="line">
          <input
            class="field name"
            aria-label={t('accounts.tab.name')}
            spellcheck="false"
            value={name}
            onkeydown={blurOnEnter}
            onblur={(e) => commit(a, e.currentTarget, 'name')}
          />
          <span class="state" class:on={a.active && status?.connected} class:off={!a.active}>
            <span class="dot" aria-hidden="true"></span>{stateOf(a, status)}
          </span>
          <div style="flex:1"></div>
          <span class="k" aria-hidden="true">{t('accounts.tab.active')}</span>
          <button
            class="switch"
            class:on={a.active}
            role="switch"
            aria-checked={a.active}
            aria-label={t('accounts.tab.active')}
            aria-describedby={last ? 'accounts-last-active' : undefined}
            title={last ? t('accounts.tab.lastActive') : undefined}
            disabled={last}
            onclick={() => save({ ...plain(a), active: !a.active })}
          ></button>
          <button class="icon-btn" aria-label={t('accounts.tab.up', { name })} disabled={i === 0} onclick={() => move(i, -1)}>↑</button>
          <button
            class="icon-btn"
            aria-label={t('accounts.tab.down', { name })}
            disabled={i === accounts.length - 1}
            onclick={() => move(i, 1)}>↓</button
          >
        </div>
        <div class="line">
          <span class="k">{t('accounts.tab.folder')}</span>
          <span class="dir mono">{tildify(a.configDir || status?.dir || '')}</span>
        </div>
        <div class="line">
          <label class="k" for="account-exe-{a.id}">{t('accounts.tab.executable')}</label>
          <input
            id="account-exe-{a.id}"
            class="field mono exe"
            spellcheck="false"
            placeholder={t('accounts.tab.executableDefault')}
            value={a.claudePath}
            onkeydown={blurOnEnter}
            onblur={(e) => commit(a, e.currentTarget, 'claudePath')}
          />
          <div style="flex:1"></div>
          {#if status}
            <!-- Not signed in, or out of date: to sign in. Signed in: to sign in again (another claude.ai account, say). -->
            <button class="btn" onclick={() => (app.modal = { kind: 'account', accountId: a.id })}>
              {status.connected && !late ? t('accounts.tab.signInAgain') : t('accounts.tab.signIn')}
            </button>
          {/if}
          {#if a.id !== PRINCIPAL}
            <button class="btn ghost danger" onclick={() => remove(a)}>{t('accounts.tab.remove')}</button>
          {/if}
        </div>
        {#if late}<p class="late">{late}</p>{/if}
        {#if refused[a.id]}<p class="refused" role="alert">{refused[a.id]}</p>{/if}
      </li>
    {/each}
  </ul>
  <span id="accounts-last-active" hidden>{t('accounts.tab.lastActive')}</span>
</Group>
<div>
  <button class="btn" onclick={() => (app.modal = { kind: 'account' })}>{t('accounts.tab.add')}</button>
</div>

<style>
  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--bg);
  }
  .acct {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 16px;
  }
  .acct:not(:first-child) {
    border-top: 1px solid var(--line);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .name {
    width: 200px;
    height: 30px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 13px;
    font-weight: 600;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
  }
  .dot {
    width: 7px;
    height: 7px;
    flex: none;
    border-radius: 50%;
    background: var(--wait);
  }
  .state.on .dot {
    background: var(--add);
  }
  .state.off .dot {
    background: var(--dim);
  }
  .k {
    flex: none;
    font-size: 12px;
    color: var(--dim);
  }
  .dir {
    min-width: 0;
    font-size: 11.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exe {
    width: 300px;
    max-width: 45%;
    height: 28px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .btn {
    height: 28px;
    font-size: 12px;
  }
  .icon-btn:disabled {
    opacity: 0.35;
    background: transparent;
    cursor: default;
  }
  /* As `Switch` has it: the last account active stays on. */
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .late,
  .refused {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.45;
  }
  .late {
    color: var(--wait);
  }
  .refused {
    color: var(--del);
  }
</style>

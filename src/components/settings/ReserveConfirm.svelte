<script lang="ts">
  import { accountName, engineOfAccount, onlyActive } from '../../lib/accounts';
  import { engineName } from '../../lib/engines';
  import { fList } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { Account, Project } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';

  // ESC-556: a reservation that would leave open projects without an active account of its harness is asked first, wherever
  // one reserves (a private project's Confidentialité, Réglages › Comptes et harness, the assistant « Nouveau projet »), and
  // says what follows. Over the page it is opened from, which keeps what it holds (a draft, the assistant's steps).
  let {
    account,
    stranded,
    target,
    onConfirm,
    onAdd,
    onclose,
  }: {
    account: Account;
    /** The projects that would have no account of its harness left (`strandedBy`). */
    stranded: Pick<Project, 'id' | 'name'>[];
    /** The name of the project it is reserved to, when it is to one alone: « Ajouter un compte réservé… » is offered for it. */
    target?: string;
    onConfirm: () => void | Promise<void>;
    /** « Ajouter un compte réservé… »: the way that takes nothing from the others. */
    onAdd?: () => void;
    onclose: () => void;
  } = $props();

  const harness = $derived(engineName(engineOfAccount(account)));
  const name = $derived(accountName(account));
  const names = $derived(fList(stranded.map((p) => t('accounts.tab.quoted', { name: p.name }))));
  const count = $derived(stranded.length);
  const body = $derived(
    onlyActive(account)
      ? t('accounts.reserve.onlyOne', { count, names, harness })
      : t('accounts.reserve.lastOne', { count, names, harness, name }),
  );
  let busy = $state(false);

  async function go() {
    busy = true;
    await onConfirm();
    busy = false;
    onclose();
  }
  function add() {
    onAdd?.();
    onclose();
  }
</script>

<Modal
  title={target ? t('accounts.reserve.title', { name, project: target }) : t('accounts.reserve.titleChange', { name })}
  width={520}
  {onclose}
>
  {#snippet lead()}<span class="lead"><Icon name="user" size={16} /></span>{/snippet}
  <p class="p" data-testid="reserve-confirm">{body}</p>
  <p class="p soft">{t('accounts.reserve.undo')}</p>
  {#if onAdd && target}<p class="p soft">{t('accounts.reserve.instead', { project: target })}</p>{/if}
  {#snippet footer()}
    <button class="btn" onclick={onclose}>{t('common.cancel')}</button>
    {#if onAdd}<button class="btn" disabled={busy} onclick={add}>{t('accounts.private.add')}</button>{/if}
    <button class="btn danger" disabled={busy} onclick={go}>{t('accounts.reserve.confirm')}</button>
  {/snippet}
</Modal>

<style>
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .soft {
    color: var(--muted);
    font-size: 12.5px;
  }
  .lead {
    display: inline-flex;
    color: var(--wait);
  }
</style>

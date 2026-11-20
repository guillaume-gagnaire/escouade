<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import type { IconName } from '../../lib/icons';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  let {
    title,
    body,
    confirm,
    danger = false,
    option,
    alt,
    icon,
    onConfirm,
    onCancel,
  }: {
    title: string;
    body: string;
    confirm: string;
    danger?: boolean;
    option?: { label: string; value: boolean };
    alt?: { label: string; onClick: () => void | Promise<void> };
    /** Before the title, in amber: what is about to happen (▶ of a command that already runs elsewhere). */
    icon?: IconName;
    onConfirm: (option: boolean) => void | Promise<void>;
    onCancel?: () => void;
  } = $props();

  let checked = $state(untrack(() => option?.value ?? false));
  let busy = $state(false);

  /** Runs a choice, then closes, unless the choice opened another modal (the next question). */
  async function choose(run: () => void | Promise<void>) {
    const self = app.modal;
    busy = true;
    // Replaced while the choice runs (« Quitter Escouade ? »), it is not asked again.
    app.markAnswered(self);
    await run();
    busy = false;
    if (app.modal === self) app.modal = null;
  }
  const go = () => choose(() => onConfirm(checked));

  function cancel() {
    // Read before closing: the props are the modal's, gone once it is closed.
    const back = onCancel;
    app.modal = null;
    back?.();
  }
</script>

<Modal {title} width={icon ? 540 : 480} onclose={cancel}>
  {#snippet lead()}{#if icon}<span class="lead"><Icon name={icon} size={16} /></span>{/if}{/snippet}
  <p class="p">{body}</p>
  {#if option}
    <label class="opt">
      <input type="checkbox" bind:checked />
      {option.label}
    </label>
  {/if}
  {#snippet footer()}
    <button class="btn" onclick={cancel}>{t('common.cancel')}</button>
    {#if alt}
      <button class="btn" disabled={busy} onclick={() => choose(alt.onClick)}>{alt.label}</button>
    {/if}
    <button class="btn {danger ? 'danger' : 'primary'}" disabled={busy} onclick={go}>{confirm}</button>
  {/snippet}
</Modal>

<style>
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
    /* A list in the text (the requests of the box « À toi » before the server is turned off, ESC-510), line by line. */
    white-space: pre-line;
  }
  .lead {
    display: inline-flex;
    color: var(--wait);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    cursor: pointer;
  }
</style>

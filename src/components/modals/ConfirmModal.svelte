<script lang="ts">
  import { untrack } from 'svelte';
  import { app } from '../../lib/state.svelte';
  import Modal from './Modal.svelte';

  let {
    title,
    body,
    confirm,
    danger = false,
    option,
    alt,
    onConfirm,
    onCancel,
  }: {
    title: string;
    body: string;
    confirm: string;
    danger?: boolean;
    option?: { label: string; value: boolean };
    alt?: { label: string; onClick: () => void | Promise<void> };
    onConfirm: (option: boolean) => void | Promise<void>;
    onCancel?: () => void;
  } = $props();

  let checked = $state(untrack(() => option?.value ?? false));
  let busy = $state(false);

  /** Runs a choice, then closes, unless the choice opened another modal (the next question). */
  async function choose(run: () => void | Promise<void>) {
    const self = app.modal;
    busy = true;
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

<Modal {title} width={480} onclose={cancel}>
  <p class="p">{body}</p>
  {#if option}
    <label class="opt">
      <input type="checkbox" bind:checked />
      {option.label}
    </label>
  {/if}
  {#snippet footer()}
    <button class="btn ghost" onclick={cancel}>Annuler</button>
    {#if alt}
      <button class="btn" disabled={busy} onclick={() => choose(alt.onClick)}>{alt.label}</button>
    {/if}
    <button class="btn {danger ? 'danger' : 'primary'}" disabled={busy} onclick={go}>{confirm}</button>
  {/snippet}
</Modal>

<style>
  .p {
    margin: 0;
    color: var(--muted);
    line-height: 1.55;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    cursor: pointer;
  }
</style>

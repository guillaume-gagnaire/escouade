<script lang="ts">
  import { t } from '../lib/i18n';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  // An image attached to a field not sent yet (the message's, a ticket's): its thumbnail, name and what is said under it,
  // and the « × » that takes it away. The thumbnail and its name are one button that shows it full size (ESC-555), by a
  // click, Entrée or Espace; before its picture is read (`url` empty), a grey square that opens nothing.
  let {
    name,
    url,
    label = name,
    detail,
    removeLabel,
    onopen,
    onremove,
  }: {
    name: string;
    url?: string;
    /** Its name in the tip and for a screen reader (a ticket's ranks it among several). */
    label?: string;
    detail: string;
    removeLabel: string;
    onopen: () => void;
    onremove: () => void;
  } = $props();
</script>

<span class="att img">
  {#if url}
    <button
      type="button"
      class="open"
      aria-label={t('composer.attachments.view', { name: label })}
      use:tip={{ text: t('composer.attachments.view', { name: label }), cut: true }}
      onclick={onopen}
    >
      <img class="th" src={url} alt={label} />
      <span class="nm2"><b>{name}</b><span>{detail}</span></span>
    </button>
  {:else}
    <span class="open" use:tip={{ text: label, cut: true }}>
      <span class="th wait" role="img" aria-label={t('board.form.images.loading', { name: label })}></span>
      <span class="nm2"><b>{name}</b><span>{detail}</span></span>
    </span>
  {/if}
  <!-- `rm`: the form that holds it moves the focus from one to the next (TicketForm). -->
  <button type="button" class="x rm" aria-label={removeLabel} onclick={onremove}><Icon name="close" size={14} /></button>
</span>

<style>
  .att {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    height: 40px;
    padding: 0 6px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
  }
  .open {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 100%;
    /* The whole left of the chip is the button's, to its edge: the chip's own padding taken back. */
    margin-left: -6px;
    padding: 0 0 0 6px;
    border: none;
    border-radius: 9px 0 0 9px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
  }
  button.open {
    cursor: zoom-in;
  }
  button.open:hover .th {
    box-shadow: 0 0 0 2px var(--line-2);
  }
  button.open:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .th {
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    object-fit: cover;
  }
  .th.wait {
    background: rgba(var(--fg-rgb), 0.06);
  }
  .nm2 {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .nm2 b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .nm2 span {
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .x {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
</style>

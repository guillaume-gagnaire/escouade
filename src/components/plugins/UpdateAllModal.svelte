<script lang="ts">
  import { t } from '../../lib/i18n';
  import { plugins, type InstallStep } from '../../lib/plugins-view.svelte';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';
  import PluginTile from './PluginTile.svelte';
  import Seal from './Seal.svelte';

  // « Tout mettre à jour » (ESC-436): the updates that ask for nothing more than the version installed (no permission, no
  // contribution), agreed to together in this one window: each its name, its versions and its seal. Those that ask for
  // more come after, each in its own window. Nothing is installed before « Tout mettre à jour ».
  let { step }: { step: Extract<InstallStep, { kind: 'batch' }> } = $props();

  const count = $derived(step.previews.length);
  const after = $derived(plugins.ahead);
</script>

<Modal title={t('plugins.install.batchTitle', { count })} width={560} onclose={() => plugins.cancel()}>
  {#snippet lead()}<Icon name="up" />{/snippet}
  <div class="body">
    <p class="same">{t('plugins.install.batchBody')}</p>
    <ul class="list" aria-label={t('plugins.updates.title')}>
      {#each step.previews as p (p.id)}
        {@const row = plugins.rows[p.id]}
        <li>
          <PluginTile id={p.id} name={p.name} src={p.icon} size={32} />
          <div class="who">
            <b>{p.name}</b>
            <span class="mono">{t('plugins.updates.row', { from: p.previous?.version ?? '', to: p.version })}</span>
          </div>
          {#if row?.kind === 'installing'}
            <span class="st" role="status"><i class="spin" aria-hidden="true"></i>{t('plugins.updates.installing')}</span>
          {:else if row?.kind === 'done'}
            <span class="st ok" role="status">{t('plugins.updates.done', { version: row.version })}</span>
          {:else if row?.kind === 'error'}
            <span class="st err" role="alert">{row.message}</span>
          {:else if p.verified}
            <Seal version={p.version} long />
          {:else}
            <span class="ptag">{t('plugins.tag.unverified')}</span>
          {/if}
        </li>
      {/each}
    </ul>
    {#if after.length}
      <p class="next"><Icon name="info" size={14} /><span>{t('plugins.install.batchNext', { count: after.length })}</span></p>
    {/if}
  </div>
  {#snippet footer()}
    <button class="btn" disabled={step.busy} onclick={() => plugins.cancel()}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={step.busy} aria-busy={step.busy} onclick={() => plugins.confirmBatch()}
      >{t('plugins.install.batchConfirm', { count })}</button
    >
  {/snippet}
</Modal>

<style>
  .body {
    display: grid;
    gap: 12px;
  }
  .same {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .list li {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 9px 12px;
  }
  .list li + li {
    border-top: 1px solid var(--line);
  }
  .who {
    min-width: 0;
  }
  .who b {
    display: block;
    font-weight: 600;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mono {
    color: var(--warn-ink);
    font-size: 11.5px;
  }
  .ptag {
    font-size: 11px;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .st {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--muted);
    max-width: 220px;
  }
  .st.ok {
    color: var(--teal-ink);
  }
  .st.err {
    color: var(--del);
  }
  .next {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 9px 11px;
    border-radius: 9px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
  }
  .next :global(.icon) {
    margin-top: 2px;
    color: var(--warn-ink);
  }
  .spin {
    width: 10px;
    height: 10px;
    border: 1.6px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>

<script lang="ts">
  import { flushDrafts } from '../../lib/drafts';
  import { buffers } from '../../lib/editor/buffers.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import Modal from './Modal.svelte';

  // The update downloaded, which a restart installs; or, `installed`, the one installed since the last start: its notes.
  let { version, notes, installed = false }: { version: string; notes: string; installed?: boolean } = $props();

  // The files « Quitter » asks about, which a restart would lose: saved first.
  const unsaved = $derived(buffers.unsaved);
  // A restart stops every agent's process: a turn under way, or a question waiting, is cut.
  const working = $derived(
    Object.values(app.agents).filter((a) => !a.archived && (a.status === 'running' || a.status === 'waiting')).length,
  );
  let restarting = $state(false);

  async function restart() {
    restarting = true;
    // What is being typed in the composers is written before the app stops.
    flushDrafts();
    // Once it is in, the app goes: nothing comes back but a failure.
    await app.run(api.updateRestart());
    restarting = false;
  }

  const close = () => (app.modal = null);
</script>

<Modal title={installed ? t('shell.update.notes', { version }) : t('shell.update.ready', { version })} width={580} onclose={close}>
  {#if notes.trim()}
    <div class="notes"><Markdown text={notes} /></div>
  {/if}
  {#if !installed && (unsaved || working)}
    <div class="state">
      {#if unsaved}
        <p class="warn">{t('shell.update.saveFirst', { count: unsaved })}</p>
      {/if}
      {#if working}
        <p>{t('shell.update.working', { count: working })}</p>
      {/if}
    </div>
  {/if}
  {#snippet footer()}
    {#if installed}
      <button class="btn primary" onclick={close}>{t('common.close')}</button>
    {:else}
      <button class="btn ghost" onclick={close}>{t('common.later')}</button>
      <button class="btn primary" disabled={!!unsaved || restarting} onclick={restart}
        >{restarting ? t('shell.update.restarting') : t('shell.update.restartNow')}</button
      >
    {/if}
  {/snippet}
</Modal>

<style>
  .notes {
    font-size: 13px;
  }
  .state {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .state p {
    margin: 0;
    color: var(--muted);
    line-height: 1.55;
  }
  .state .warn {
    color: var(--wait);
  }
</style>

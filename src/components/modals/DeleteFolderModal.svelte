<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
  import { returnHeld } from '../../lib/outbox.svelte';
  import { app } from '../../lib/state.svelte';
  import { stopTests } from '../../lib/test-launch.svelte';
  import Modal from './Modal.svelte';

  let { projectId, folderId }: { projectId: string; folderId: string } = $props();

  type Choice = 'out' | 'archive' | 'remove';
  let choice = $state<Choice>('out');
  let removeWorktree = $state(true);
  let busy = $state(false);

  const folder = app.projects.find((p) => p.id === projectId)?.agentFolders?.find((f) => f.id === folderId);
  // Read once, as the modal opens; the archived ones are never touched.
  const agents = (folder?.agents ?? []).map((id) => app.agents[id]).filter((a) => a && !a.archived);
  const count = agents.length;
  const tickets = agents
    .map((a) => app.ticketOf(a.id))
    .filter((k) => k?.column === 'doing' || k?.column === 'review')
    .map((k) => k!.key);
  const hasWorktree = agents.some((a) => a.worktree);
  const unsaved = agents.reduce((n, a) => n + buffers.unsavedIn(projectId, a.id), 0);

  // A folder gone in the meantime: nothing to ask.
  if (!folder || !count) queueMicrotask(close);

  function close() {
    app.modal = null;
  }

  async function go() {
    busy = true;
    const self = app.modal;
    for (const a of agents) {
      if (choice === 'archive') {
        stopTests(a.id);
        returnHeld(a.id);
        await app.run(api.archiveAgent(a.id, true));
      } else if (choice === 'remove') {
        stopTests(a.id);
        const warning = await app.run(api.deleteAgent(a.id, removeWorktree && !!a.worktree));
        if (warning) app.toast(warning, 'error');
      }
    }
    await app.run(api.agentFolderDelete(projectId, folderId));
    busy = false;
    if (app.modal === self) close();
  }
</script>

<Modal title={t('nav.sidebar.folders.deleteModal.title', { name: folder?.name ?? '' })} width={480} onclose={close}>
  <div class="choices">
    <label class="opt">
      <input type="radio" name="choice" value="out" bind:group={choice} />
      {t('nav.sidebar.folders.deleteModal.out')}
    </label>
    <label class="opt">
      <input type="radio" name="choice" value="archive" bind:group={choice} />
      {t('nav.sidebar.folders.deleteModal.archive', { count })}
    </label>
    {#if choice === 'archive' && tickets.length}
      <p class="note">{t('nav.sidebar.archiveBody', { key: tickets.join(', ') })}</p>
    {/if}
    <label class="opt">
      <input type="radio" name="choice" value="remove" bind:group={choice} />
      {t('nav.sidebar.folders.deleteModal.remove', { count })}
    </label>
    {#if choice === 'remove'}
      {#if unsaved}<p class="note">{lossNotice(unsaved).trim()}</p>{/if}
      {#if hasWorktree}
        <label class="opt sub">
          <input type="checkbox" bind:checked={removeWorktree} />
          {t('nav.sidebar.folders.deleteModal.worktrees')}
        </label>
      {/if}
    {/if}
  </div>
  {#snippet footer()}
    <button class="btn" onclick={close}>{t('common.cancel')}</button>
    <button class="btn {choice === 'remove' ? 'danger' : 'primary'}" disabled={busy} onclick={go}>
      {t('nav.sidebar.folders.deleteModal.confirm')}
    </button>
  {/snippet}
</Modal>

<style>
  .choices {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    cursor: pointer;
  }
  .sub {
    margin-left: 22px;
  }
  .note {
    margin: 0 0 0 22px;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
</style>

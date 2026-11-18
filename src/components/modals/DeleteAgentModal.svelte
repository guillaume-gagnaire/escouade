<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
  import { engineName, speakerOf } from '../../lib/engines';
  import { app } from '../../lib/state.svelte';
  import { stopTests } from '../../lib/test-launch.svelte';
  import type { DeleteLoss } from '../../lib/types';
  import Modal from './Modal.svelte';

  /**
   * « Supprimer l'agent » (ESC-511): asked by the user, or by the agent that made it (`delete_agent`, under its name). It
   * says what would be lost: its uncommitted files and its commits not merged (read by git as it opens, « inconnu » if git
   * is late), a turn under way or a request waiting, its ticket. The worktree's box is unticked while there is work in
   * progress, ticked otherwise (as the asking agent wants it, then). The focus is the dialog's: Entrée confirms nothing.
   */
  let { agentId }: { agentId: string } = $props();

  // Read once, as it opens.
  const a = untrack(() => app.agents[agentId]);
  const ask = a ? app.deleteAskOf(a) : null;
  const ticket = a ? app.ticketOf(a.id) : undefined;
  const unsaved = a ? buffers.unsavedIn(a.projectId, a.id) : 0;
  const turn = a?.status === 'running';
  const request = a?.status === 'waiting' || !!a?.requests?.length;

  let loss = $state<DeleteLoss | null>(null);
  let checked = $state(false);
  /** Changed by the user: what git says later does not change it back. */
  let touched = false;
  let busy = $state(false);

  // Gone in the meantime: nothing to ask.
  if (!a) queueMicrotask(close);
  else if (a.worktree) {
    api
      .deleteLoss(a.id)
      .catch(() => ({ known: false, dirty: 0, names: [], unmerged: 0, base: '' }) satisfies DeleteLoss)
      .then((l) => {
        loss = l;
        // Ticked only when nothing would be lost; unknown counts as something.
        if (!touched) checked = l.known && !l.dirty && !l.unmerged && (ask?.worktree ?? true);
      });
  }

  const work = $derived(!!loss && (!loss.known || loss.dirty > 0 || loss.unmerged > 0));

  function close() {
    app.modal = null;
  }

  function refuse() {
    if (a) app.refuseDelete(a.id);
    close();
  }

  async function go() {
    if (!a) return;
    const self = app.modal;
    busy = true;
    app.markAnswered(self);
    stopTests(a.id);
    const warning = await app.run(api.deleteAgent(a.id, checked && !!a.worktree));
    if (warning) app.toast(warning, 'error');
    busy = false;
    if (app.modal === self) close();
  }

  const dirtyNames = (l: DeleteLoss) =>
    l.names.join(', ') + (l.dirty > l.names.length ? t('nav.sidebar.deleteLoss.more', { count: l.dirty - l.names.length }) : '');
</script>

{#if a}
  <Modal title={t('nav.sidebar.deleteTitle', { name: a.name })} width={500} frameFocus onclose={close}>
    {#if ask}<p class="by" data-delete-by>{t('nav.sidebar.deleteAsk.by', { by: ask.name })}</p>{/if}
    <p class="p">
      {t('nav.sidebar.deleteBody', { engine: speakerOf(a.engine), harness: engineName(a.engine) })}{lossNotice(unsaved)}
    </p>
    <section class="loss" aria-label={t('nav.sidebar.deleteLoss.heading')} data-delete-loss>
      <h3>{t('nav.sidebar.deleteLoss.heading')}</h3>
      <ul>
        {#if a.worktree}
          {#if !loss}
            <li class="soft" data-loss="reading">{t('nav.sidebar.deleteLoss.reading')}</li>
          {:else if !loss.known}
            <li class="warn" data-loss="unknown">{t('nav.sidebar.deleteLoss.unknown')}</li>
          {:else}
            {#if loss.dirty}
              <li class="warn" data-loss="dirty">
                {t('nav.sidebar.deleteLoss.dirty', { count: loss.dirty, names: dirtyNames(loss) })}
              </li>
            {/if}
            {#if loss.unmerged}
              <li class="warn" data-loss="unmerged">
                {t('nav.sidebar.deleteLoss.unmerged', { count: loss.unmerged, base: loss.base })}
              </li>
            {/if}
          {/if}
        {/if}
        {#if turn}<li class="warn" data-loss="turn">{t('nav.sidebar.deleteLoss.turn')}</li>{/if}
        {#if request}<li class="warn" data-loss="request">{t('nav.sidebar.deleteLoss.request')}</li>{/if}
        {#if ticket}<li data-loss="ticket">{t('nav.sidebar.deleteLoss.ticketLinked', { key: ticket.key })}</li>{/if}
        {#if (!a.worktree || (loss?.known && !loss.dirty && !loss.unmerged)) && !turn && !request && !ticket}
          <li class="soft" data-loss="nothing">{t('nav.sidebar.deleteLoss.nothing')}</li>
        {/if}
      </ul>
    </section>
    {#if a.worktree}
      <label class="opt">
        <input type="checkbox" bind:checked disabled={!loss} onchange={() => (touched = true)} data-delete-worktree />
        {a.worktree.existing
          ? t('branches.agent.deleteWorktree', { branch: a.worktree.branch })
          : t('nav.sidebar.deleteWorktree', { branch: a.worktree.branch })}
      </label>
      {#if work && !checked}<p class="note">{t('nav.sidebar.deleteLoss.kept')}</p>{/if}
    {/if}
    {#snippet footer()}
      <button class="btn" onclick={close}>{t('common.cancel')}</button>
      {#if ask}
        <button class="btn" disabled={busy} onclick={refuse}>{t('nav.sidebar.deleteAsk.refuse')}</button>
      {/if}
      <button class="btn danger" disabled={busy} onclick={go}>{t('common.delete')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .by {
    margin: 0;
    color: var(--wait);
    font-weight: 550;
    line-height: 1.5;
  }
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .loss h3 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .loss ul {
    margin: 0;
    padding: 0 0 0 18px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
    line-height: 1.5;
    overflow-wrap: anywhere;
  }
  .warn {
    color: var(--text);
  }
  .warn::marker {
    color: var(--wait);
  }
  .soft {
    color: var(--soft);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    cursor: pointer;
  }
  .note {
    margin: -4px 0 0 26px;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
</style>

<script lang="ts">
  // « Choisis ton harness » (TS-17): the agent passed ran on a harness this machine does not have for the project; it
  // goes on on one of this machine's, with the handover file. Never an account of the teammate.
  import { engineName } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import type { EngineId } from '../../lib/types';
  import EngineMark from '../EngineMark.svelte';
  import Modal from '../modals/Modal.svelte';

  let {
    ticketId,
    engines,
    onPick,
    onCancel,
  }: { ticketId: string; engines: EngineId[]; onPick: (e: EngineId) => void; onCancel?: () => void } = $props();

  const handoff = $derived(app.tickets[ticketId]?.team?.handoff ?? null);
  const from = $derived(handoff ? engineName(handoff.engine as EngineId) : '');

  function pick(e: EngineId) {
    app.modal = null;
    onPick(e);
  }
  function close() {
    app.modal = null;
    onCancel?.();
  }
</script>

<Modal title={t('teamBoard.handoff.pick')} width={460} onclose={close}>
  <div class="take" data-testid="team-handoff-take">
    <p class="lead">{t('teamBoard.handoff.pickHint', { harness: from })}</p>
    <div class="list">
      {#each engines as e (e)}
        <button class="btn engine" data-engine={e} onclick={() => pick(e)}><EngineMark engine={e} /><span>{engineName(e)}</span></button>
      {/each}
    </div>
  </div>
  {#snippet footer()}
    <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
  {/snippet}
</Modal>

<style>
  .take {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .lead {
    margin: 0;
    color: var(--soft);
    line-height: 1.5;
    font-size: 13px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .engine {
    justify-content: flex-start;
    gap: 9px;
  }
</style>

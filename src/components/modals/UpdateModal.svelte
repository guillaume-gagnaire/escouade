<script lang="ts">
  import { buffers } from '../../lib/editor/buffers.svelte';
  import { plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import Modal from './Modal.svelte';

  // The update downloaded, which a restart installs; or, `installed`, the one installed since the last start: its notes.
  let { version, notes, installed = false }: { version: string; notes: string; installed?: boolean } = $props();

  // The files « Quitter » asks about, which a restart would lose: saved first.
  const unsaved = $derived(buffers.unsaved);
  const working = $derived(Object.values(app.agents).filter((a) => !a.archived && a.status === 'running').length);
  let restarting = $state(false);

  async function restart() {
    restarting = true;
    // Once it is in, the app goes: nothing comes back but a failure.
    await app.run(api.updateRestart());
    restarting = false;
  }

  const close = () => (app.modal = null);
</script>

<Modal title={installed ? `Nouveautés d’Escouade ${version}` : `Escouade ${version} est prête`} width={580} onclose={close}>
  {#if notes.trim()}
    <div class="notes"><Markdown text={notes} /></div>
  {/if}
  {#if !installed && (unsaved || working)}
    <div class="state">
      {#if unsaved}
        <p class="warn">
          Enregistre d’abord tes fichiers : {plural(unsaved, 'fichier n’est pas enregistré', 'fichiers ne sont pas enregistrés')}.
        </p>
      {/if}
      {#if working}
        <p>
          {working > 1
            ? `${working} agents travaillent : ils reprendront après le redémarrage.`
            : '1 agent travaille : il reprendra après le redémarrage.'}
        </p>
      {/if}
    </div>
  {/if}
  {#snippet footer()}
    {#if installed}
      <button class="btn primary" onclick={close}>Fermer</button>
    {:else}
      <button class="btn ghost" onclick={close}>Plus tard</button>
      <button class="btn primary" disabled={!!unsaved || restarting} onclick={restart}
        >{restarting ? 'Redémarrage…' : 'Redémarrer maintenant'}</button
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

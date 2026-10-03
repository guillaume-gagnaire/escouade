<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../../lib/ipc';
  import { forgetLaunches } from '../../lib/launch-actions';
  import { app } from '../../lib/state.svelte';
  import type { RunCommand } from '../../lib/types';
  import Modal from './Modal.svelte';

  let { projectId }: { projectId: string } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  let cmds = $state<RunCommand[]>(untrack(() => $state.snapshot(project?.runCommands ?? [])));
  let copy = $state(untrack(() => (project?.worktreeCopy ?? ['.env*']).join('\n')));
  let busy = $state(false);

  const valid = $derived(cmds.every((c) => c.name.trim() && c.command.trim()));

  function add() {
    cmds.push({ id: crypto.randomUUID(), name: '', command: '', shell: app.shells[0]?.id ?? 'pwsh', cwd: '' });
  }

  const close = () => (app.modal = null);

  async function save() {
    if (!project || !valid) return;
    busy = true;
    const runCommands = cmds.map((c) => ({ ...c, name: c.name.trim(), command: c.command.trim(), cwd: c.cwd.trim() }));
    const worktreeCopy = copy
      .split('\n')
      .map((l) => l.trim())
      .filter(Boolean);
    const p = { ...$state.snapshot(project), runCommands, worktreeCopy };
    const saved = await app.run(api.updateProject(p).then(() => true));
    busy = false;
    if (!saved) return;
    forgetLaunches(project.runCommands.filter((c) => !runCommands.some((x) => x.id === c.id)).map((c) => c.id));
    const i = app.projects.findIndex((x) => x.id === projectId);
    if (i >= 0) app.projects[i] = p;
    close();
  }
</script>

<Modal title="Commandes de lancement{project ? ` · ${project.name}` : ''}" width={640} onclose={close}>
  <p class="note">
    Chaque commande tourne dans son propre terminal, en lecture seule. Lance-les depuis la section « Lancement » de la barre latérale.
  </p>
  {#each cmds as c, i (c.id)}
    <fieldset class="cmd">
      <legend class="sr">Commande {i + 1}</legend>
      <div class="row">
        <label class="f grow"><span>Nom</span><input class="field" bind:value={c.name} placeholder="ex. Front" /></label>
        <label class="f"
          ><span>Shell</span>
          <select class="field" bind:value={c.shell}>
            {#each app.shells as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
            {#if !app.shells.some((s) => s.id === c.shell)}<option value={c.shell}>{c.shell} (introuvable)</option>{/if}
          </select>
        </label>
        <button class="del" title="Supprimer" aria-label="Supprimer" onclick={() => cmds.splice(i, 1)}>×</button>
      </div>
      <label class="f"><span>Commande</span><input class="field mono" bind:value={c.command} placeholder="ex. npm run dev" /></label>
      <label class="f"
        ><span>Sous-dossier <em>(vide = dossier du projet)</em></span><input
          class="field mono"
          bind:value={c.cwd}
          placeholder="ex. apps/web"
        /></label
      >
    </fieldset>
  {:else}
    <div class="empty">Aucune commande pour l'instant.</div>
  {/each}
  <button class="btn add" onclick={add}>+ Ajouter une commande</button>
  <label class="f">
    <span>Fichiers copiés dans les worktrees <em>(non suivis par git ; un motif par ligne : .env* à la racine, **/.env* partout)</em></span>
    <textarea class="field mono" rows="3" placeholder=".env*" bind:value={copy}></textarea>
  </label>

  {#snippet footer()}
    <button class="btn ghost" onclick={close}>Annuler</button>
    <button class="btn primary" disabled={busy || !valid} onclick={save}>Enregistrer</button>
  {/snippet}
</Modal>

<style>
  .note,
  .empty {
    margin: 0;
    font-size: 12px;
    color: var(--dim);
    line-height: 1.5;
  }
  .cmd {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 14px;
    border: 1px solid var(--line2);
    border-radius: var(--r);
    background: var(--elev);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .row {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  em {
    color: var(--dim);
    font-style: normal;
  }
  .field {
    font-size: 12.5px;
  }
  select.field {
    min-width: 170px;
  }
  textarea.field {
    height: auto;
    resize: vertical;
    padding: 8px 10px;
  }
  .del {
    width: 36px;
    height: 36px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 15px;
    cursor: pointer;
  }
  .del:hover {
    border-color: var(--del);
    color: var(--del);
  }
  .add {
    align-self: flex-start;
  }
</style>

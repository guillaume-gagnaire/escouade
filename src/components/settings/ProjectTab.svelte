<script lang="ts">
  import { openPath } from '@tauri-apps/plugin-opener';
  import { askCloseProject } from '../../lib/project-actions';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { PROJECT_COLORS } from '../../lib/theme';
  import type { Project } from '../../lib/types';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // A project's own settings: who it is, its worktrees, its launch commands (its draft: the modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);

  /** Asks before closing it, then comes back to these settings, their draft as it was. */
  function askClose() {
    // Taken now: the tab is gone while the confirmation is open.
    const projectId = project.id;
    askCloseProject(project, () => {
      app.modal = { kind: 'settings', tab: 'projects', projectId, resume: true };
    });
  }
</script>

<Group title="Identité">
  <Row label="Nom">
    <input class="field input" aria-label="Nom du projet" spellcheck="false" bind:value={draft.name} />
  </Row>
  <Row label="Dossier">
    <span class="path mono" title={project.path}>{project.path}</span>
    <button class="btn" onclick={() => openPath(project.path).catch((err) => app.toast(String(err), 'error'))}>Ouvrir</button>
  </Row>
  <Row label="Couleur">
    <div class="colors" role="group" aria-label="Couleur">
      {#each PROJECT_COLORS as c, i (c)}
        <button
          class="swatch"
          aria-label="Couleur {i + 1}"
          aria-pressed={draft.color === c}
          style:background={c}
          style:box-shadow={draft.color === c ? '0 0 0 2px var(--bg), 0 0 0 4px var(--text)' : 'none'}
          onclick={() => (draft.color = c)}
        ></button>
      {/each}
    </div>
  </Row>
</Group>

<Group title="Git">
  <Row label="Un worktree par agent" desc="Isole les modifications de chaque nouvel agent dans sa propre branche.">
    <Switch label="Un worktree par agent" bind:on={draft.worktreePerAgent} />
  </Row>
  <Row
    label="Fichiers copiés dans les worktrees"
    desc="Seuls ceux que git ignore, jamais commités ; un motif par ligne : .env* à la racine, **/.env* partout."
    descId="copy-desc"
    wide
  >
    <textarea
      class="field mono copy"
      rows="3"
      placeholder=".env*"
      aria-label="Fichiers copiés dans les worktrees"
      aria-describedby="copy-desc"
      bind:value={draft.copy}
    ></textarea>
  </Row>
</Group>

<Group
  title="Lancement"
  anchor="launch"
  note="Chaque commande tourne dans son propre terminal, en lecture seule. Lance-les depuis la section « Lancement » de la barre latérale."
>
  {#each draft.runCommands as c, i (c.id)}
    <fieldset class="cmd">
      <legend class="sr">Commande {i + 1}</legend>
      <div class="line">
        <label class="f grow"><span>Nom</span><input class="field" bind:value={c.name} placeholder="ex. Front" /></label>
        <label class="f"
          ><span>Shell</span>
          <select class="field" bind:value={c.shell}>
            {#each app.shells as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
            {#if !app.shells.some((s) => s.id === c.shell)}<option value={c.shell}>{c.shell} (introuvable)</option>{/if}
          </select>
        </label>
        <button class="del" title="Supprimer" aria-label="Supprimer" onclick={() => draft.runCommands.splice(i, 1)}>×</button>
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
  <div class="add"><button class="btn" onclick={() => settingsForm.addCommand()}>+ Ajouter une commande</button></div>
</Group>

<Group title="Zone sensible">
  <Row
    label="Fermer le projet"
    desc="Le projet et ses agents sont retirés de l'application, conversations comprises. Les fichiers et les worktrees sur le disque ne sont pas touchés."
  >
    <button class="btn danger" onclick={askClose}>Fermer le projet…</button>
  </Row>
</Group>

<style>
  .input {
    flex: 1;
    min-width: 0;
    max-width: 340px;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-weight: 600;
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 9px;
    max-width: 60%;
  }
  .swatch {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 7px;
    padding: 0;
    cursor: pointer;
  }
  .copy {
    height: auto;
    resize: vertical;
    padding: 8px 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .cmd {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 14px 16px;
    border: none;
    border-bottom: 1px solid var(--line);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .line {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  em {
    color: var(--dim);
    font-style: normal;
  }
  .cmd .field {
    height: 32px;
    background: var(--panel);
    font-size: 12.5px;
  }
  select.field {
    min-width: 170px;
  }
  .del {
    width: 32px;
    height: 32px;
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
  .empty {
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
    color: var(--dim);
  }
  .add {
    padding: 12px 16px;
  }
</style>

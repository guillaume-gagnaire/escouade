<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';
  import { plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { app, type CommitDraft } from '../../lib/state.svelte';
  import type { CommitScope } from '../../lib/types';
  import Modal from './Modal.svelte';

  // A direct commit: the files it takes, a message Haiku proposes that the user reads and edits, and « Commiter »,
  // the only thing that commits. `agentId` null: the project's own checkout, its agents' worktrees apart. `resume`:
  // what it had written, back from a dialog that took its place.
  let { projectId, agentId, resume }: { projectId: string; agentId: string | null; resume?: CommitDraft } = $props();

  const owner = $derived((agentId ? app.agents[agentId]?.name : app.projects.find((p) => p.id === projectId)?.name) ?? '');
  const SC: Record<string, string> = { A: 'var(--add)', M: 'var(--wait)', D: 'var(--del)' };
  /** Rows listed: a commit of thousands of files shows the first ones, and how many more. */
  const SHOWN = 500;

  let scope = $state<CommitScope | null>(null);
  /** Why the files could not be listed. */
  let failure = $state<string | null>(null);
  let message = $state(untrack(() => resume?.message ?? ''));
  /** Haiku's proposal as it was put in the field: leaving the window on it loses nothing. */
  let proposed = untrack(() => resume?.proposed ?? '');
  let proposing = $state(false);
  let noProposal = $state<string | null>(null);
  let committing = $state(false);
  /** Git's refusal of the commit (a hook, nothing left to commit…). */
  let refused = $state<string | null>(null);
  /** Typed since the last proposal was asked for: the proposal arriving then does not replace it. */
  let typed = false;
  let asked = 0;
  /** Replaced by another modal while the commit ran: its outcome goes to a toast. */
  let gone = false;
  /**
   * The dialog it is. Another one taking its place (the confirmation of its closing, « Quitter Escouade ? ») comes back to
   * it once cancelled: what was written is kept in it.
   */
  const self = untrack(() => app.modal);
  onDestroy(() => {
    gone = true;
    if (self?.kind === 'commit') self.resume = { message, proposed };
  });

  const paths = $derived(scope?.files.map((f) => f.path) ?? []);
  const leftOut = $derived.by(() => {
    const files = scope?.leftOut ?? [];
    if (!files.length) return null;
    return files.length === 1
      ? `Jamais commité : ${files[0]} (copié dans les worktrees).`
      : `Jamais commités : ${files.join(', ')} (copiés dans les worktrees).`;
  });

  onMount(async () => {
    try {
      scope = await api.commitPreview(projectId, agentId);
    } catch (e) {
      failure = String(e);
      return;
    }
    // Back with a message: not replaced by a new proposal.
    if (scope.files.length && !message) propose();
  });

  /** Asks Haiku for a message; the field stays editable meanwhile. */
  async function propose() {
    const mine = ++asked;
    proposing = true;
    noProposal = null;
    typed = false;
    try {
      const proposal = await api.commitPropose(projectId, agentId, paths);
      if (mine === asked && !typed) message = proposed = proposal;
    } catch (e) {
      if (mine === asked) noProposal = `Pas de proposition : ${String(e).trim().replace(/\.$/, '')}.`;
    } finally {
      if (mine === asked) proposing = false;
    }
  }

  async function commit() {
    if (!message.trim() || committing || !paths.length) return;
    committing = true;
    refused = null;
    try {
      const hash = await api.commitDirect(projectId, agentId, paths, message);
      // Another modal may have taken its place meanwhile.
      if (!gone) app.modal = null;
      app.toast(`Commit ${hash} créé`, 'ok');
    } catch (e) {
      // A refusal is never lost: in the window, or told once it is gone.
      if (gone) app.toast(String(e), 'error');
      else refused = String(e);
    } finally {
      committing = false;
    }
  }

  /**
   * Not while the commit runs: its outcome (a hook's refusal…) would have nowhere to show. A message written (not the
   * proposal as it came) is given up only once the user agrees; « Annuler » there comes back to it.
   */
  function close() {
    if (committing) return;
    if (!message.trim() || message === proposed) {
      app.modal = null;
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: 'Abandonner le message ?',
      body: 'Le message que tu as écrit pour ce commit sera perdu.',
      confirm: 'Abandonner',
      danger: true,
      onConfirm: () => {},
      onCancel: () => (app.modal = self),
    };
  }
</script>

<Modal title="Commit" width={600} onclose={close}>
  <p class="scope">Modifications de {owner}</p>
  {#if failure}
    <p class="error" role="alert">{failure}</p>
  {:else if scope}
    {#if scope.files.length}
      <ul class="files" aria-label="Fichiers du commit">
        {#each scope.files.slice(0, SHOWN) as f (f.path)}
          <li><span class="st mono" style:color={SC[f.status]}>{f.status}</span> <span class="path mono">{f.path}</span></li>
        {/each}
      </ul>
      {#if scope.files.length > SHOWN}<p class="note">… et {plural(scope.files.length - SHOWN, 'autre fichier', 'autres fichiers')}</p>{/if}
    {:else}
      <p class="note">
        {agentId
          ? 'Aucun fichier à commiter pour cet agent.'
          : 'Aucune modification dans le dossier du projet : le worktree d’un agent se commite depuis « Cet agent ».'}
      </p>
    {/if}
    {#if leftOut}<p class="note">{leftOut}</p>{/if}
  {/if}
  <div class="grp">
    <label class="lab" for="commit-message">Message</label>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea id="commit-message" class="field mono message" rows="6" bind:value={message} oninput={() => (typed = true)} autofocus
    ></textarea>
    <p class="note" aria-live="polite">{proposing ? 'Haiku rédige le message…' : (noProposal ?? '')}</p>
    {#if refused}<p class="error" role="alert">{refused}</p>{/if}
  </div>
  {#snippet footer()}
    <button class="btn ghost" disabled={committing} onclick={close}>Annuler</button>
    <button class="btn" disabled={proposing || committing || !paths.length} onclick={propose}>Régénérer</button>
    <button class="btn primary" disabled={!message.trim() || committing || !paths.length} onclick={commit}
      >{committing ? 'Commit…' : 'Commiter'}</button
    >
  {/snippet}
</Modal>

<style>
  .scope {
    margin: -8px 0 0;
    font-size: 13px;
    color: var(--muted);
  }
  .files {
    list-style: none;
    margin: 0;
    padding: 6px 0;
    max-height: 180px;
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--bg);
  }
  .files li {
    display: flex;
    gap: 10px;
    padding: 3px 12px;
    font-size: 12px;
  }
  .st {
    width: 12px;
    flex: none;
    font-weight: 700;
  }
  .path {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lab {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .message {
    height: auto;
    padding: 8px 12px;
    resize: vertical;
    font-size: 12.5px;
    line-height: 1.5;
  }
  .note {
    margin: 0;
    min-height: 1em;
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .error {
    margin: 0;
    font-size: 12.5px;
    color: var(--del);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>

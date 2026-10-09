<script lang="ts">
  import { untrack } from 'svelte';
  import { app } from '../../lib/state.svelte';
  import type { Ticket, TicketDraft } from '../../lib/types';

  let { ticket, onsubmit, oncancel }: { ticket?: Ticket; onsubmit: (d: TicketDraft) => void | Promise<void>; oncancel: () => void } =
    $props();

  // What the form opened with, to tell whether leaving it loses anything.
  const start = untrack(() => ({
    title: ticket?.title ?? '',
    description: ticket?.description ?? '',
    criteria: ticket?.criteria.map((c) => c.text).join('\n') ?? '',
    maxLoops: ticket?.maxLoops ?? 5,
  }));

  let title = $state(start.title);
  let description = $state(start.description);
  let criteria = $state(start.criteria);
  let maxLoops = $state(start.maxLoops);
  let busy = $state(false);
  const ready = $derived(title.trim().length > 0 && !busy);
  const changed = $derived(
    title !== start.title || description !== start.description || criteria !== start.criteria || maxLoops !== start.maxLoops,
  );

  /** Leaves the form; what was typed is only given up once the user agrees. */
  function leave() {
    if (!changed) {
      oncancel();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: 'Abandonner les modifications ?',
      body: 'Ce que tu as saisi dans ce ticket ne sera pas enregistré.',
      confirm: 'Abandonner',
      danger: true,
      onConfirm: oncancel,
    };
  }

  function escape(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    // The confirmation closes on Escape through a window listener, which is already there when this very event reaches the window: it would close at once.
    if (changed) e.stopPropagation();
    leave();
  }

  async function submit() {
    if (!ready) return;
    busy = true;
    await onsubmit({
      title: title.trim(),
      description: description.trim(),
      criteria: criteria
        .split('\n')
        .map((l) => l.trim())
        .filter(Boolean),
      maxLoops,
    });
    busy = false;
  }
</script>

<!-- Escape leaves the form from any of its fields; Enter only submits from the title (the textareas take it as a new line). -->
<div class="form" role="presentation" onkeydown={escape}>
  <!-- svelte-ignore a11y_autofocus -->
  <input
    class="title"
    placeholder="Titre du ticket"
    aria-label="Titre du ticket"
    bind:value={title}
    autofocus
    onkeydown={(e) => e.key === 'Enter' && submit()}
  />
  <textarea rows="2" placeholder="Description (facultative)" aria-label="Description" bind:value={description}></textarea>
  <textarea rows="4" placeholder="Critères d'acceptation, un par ligne" aria-label="Critères d'acceptation" bind:value={criteria}
  ></textarea>
  <div class="loops" role="group" aria-label="Boucles max">
    <span class="k">Boucles max</span>
    {#each [3, 5, 8] as n (n)}
      <button class:on={maxLoops === n} aria-pressed={maxLoops === n} onclick={() => (maxLoops = n)}>{n}</button>
    {/each}
  </div>
  <div class="actions">
    <button class="btn ghost" onclick={leave}>Annuler</button>
    <button class="btn primary" disabled={!ready} onclick={submit}>{ticket ? 'Enregistrer' : 'Ajouter'}</button>
  </div>
</div>

<style>
  .form {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-radius: var(--r-sm);
    background: var(--elev);
    border: 1px solid var(--accent);
  }
  .title,
  textarea {
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .title {
    height: 32px;
    font-size: 13px;
    font-weight: 600;
  }
  textarea {
    resize: none;
    padding: 8px 10px;
    font-size: 12px;
    line-height: 1.5;
  }
  .loops {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .k {
    font-size: 11px;
    color: var(--dim);
    margin-right: 4px;
  }
  .loops button {
    height: 24px;
    min-width: 28px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .loops button.on {
    background: var(--elev2);
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .actions .btn {
    height: 28px;
    font-size: 12px;
  }
</style>

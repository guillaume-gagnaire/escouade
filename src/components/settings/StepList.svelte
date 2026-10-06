<script lang="ts">
  import { app } from '../../lib/state.svelte';
  import type { WorktreeStep } from '../../lib/types';

  // The commands a project runs in its worktrees (their setup, or their teardown), in order: shell, command line,
  // folder of the worktree.
  let { steps = $bindable(), label, onadd }: { steps: WorktreeStep[]; label: string; onadd: () => void } = $props();
</script>

<div class="steps">
  {#each steps as s, i (s.id)}
    <div class="step" role="group" aria-label="{label} {i + 1}">
      <span class="n mono" aria-hidden="true">{i + 1}</span>
      <select class="field" aria-label="Shell" bind:value={s.shell}>
        {#each app.shells as sh (sh.id)}<option value={sh.id}>{sh.label}</option>{/each}
        {#if !app.shells.some((sh) => sh.id === s.shell)}<option value={s.shell}>{s.shell || 'Shell'} (introuvable)</option>{/if}
      </select>
      <input class="field mono cmd" aria-label="Commande" placeholder="ex. npm ci" spellcheck="false" bind:value={s.command} />
      <input
        class="field mono dir"
        aria-label="Sous-dossier"
        title="Sous-dossier du worktree (vide : sa racine)"
        placeholder="sous-dossier"
        spellcheck="false"
        bind:value={s.cwd}
      />
      <button class="del" title="Supprimer" aria-label="Supprimer la commande {i + 1}" onclick={() => steps.splice(i, 1)}>×</button>
    </div>
  {/each}
  <div><button class="btn" aria-label="Ajouter une {label.toLowerCase()}" onclick={onadd}>+ Ajouter une commande</button></div>
</div>

<style>
  .steps {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .step {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .n {
    width: 16px;
    font-size: 11px;
    color: var(--dim);
    text-align: right;
  }
  .field {
    height: 32px;
    background: var(--panel);
    font-size: 12.5px;
  }
  select.field {
    width: 150px;
  }
  .cmd {
    flex: 1;
    min-width: 180px;
  }
  .dir {
    width: 130px;
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
</style>

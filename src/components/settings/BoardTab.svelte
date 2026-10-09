<script lang="ts">
  import { commitPreview } from '../../lib/board';
  import { fPct } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { EFFORTS, MODES, modelLabel, modelOptions } from '../../lib/models';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { BoardAction, BoardSettings, Project } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // A project's Kanban: what validating a ticket does, and the agents that take its tickets (its draft:
  // the modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  const b = $derived(draft.board);
  const target = $derived(b.target || app.git[project.id]?.branch || 'main');
  /** The local branches, by project. */
  let branches = $state<Record<string, string[]>>({});

  $effect(() => {
    const id = project.id;
    if (branches[id]) return;
    api
      .gitBranches(id)
      .then((list) => {
        branches[id] = list;
      })
      .catch(() => {});
  });

  const ACTIONS: { id: BoardAction; label: string; desc: string }[] = [
    { id: 'merge', label: 'Merger dans une branche', desc: "Fusionne le worktree de l'agent dans la branche cible, puis libère l'agent." },
    { id: 'pr', label: 'Ouvrir une pull request', desc: 'Pousse ticket/<clé> et ouvre une PR vers la branche cible pour relecture.' },
    { id: 'push', label: 'Pousser la branche du ticket', desc: 'Commit et push sur ticket/<clé>, sans merge ni PR.' },
    { id: 'keep', label: "Laisser en l'état", desc: 'Les modifications restent non commitées dans le worktree.' },
  ];
  const STRATEGIES: { value: BoardSettings['strategy']; label: string }[] = [
    { value: 'merge', label: 'Merge commit' },
    { value: 'squash', label: 'Squash' },
    { value: 'rebase', label: 'Rebase' },
  ];
  const CONFLICTS: { value: BoardSettings['conflict']; label: string }[] = [
    { value: 'ask', label: 'Me demander' },
    { value: 'agent', label: "L'agent résout" },
    { value: 'abort', label: 'Annuler' },
  ];
  const PARALLEL = [1, 2, 3, 4, 5, 6].map((n) => ({ value: n, label: String(n) }));
  /** As the backend offers them (`board::QUOTA_PAUSES`). */
  const QUOTA_PAUSES = [80, 90, 95, 100].map((n) => ({ value: n, label: fPct(n) }));
</script>

<Group title="Quand je valide un ticket « À tester »" plain>
  <div class="actions" role="radiogroup" aria-label="Quand je valide un ticket « À tester »">
    {#each ACTIONS as a (a.id)}
      <button class="action" class:on={b.action === a.id} role="radio" aria-checked={b.action === a.id} onclick={() => (b.action = a.id)}>
        <span class="ring"><span class="dot"></span></span>
        <span class="txt"><span class="l">{a.label}</span><span class="d">{a.desc}</span></span>
      </button>
    {/each}
  </div>
  {#if b.action === 'merge' || b.action === 'pr'}
    <div class="box">
      <div class="line">
        <span class="k">Branche cible</span>
        {#each branches[project.id]?.length ? branches[project.id] : [target] as br (br)}
          <button class="chip mono" class:on={target === br} aria-pressed={target === br} onclick={() => (b.target = br)}>⎇ {br}</button>
        {/each}
      </div>
      {#if b.action === 'merge'}
        <div class="line">
          <span class="k">Stratégie</span>
          {#each STRATEGIES as st (st.value)}
            <button
              class="chip"
              class:on={b.strategy === st.value}
              aria-pressed={b.strategy === st.value}
              onclick={() => (b.strategy = st.value)}>{st.label}</button
            >
          {/each}
        </div>
      {:else}
        <div class="line">
          <span class="k">PR en brouillon</span>
          <Switch label="PR en brouillon" bind:on={b.draft} />
        </div>
      {/if}
    </div>
  {/if}
</Group>

<Group title="Avant et après">
  <Row label="Commande de tests" hint="lancée dans le worktree du ticket">
    <input
      class="field mono input"
      placeholder="ex. npm test"
      aria-label="Commande de tests"
      spellcheck="false"
      bind:value={b.testCommand}
    />
  </Row>
  <Row label="Relancer les tests avant" desc="Bloque l'action si un test échoue et renvoie le ticket à l'agent.">
    <Switch
      label="Relancer les tests avant"
      disabled={!b.testCommand.trim()}
      bind:on={() => b.testsFirst && !!b.testCommand.trim(), (v) => (b.testsFirst = v)}
    />
  </Row>
  <Row
    label="Supprimer le worktree une fois validé"
    desc="Libère l'espace disque, après ses commandes de démontage. Après un merge, sa branche part aussi ; poussée ou proposée en PR, elle reste."
  >
    <Switch label="Supprimer le worktree une fois validé" bind:on={b.cleanup} />
  </Row>
  <Row label="Message de commit généré" desc={commitPreview(b, draft.name.trim() || project.name)}>
    <Switch label="Message de commit généré" bind:on={b.conventional} />
  </Row>
  <Row label="En cas de conflit">
    <Chips label="En cas de conflit" options={CONFLICTS} bind:value={b.conflict} />
  </Row>
</Group>

<Group title="Pilote auto">
  <Row label="Attribuer les tickets automatiquement" desc="Un agent libre prend le prochain ticket « À faire »">
    <Switch label="Attribuer les tickets automatiquement" bind:on={b.autopilot} />
  </Row>
  <!-- The quotas are the account's: an app setting, shown with each project's autopilot. -->
  <Row
    label="Pause au-delà du quota"
    hint="pour tous les projets"
    desc="Aucun ticket ne démarre tant que la fenêtre de 5 h ou la fenêtre hebdomadaire dépasse ce seuil."
  >
    <Chips label="Pause au-delà du quota" options={QUOTA_PAUSES} mono bind:value={settingsForm.settings.quotaPause} />
  </Row>
</Group>

<Group title="Agents">
  <Row label="En parallèle" desc="Au-delà, les tickets « À faire » attendent une place libre">
    <Chips label="En parallèle" options={PARALLEL} mono bind:value={b.maxParallel} />
  </Row>
  <Row label="Modèle">
    <select class="field select" aria-label="Modèle" bind:value={b.model}>
      <option value="">Par défaut ({modelLabel(settingsForm.settings.defaultModel, app.models)})</option>
      {#each modelOptions(app.models) as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
    </select>
  </Row>
  <Row label="Effort">
    <select class="field select" aria-label="Effort" bind:value={b.effort}>
      <option value="">Par défaut</option>
      {#each EFFORTS as e (e.value)}<option value={e.value}>{e.label}</option>{/each}
    </select>
  </Row>
  <Row label="Mode">
    <select class="field select" aria-label="Mode" bind:value={b.mode}>
      <option value="">Par défaut</option>
      {#each MODES as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
    </select>
  </Row>
</Group>

<style>
  .actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 8px;
  }
  .action {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 12px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .action.on {
    border-color: var(--accent);
    background: var(--elev);
  }
  .ring {
    width: 14px;
    height: 14px;
    flex: none;
    margin-top: 2px;
    border-radius: 50%;
    border: 1.5px solid var(--line2);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .action.on .ring {
    border-color: var(--accent);
  }
  .action.on .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .txt {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }
  .l {
    font-size: 13px;
    font-weight: 600;
  }
  .d {
    font-size: 11.5px;
    line-height: 1.45;
    color: var(--muted);
    text-wrap: pretty;
  }
  .box {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
    border-radius: var(--r);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .k {
    width: 110px;
    font-size: 12px;
    color: var(--muted);
  }
  .chip {
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip.mono {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
  .input {
    width: 220px;
    max-width: 50%;
    flex: none;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .select {
    width: 240px;
    max-width: 50%;
    flex: none;
    height: 32px;
    background: var(--panel);
    font-size: 12px;
  }
</style>

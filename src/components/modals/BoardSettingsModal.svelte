<script lang="ts">
  import { onMount } from 'svelte';
  import { commitPreview } from '../../lib/board';
  import { api } from '../../lib/ipc';
  import { EFFORTS, MODES, modelLabel, modelOptions } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import type { BoardAction, BoardSettings } from '../../lib/types';
  import Modal from './Modal.svelte';

  let { projectId }: { projectId: string } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  const s = $derived(project?.board);
  const target = $derived(s?.target || app.git[projectId]?.branch || 'main');
  let branches = $state<string[]>([]);

  onMount(() => {
    api
      .gitBranches(projectId)
      .then((b) => (branches = b))
      .catch(() => {});
  });

  const ACTIONS: { id: BoardAction; label: string; desc: string }[] = [
    { id: 'merge', label: 'Merger dans une branche', desc: "Fusionne le worktree de l'agent dans la branche cible, puis libère l'agent." },
    { id: 'pr', label: 'Ouvrir une pull request', desc: 'Pousse ticket/<clé> et ouvre une PR vers la branche cible pour relecture.' },
    { id: 'push', label: 'Pousser la branche du ticket', desc: 'Commit et push sur ticket/<clé>, sans merge ni PR.' },
    { id: 'keep', label: "Laisser en l'état", desc: 'Les modifications restent non commitées dans le worktree.' },
  ];
  const STRATEGIES: { id: BoardSettings['strategy']; label: string }[] = [
    { id: 'merge', label: 'Merge commit' },
    { id: 'squash', label: 'Squash' },
    { id: 'rebase', label: 'Rebase' },
  ];
  const CONFLICTS: { id: BoardSettings['conflict']; label: string }[] = [
    { id: 'ask', label: 'Me demander' },
    { id: 'agent', label: "L'agent résout" },
    { id: 'abort', label: 'Annuler' },
  ];

  /** Applied at once: shown right away, then saved; put back if the backend refuses it. */
  async function set(patch: Partial<BoardSettings>) {
    if (!project || !s) return;
    const before = $state.snapshot(s);
    const next = { ...before, ...patch };
    project.board = next;
    const saved = await app.run(api.boardSet(projectId, next));
    if (saved) app.replaceProject(saved);
    else if (project) project.board = before;
  }

  const close = () => (app.modal = null);
</script>

<Modal title="Réglages du tableau" width={600} onclose={close}>
  {#if project && s}
    <span class="sub mono">{project.name}</span>
    <section class="group">
      <span class="h">Quand je valide un ticket « À tester »</span>
      <div class="actions" role="radiogroup" aria-label="Quand je valide un ticket « À tester »">
        {#each ACTIONS as a (a.id)}
          <button
            class="action"
            class:on={s.action === a.id}
            role="radio"
            aria-checked={s.action === a.id}
            onclick={() => set({ action: a.id })}
          >
            <span class="ring"><span class="dot"></span></span>
            <span class="txt"><span class="l">{a.label}</span><span class="d">{a.desc}</span></span>
          </button>
        {/each}
      </div>
    </section>
    {#if s.action === 'merge' || s.action === 'pr'}
      <div class="box">
        <div class="row">
          <span class="k">Branche cible</span>
          {#each branches.length ? branches : [target] as b (b)}
            <button class="chip mono" class:on={target === b} aria-pressed={target === b} onclick={() => set({ target: b })}>⎇ {b}</button>
          {/each}
        </div>
        {#if s.action === 'merge'}
          <div class="row">
            <span class="k">Stratégie</span>
            {#each STRATEGIES as st (st.id)}
              <button
                class="chip"
                class:on={s.strategy === st.id}
                aria-pressed={s.strategy === st.id}
                onclick={() => set({ strategy: st.id })}>{st.label}</button
              >
            {/each}
          </div>
        {:else}
          <div class="row">
            <span class="k">Ouvrir en brouillon</span>
            <button
              class="switch"
              class:on={s.draft}
              role="switch"
              aria-checked={s.draft}
              aria-label="Ouvrir en brouillon"
              onclick={() => set({ draft: !s.draft })}
            ></button>
          </div>
        {/if}
      </div>
    {/if}
    <div class="sep"></div>
    <div class="toggles">
      <div class="tg">
        <div class="txt">
          <span class="l">Relancer les tests avant</span>
          <span class="d">Bloque l'action si un test échoue et renvoie le ticket à l'agent.</span>
          <input
            class="field mono cmd"
            placeholder="ex. npm test"
            aria-label="Commande de tests"
            value={s.testCommand}
            onchange={(e) => {
              const testCommand = e.currentTarget.value.trim();
              set({ testCommand, testsFirst: testCommand ? s.testsFirst : false });
            }}
          />
        </div>
        <button
          class="switch"
          class:on={s.testsFirst && !!s.testCommand}
          role="switch"
          aria-checked={s.testsFirst && !!s.testCommand}
          aria-label="Relancer les tests avant"
          disabled={!s.testCommand.trim()}
          onclick={() => set({ testsFirst: !s.testsFirst })}
        ></button>
      </div>
      <div class="tg">
        <div class="txt">
          <span class="l">Supprimer le worktree après merge</span>
          <span class="d">Libère l'espace disque et repart d'une branche propre.</span>
        </div>
        <button
          class="switch"
          class:on={s.cleanup}
          role="switch"
          aria-checked={s.cleanup}
          aria-label="Supprimer le worktree après merge"
          onclick={() => set({ cleanup: !s.cleanup })}
        ></button>
      </div>
      <div class="tg">
        <div class="txt">
          <span class="l">Message de commit généré</span>
          <span class="d">Format Conventional Commits, avec la clé du ticket.</span>
        </div>
        <button
          class="switch"
          class:on={s.conventional}
          role="switch"
          aria-checked={s.conventional}
          aria-label="Message de commit généré"
          onclick={() => set({ conventional: !s.conventional })}
        ></button>
      </div>
      <div class="preview mono">{commitPreview(s, project.name)}</div>
      <div class="row">
        <span class="l grow">En cas de conflit</span>
        {#each CONFLICTS as c (c.id)}
          <button class="chip" class:on={s.conflict === c.id} aria-pressed={s.conflict === c.id} onclick={() => set({ conflict: c.id })}
            >{c.label}</button
          >
        {/each}
      </div>
    </div>
    <div class="sep"></div>
    <section class="group">
      <span class="h">Agents</span>
      <div class="row">
        <span class="k">En parallèle</span>
        {#each [1, 2, 3, 4, 5, 6] as n (n)}
          <button
            class="chip mono"
            class:on={s.maxParallel === n}
            aria-pressed={s.maxParallel === n}
            onclick={() => set({ maxParallel: n })}>{n}</button
          >
        {/each}
      </div>
      <div class="row">
        <span class="k">Modèle</span>
        <select class="field" aria-label="Modèle" value={s.model} onchange={(e) => set({ model: e.currentTarget.value })}>
          <option value="">Comme les réglages ({modelLabel(app.settings.defaultModel, app.models)})</option>
          {#each modelOptions(app.models) as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
        </select>
      </div>
      <div class="row">
        <span class="k">Effort</span>
        <select class="field" aria-label="Effort" value={s.effort} onchange={(e) => set({ effort: e.currentTarget.value })}>
          <option value="">Comme les réglages</option>
          {#each EFFORTS as e (e.value)}<option value={e.value}>{e.label}</option>{/each}
        </select>
      </div>
      <div class="row">
        <span class="k">Mode</span>
        <select class="field" aria-label="Mode" value={s.mode} onchange={(e) => set({ mode: e.currentTarget.value })}>
          <option value="">Comme les réglages</option>
          {#each MODES as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
        </select>
      </div>
    </section>
  {/if}
  {#snippet footer()}
    <button class="btn primary" onclick={close}>Terminé</button>
  {/snippet}
</Modal>

<style>
  .sub {
    margin-top: -14px;
    font-size: 11px;
    color: var(--dim);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .h {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
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
  .row {
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
  .grow {
    flex: 1;
    min-width: 140px;
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
  .sep {
    height: 1px;
    background: var(--line);
  }
  .toggles {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .tg {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .cmd {
    margin-top: 4px;
    height: 30px;
    font-size: 12px;
  }
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .preview {
    font-size: 11px;
    padding: 8px 10px;
    border-radius: var(--r-sm);
    background: var(--bg);
    border: 1px solid var(--line);
    color: var(--muted);
  }
  select.field {
    min-width: 220px;
    height: 30px;
    font-size: 12px;
  }
</style>

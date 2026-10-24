<script lang="ts">
  import { newAgentDefaults } from '../../lib/accounts';
  import { commitPreview } from '../../lib/board';
  import { fPct } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { EFFORTS, MODES, modelLabel, modelOptions } from '../../lib/models';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { BoardAction, BoardSettings, Project } from '../../lib/types';
  import BranchPicker from '../branches/BranchPicker.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import SharedMark from './SharedMark.svelte';
  import Switch from './Switch.svelte';

  // A project's Kanban: what validating a ticket does, and the agents that take its tickets (its draft:
  // the modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  const b = $derived(draft.board);
  const target = $derived(b.target || app.git[project.id]?.branch || 'main');
  /** The target branch is chosen in the branch picker, searched: a repository may have many branches. */
  let choosing = $state(false);
  let targetButton = $state<HTMLButtonElement>();

  // The choices below are texts: derived, so that they follow the language of the interface.
  const ACTION_IDS: BoardAction[] = ['merge', 'pr', 'push', 'keep'];
  const actions = $derived(
    ACTION_IDS.map((id) => ({
      id,
      label: t(`boardSettings.tab.actions.${id}.label`),
      desc:
        id === 'pr' || id === 'push'
          ? t(`boardSettings.tab.actions.${id}.desc`, { prefix: draft.ticketBranchPrefix.trim() })
          : t(`boardSettings.tab.actions.${id}.desc`),
    })),
  );
  const STRATEGY_IDS: BoardSettings['strategy'][] = ['merge', 'squash', 'rebase'];
  const strategies = $derived(STRATEGY_IDS.map((value) => ({ value, label: t(`boardSettings.tab.strategies.${value}`) })));
  const CONFLICT_IDS: BoardSettings['conflict'][] = ['ask', 'agent', 'abort'];
  const conflicts = $derived(CONFLICT_IDS.map((value) => ({ value, label: t(`boardSettings.tab.conflicts.${value}`) })));
  const PARALLEL = [1, 2, 3, 4, 5, 6].map((n) => ({ value: n, label: String(n) }));
  /** As the backend offers them (`board::QUOTA_PAUSES`). */
  const quotaPauses = $derived([80, 90, 95, 100].map((n) => ({ value: n, label: fPct(n) })));
</script>

<Group title={t('boardSettings.tab.actionsTitle')} plain>
  <div class="actions" role="radiogroup" aria-label={t('boardSettings.tab.actionsTitle')}>
    {#each actions as a (a.id)}
      <button class="action" class:on={b.action === a.id} role="radio" aria-checked={b.action === a.id} onclick={() => (b.action = a.id)}>
        <span class="ring"><span class="dot"></span></span>
        <span class="txt"><span class="l">{a.label}</span><span class="d">{a.desc}</span></span>
      </button>
    {/each}
  </div>
  <SharedMark keys="board.on_approve" />
  {#if b.action === 'merge' || b.action === 'pr'}
    <div class="box">
      <div class="line">
        <span class="k">{t('boardSettings.tab.targetBranch')}</span>
        <button
          class="chip mono on"
          aria-haspopup="dialog"
          aria-label={t('boardSettings.tab.targetBranchChoose', { branch: target })}
          bind:this={targetButton}
          onclick={() => (choosing = true)}>⎇ {target} ▾</button
        >
      </div>
      <SharedMark keys="board.target_branch" />
      {#if b.action === 'merge'}
        <div class="line">
          <span class="k">{t('boardSettings.tab.strategy')}</span>
          {#each strategies as st (st.value)}
            <button
              class="chip"
              class:on={b.strategy === st.value}
              aria-pressed={b.strategy === st.value}
              onclick={() => (b.strategy = st.value)}>{st.label}</button
            >
          {/each}
        </div>
        <SharedMark keys="board.merge_strategy" />
      {:else}
        <div class="line">
          <span class="k">{t('boardSettings.tab.draftPr')}</span>
          <Switch label={t('boardSettings.tab.draftPr')} bind:on={b.draft} />
        </div>
        <SharedMark keys="board.draft_pr" />
      {/if}
    </div>
  {/if}
</Group>

{#if choosing}
  <BranchPicker
    over
    projectId={project.id}
    anchor={targetButton}
    mode="target"
    chosen={target}
    onclose={() => (choosing = false)}
    onpick={(br) => (b.target = br.name)}
  />
{/if}

<Group title={t('boardSettings.tab.beforeAfter')}>
  <Row label={t('boardSettings.tab.testCommand')} hint={t('boardSettings.tab.testCommandHint')} shared="board.test_command">
    <input
      class="field mono input"
      placeholder={t('boardSettings.tab.testCommandPlaceholder')}
      aria-label={t('boardSettings.tab.testCommand')}
      spellcheck="false"
      bind:value={b.testCommand}
    />
  </Row>
  <Row label={t('boardSettings.tab.testsFirst')} desc={t('boardSettings.tab.testsFirstDesc')} shared="board.tests_before_approve">
    <Switch
      label={t('boardSettings.tab.testsFirst')}
      disabled={!b.testCommand.trim()}
      bind:on={() => b.testsFirst && !!b.testCommand.trim(), (v) => (b.testsFirst = v)}
    />
  </Row>
  <Row label={t('boardSettings.tab.cleanup')} desc={t('boardSettings.tab.cleanupDesc')} shared="board.remove_worktree">
    <Switch label={t('boardSettings.tab.cleanup')} bind:on={b.cleanup} />
  </Row>
  <Row
    label={t('boardSettings.tab.commitMessage')}
    desc={commitPreview(b, draft.name.trim() || project.name)}
    shared="git.conventional_commits"
  >
    <Switch label={t('boardSettings.tab.commitMessage')} bind:on={b.conventional} />
  </Row>
  <Row label={t('boardSettings.tab.onConflict')} shared="board.on_conflict">
    <Chips label={t('boardSettings.tab.onConflict')} options={conflicts} bind:value={b.conflict} />
  </Row>
</Group>

<Group title={t('boardSettings.tab.autopilot')}>
  <Row label={t('boardSettings.tab.autoAssign')} desc={t('boardSettings.tab.autoAssignDesc')}>
    <Switch label={t('boardSettings.tab.autoAssign')} bind:on={b.autopilot} />
  </Row>
  <!-- The quotas are the account's: an app setting, shown with each project's autopilot. -->
  <Row label={t('boardSettings.tab.quotaPause')} hint={t('boardSettings.tab.quotaPauseHint')} desc={t('boardSettings.tab.quotaPauseDesc')}>
    <Chips label={t('boardSettings.tab.quotaPause')} options={quotaPauses} mono bind:value={settingsForm.settings.quotaPause} />
  </Row>
</Group>

<Group title={t('common.agents')}>
  <Row label={t('boardSettings.tab.parallel')} desc={t('boardSettings.tab.parallelDesc')}>
    <Chips label={t('boardSettings.tab.parallel')} options={PARALLEL} mono bind:value={b.maxParallel} />
  </Row>
  <Row label={t('common.model')} shared="board.model">
    <select class="field select" aria-label={t('common.model')} bind:value={b.model}>
      <option value="">{t('boardSettings.tab.defaultModel', { model: modelLabel(newAgentDefaults(project).model, app.models) })}</option>
      {#each modelOptions(app.models) as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
    </select>
  </Row>
  <Row label={t('boardSettings.tab.effort')} shared="board.effort">
    <select class="field select" aria-label={t('boardSettings.tab.effort')} bind:value={b.effort}>
      <option value="">{t('boardSettings.tab.default')}</option>
      {#each EFFORTS as e (e.value)}<option value={e.value}>{e.label}</option>{/each}
    </select>
  </Row>
  <Row label={t('boardSettings.tab.mode')} shared="board.mode">
    <select class="field select" aria-label={t('boardSettings.tab.mode')} bind:value={b.mode}>
      <option value="">{t('boardSettings.tab.default')}</option>
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

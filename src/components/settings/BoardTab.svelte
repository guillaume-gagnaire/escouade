<script lang="ts">
  import Icon from '../Icon.svelte';
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
  import { joinNames, splitNames } from '../../lib/ticket-prs';

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
  const PR_UPDATE_IDS: BoardSettings['prUpdate'][] = ['merge', 'rebase'];
  const prUpdates = $derived(PR_UPDATE_IDS.map((value) => ({ value, label: t(`boardSettings.tab.prUpdates.${value}`) })));
  const conflicts = $derived(CONFLICT_IDS.map((value) => ({ value, label: t(`boardSettings.tab.conflicts.${value}`) })));
  const PARALLEL = [1, 2, 3, 4, 5, 6].map((n) => ({ value: n, label: String(n) }));
  /** As the backend offers them (`board::QUOTA_PAUSES`). */
  const quotaPauses = $derived([80, 90, 95, 100].map((n) => ({ value: n, label: fPct(n) })));
</script>

<Group title={t('boardSettings.tab.actionsTitle')} plain>
  <div class="w-radios actions" role="radiogroup" aria-label={t('boardSettings.tab.actionsTitle')}>
    {#each actions as a (a.id)}
      <button class="w-radio" role="radio" aria-checked={b.action === a.id} onclick={() => (b.action = a.id)}>
        <span class="dot" aria-hidden="true"></span>
        <span><b>{a.label}</b><span class="d">{a.desc}</span></span>
      </button>
    {/each}
  </div>
  <SharedMark keys="board.on_approve" />
  {#if b.action === 'merge' || b.action === 'pr'}
    <div class="box">
      <div class="line">
        <span class="k">{t('boardSettings.tab.targetBranch')}</span>
        <button
          class="pick mono"
          aria-haspopup="dialog"
          aria-label={t('boardSettings.tab.targetBranchChoose', { branch: target })}
          bind:this={targetButton}
          onclick={() => (choosing = true)}><Icon name="branch" size={14} />{target}<Icon name="down" size={14} /></button
        >
      </div>
      <SharedMark keys="board.target_branch" />
      {#if b.action === 'merge'}
        <div class="line">
          <span class="k">{t('boardSettings.tab.strategy')}</span>
          <span class="segmented" role="group" aria-label={t('boardSettings.tab.strategy')}>
            {#each strategies as st (st.value)}
              <button class:on={b.strategy === st.value} aria-pressed={b.strategy === st.value} onclick={() => (b.strategy = st.value)}
                >{st.label}</button
              >
            {/each}
          </span>
        </div>
        <SharedMark keys="board.merge_strategy" />
      {:else}
        <div class="line">
          <span class="k">{t('boardSettings.tab.draftPr')}</span>
          <Switch label={t('boardSettings.tab.draftPr')} bind:on={b.draft} />
        </div>
        <SharedMark keys="board.draft_pr" />
        <!-- Who its pull requests ask for a review, and their labels (ESC-59): logins and names, by commas. -->
        <label class="line">
          <span class="k">{t('boardSettings.tab.reviewers')}</span>
          <input
            class="field mono input names"
            placeholder={t('boardSettings.tab.reviewersPlaceholder')}
            spellcheck="false"
            value={joinNames(b.reviewers)}
            onchange={(e) => (b.reviewers = splitNames(e.currentTarget.value))}
          />
        </label>
        <label class="line">
          <span class="k">{t('boardSettings.tab.labels')}</span>
          <input
            class="field mono input names"
            placeholder={t('boardSettings.tab.labelsPlaceholder')}
            spellcheck="false"
            value={joinNames(b.labels)}
            onchange={(e) => (b.labels = splitNames(e.currentTarget.value))}
          />
        </label>
      {/if}
    </div>
  {/if}
  {#if b.action === 'pr'}
    <!-- After its review (ESC-61): answers under the threads its agent fixed, off by default (never resolved); how a conflict with the target is settled. -->
    <Row label={t('boardSettings.tab.replyThreads')} desc={t('boardSettings.tab.replyThreadsDesc')}>
      <Switch label={t('boardSettings.tab.replyThreads')} bind:on={b.replyThreads} />
    </Row>
    <Row label={t('boardSettings.tab.prUpdate')} desc={t('boardSettings.tab.prUpdateDesc')}>
      <span class="segmented" role="group" aria-label={t('boardSettings.tab.prUpdate')} data-pr-update>
        {#each prUpdates as u (u.value)}
          <button
            class:on={(b.prUpdate || 'merge') === u.value}
            aria-pressed={(b.prUpdate || 'merge') === u.value}
            onclick={() => (b.prUpdate = u.value)}>{u.label}</button
          >
        {/each}
      </span>
    </Row>
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
  <!-- ESC-63: off by default; the target's CI is read from its code host, like the pull requests of the tickets. -->
  <Row label={t('boardSettings.tab.redTargetPause')} desc={t('boardSettings.tab.redTargetPauseDesc')}>
    <Switch label={t('boardSettings.tab.redTargetPause')} bind:on={() => !!b.pauseOnRedTarget, (v) => (b.pauseOnRedTarget = v)} />
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
  /* What validating a ticket does: the radios of the windows (`.w-radio`, src/app.css, lot 11a). */
  .actions {
    margin-top: 10px;
  }
  /* The branch and the strategy of the action chosen, under it. */
  .box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 18px;
    margin: 6px 0 8px 28px;
  }
  .line {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .k {
    color: var(--muted);
    font-size: 12px;
  }
  .names {
    flex: 1 1 200px;
    min-width: 0;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 9px 0 11px;
    border: none;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12px;
    font-weight: 550;
    cursor: pointer;
  }
  .pick:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .pick :global(.icon) {
    color: var(--muted);
  }
  .input {
    width: 220px;
    max-width: 100%;
    flex: none;
  }
  .select {
    width: 240px;
    max-width: 100%;
  }
</style>

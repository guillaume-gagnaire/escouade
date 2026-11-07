<script lang="ts">
  import { newAgentDefaults } from '../../lib/accounts';
  import { t } from '../../lib/i18n';
  import { EFFORTS, MODES, modelLabel, modelOptions } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import type { BoardAction, BoardSettings } from '../../lib/types';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Chips from '../settings/Chips.svelte';
  import Row from '../settings/Row.svelte';
  import Switch from '../settings/Switch.svelte';
  import Icon from '../Icon.svelte';
  import Checks from './Checks.svelte';

  // « Kanban »: what validating a ticket does (action, target branch, strategy), what goes with it (tests, cleanup,
  // conventional message, conflicts), and the agents of the tickets (in parallel, autopilot, model, effort, mode).
  let { form }: { form: WizardForm } = $props();

  const b = $derived(form.config.board);
  const branches = $derived(form.setup?.branches ?? []);
  /** The target as validating reads it: empty is the current branch. */
  const target = $derived(b.target || branches[0] || '');

  const actions = $derived(
    (['merge', 'pr', 'push', 'keep'] as BoardAction[]).map((value) => ({ value, label: t(`boardSettings.tab.actions.${value}.label`) })),
  );
  const strategies = $derived(
    (['merge', 'squash', 'rebase'] as BoardSettings['strategy'][]).map((value) => ({
      value,
      label: t(`boardSettings.tab.strategies.${value}`),
    })),
  );
  const conflicts = $derived(
    (['ask', 'agent', 'abort'] as BoardSettings['conflict'][]).map((value) => ({
      value,
      label: t(`boardSettings.tab.conflicts.${value}`),
    })),
  );
  const PARALLEL = [1, 2, 3, 4, 5, 6].map((n) => ({ value: n, label: String(n) }));
</script>

<Checks {form} step="kanban" />
<div class="box">
  <Row label={t('boardSettings.tab.actionsTitle')}>
    <Chips label={t('boardSettings.tab.actionsTitle')} options={actions} bind:value={form.config.board.action} />
  </Row>
  {#if b.action === 'merge' || b.action === 'pr'}
    <Row label={t('boardSettings.tab.targetBranch')}>
      <div class="branches" role="group" aria-label={t('boardSettings.tab.targetBranch')}>
        {#each branches.length ? branches : [target || t('wizard.currentBranch')] as br (br)}
          <button class="w-chip mono" aria-pressed={target === br} onclick={() => (form.config.board.target = br)}
            ><Icon name="branch" size={12} />{br}</button
          >
        {/each}
        {#if b.target && !branches.includes(b.target)}
          <button class="w-chip mono bad" aria-pressed="true"><Icon name="branch" size={12} />{b.target}</button>
        {/if}
      </div>
    </Row>
    {#if b.action === 'merge'}
      <Row label={t('boardSettings.tab.strategy')}>
        <Chips label={t('boardSettings.tab.strategy')} options={strategies} bind:value={form.config.board.strategy} />
      </Row>
    {:else}
      <Row label={t('boardSettings.tab.draftPr')}>
        <Switch label={t('boardSettings.tab.draftPr')} bind:on={form.config.board.draft} />
      </Row>
    {/if}
  {/if}
  <Row label={t('boardSettings.tab.cleanup')} desc={t('boardSettings.tab.cleanupDesc')}>
    <Switch label={t('boardSettings.tab.cleanup')} bind:on={form.config.board.cleanup} />
  </Row>
  <Row label={t('boardSettings.tab.commitMessage')}>
    <Switch label={t('boardSettings.tab.commitMessage')} bind:on={form.config.board.conventional} />
  </Row>
  <Row label={t('boardSettings.tab.onConflict')}>
    <Chips label={t('boardSettings.tab.onConflict')} options={conflicts} bind:value={form.config.board.conflict} />
  </Row>
  <Row label={t('boardSettings.tab.testCommand')} hint={t('boardSettings.tab.testCommandHint')}>
    <input
      class="field mono input"
      placeholder={t('boardSettings.tab.testCommandPlaceholder')}
      aria-label={t('boardSettings.tab.testCommand')}
      spellcheck="false"
      bind:value={form.config.board.testCommand}
    />
  </Row>
  <Row label={t('boardSettings.tab.testsFirst')} desc={t('boardSettings.tab.testsFirstDesc')}>
    <Switch
      label={t('boardSettings.tab.testsFirst')}
      disabled={!b.testCommand.trim()}
      bind:on={() => b.testsFirst && !!b.testCommand.trim(), (v) => (form.config.board.testsFirst = v)}
    />
  </Row>
  <Row label={t('boardSettings.tab.parallel')} desc={t('boardSettings.tab.parallelDesc')}>
    <Chips label={t('boardSettings.tab.parallel')} options={PARALLEL} mono bind:value={form.config.board.maxParallel} />
  </Row>
  <Row label={t('boardSettings.tab.autoAssign')} desc={t('boardSettings.tab.autoAssignDesc')}>
    <Switch label={t('boardSettings.tab.autoAssign')} bind:on={form.config.board.autopilot} />
  </Row>
  <Row label={t('common.model')}>
    <select class="field select" aria-label={t('common.model')} bind:value={form.config.board.model}>
      <option value="">{t('boardSettings.tab.defaultModel', { model: modelLabel(newAgentDefaults(undefined).model, app.models) })}</option>
      {#each modelOptions(app.models) as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
    </select>
  </Row>
  <Row label={t('boardSettings.tab.effort')}>
    <select class="field select" aria-label={t('boardSettings.tab.effort')} bind:value={form.config.board.effort}>
      <option value="">{t('boardSettings.tab.default')}</option>
      {#each EFFORTS as e (e.value)}<option value={e.value}>{e.label}</option>{/each}
    </select>
  </Row>
  <Row label={t('boardSettings.tab.mode')}>
    <select class="field select" aria-label={t('boardSettings.tab.mode')} bind:value={form.config.board.mode}>
      <option value="">{t('boardSettings.tab.default')}</option>
      {#each MODES as m (m.value)}<option value={m.value}>{m.label}</option>{/each}
    </select>
  </Row>
  <!-- « Pas de workflow pour ce projet » (ESC-309): nothing of its workflows read nor written. -->
  <Row label={t('workflowSettings.enabled')} desc={t('workflowSettings.enabledDesc')}>
    <div class="segmented" role="radiogroup" aria-label={t('workflowSettings.enabled')}>
      {#each [true, false] as on (on)}
        <button
          role="radio"
          aria-checked={form.config.workflowsEnabled === on}
          class:on={form.config.workflowsEnabled === on}
          onclick={() => (form.config.workflowsEnabled = on)}>{t(on ? 'workflowSettings.on' : 'workflowSettings.off')}</button
        >
      {/each}
    </div>
  </Row>
</div>

<style>
  /* The step's settings, a row each between rules (no box: the window is the box). */
  .box {
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .box > :global(.row) {
    padding-inline: 0;
  }
  .branches {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
  .input {
    width: 200px;
    height: 30px;
    font-size: 12.5px;
  }
  .select {
    height: 30px;
    font-size: 12.5px;
  }
</style>

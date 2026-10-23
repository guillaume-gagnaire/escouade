<script lang="ts">
  import { t } from '../../lib/i18n';
  import { worktreesProblems } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { WorktreeStep } from '../../lib/types';
  import { DEFAULT_WORKTREES } from '../../lib/worktrees';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Row from '../settings/Row.svelte';
  import StepList from '../settings/StepList.svelte';
  import Switch from '../settings/Switch.svelte';
  import WorktreeBaseField from '../settings/WorktreeBaseField.svelte';
  import Checks from './Checks.svelte';

  // « Worktrees »: one per agent, their folder and branches, the files copied into them, what prepares and removes
  // them (« Remplir automatiquement »: Claude reads the folder).
  let { form }: { form: WizardForm } = $props();

  const c = $derived(form.config);
  const problems = $derived(worktreesProblems(form.path.trim(), c));
  const DIR_PROBLEMS = {
    itself: 'settings.project.worktreeDirItself',
    contains: 'settings.project.worktreeDirContains',
    git: 'settings.project.worktreeDirGit',
    partial: 'settings.project.worktreeDirPartial',
  } as const;

  const step = (): WorktreeStep => ({ id: crypto.randomUUID(), command: '', shell: app.shells[0]?.id ?? 'pwsh', cwd: '' });

  // The folder typed is checked again after a pause: is it one git tracks?
  let timer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void c.worktreeDir;
    clearTimeout(timer);
    timer = setTimeout(() => void form.check(), 300);
    return () => clearTimeout(timer);
  });
</script>

<Checks {form} step="worktrees" />
<div class="box">
  <Row label={t('settings.project.worktreePerAgent')} desc={t('wizard.worktrees.note')}>
    <Switch label={t('settings.project.worktreePerAgent')} bind:on={form.config.worktreePerAgent} />
  </Row>
  {#if form.config.worktreePerAgent}
    <Row label={t('settings.project.worktreeAtFirstMessage')} desc={t('settings.project.worktreeAtFirstMessageDesc')}>
      <Switch label={t('settings.project.worktreeAtFirstMessage')} bind:on={form.config.worktreeAtFirstMessage} />
    </Row>
    <!-- Its branches searched once the project is added; before, among the folder's. -->
    <Row label={t('settings.project.worktreeBase')} desc={t('settings.project.worktreeBaseDesc')} wide>
      <WorktreeBaseField
        bind:value={form.config.worktreeBase}
        projectId={form.project?.id}
        branches={form.setup?.branches ?? []}
        target={form.config.board.target || form.setup?.branches[0] || 'main'}
      />
    </Row>
  {/if}
  <Row label={t('settings.project.worktreeDir')} desc={t('settings.project.worktreeDirDesc')} wide>
    <input
      class="field mono input"
      spellcheck="false"
      placeholder={DEFAULT_WORKTREES}
      aria-label={t('settings.project.worktreeDir')}
      aria-invalid={!!problems.dir}
      bind:value={form.config.worktreeDir}
    />
    {#if problems.dir}<p class="error">{t(DIR_PROBLEMS[problems.dir])}</p>{/if}
  </Row>
  <Row label={t('settings.project.agentPrefix')} wide>
    <input
      class="field mono input"
      spellcheck="false"
      aria-label={t('settings.project.agentPrefix')}
      aria-invalid={problems.agentPrefix}
      bind:value={form.config.agentBranchPrefix}
    />
  </Row>
  <Row label={t('settings.project.ticketPrefix')} wide>
    <input
      class="field mono input"
      spellcheck="false"
      aria-label={t('settings.project.ticketPrefix')}
      aria-invalid={problems.ticketPrefix}
      bind:value={form.config.ticketBranchPrefix}
    />
  </Row>
  <Row label={t('settings.project.copy')} desc={t('settings.project.copyDesc')} wide>
    <textarea class="field mono copy" rows="2" placeholder=".env*" aria-label={t('settings.project.copy')} bind:value={form.config.copy}
    ></textarea>
  </Row>
  <Row label={t('settings.project.fill')} desc={t('settings.project.fillWorktreesDesc')}>
    <button class="btn" disabled={!!form.filling} onclick={() => form.fill('worktrees')}>
      {form.filling === 'worktrees' ? t('wizard.buttons.filling') : t('wizard.buttons.fill')}
    </button>
  </Row>
  <Row label={t('settings.project.setupTitle')} desc={t('settings.project.setupDesc')} wide>
    <StepList bind:steps={form.config.worktreeSetup} kind="setup" onadd={() => form.config.worktreeSetup.push(step())} />
  </Row>
  <Row label={t('settings.project.teardownTitle')} desc={t('settings.project.teardownDesc')} wide>
    <StepList bind:steps={form.config.worktreeTeardown} kind="teardown" onadd={() => form.config.worktreeTeardown.push(step())} />
  </Row>
</div>

<style>
  .box {
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--bg);
  }
  .input {
    width: 100%;
    height: 32px;
    font-size: 12.5px;
    background: var(--panel);
  }
  .input[aria-invalid='true'] {
    border-color: var(--del);
  }
  .copy {
    width: 100%;
    height: auto;
    padding: 8px 10px;
    font-size: 12.5px;
    resize: vertical;
    background: var(--panel);
  }
  .error {
    margin: 0;
    font-size: 11.5px;
    color: var(--del);
  }
</style>

<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { joinPath, tildify } from '../../lib/format';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm, worktreesProblems } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { repoLabel } from '../../lib/repos';
  import type { CommitMode, LeftoverReport, Project, WorktreeStep } from '../../lib/types';
  import { DEFAULT_WORKTREES, worktreesRoot, type WorktreesProblem } from '../../lib/worktrees';
  import Chips from './Chips.svelte';
  import DocsBranchField from './DocsBranchField.svelte';
  import Facts from './Facts.svelte';
  import Group from './Group.svelte';
  import Proposal, { ask, settle } from './Proposal.svelte';
  import Row from './Row.svelte';
  import SharedMark from './SharedMark.svelte';
  import StepList from './StepList.svelte';
  import Switch from './Switch.svelte';
  import WorktreeBaseField from './WorktreeBaseField.svelte';

  // Projets › <projet> › Git et worktrees: a worktree per agent and where it goes, its branches, the files copied into
  // it, who commits, the branch of the docs; what prepares and takes down a worktree; the worktrees left; its
  // repositories (a project of several).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);

  /** Its repositories as the backend last found them (ESC-169), whatever the draft says. */
  const live = $derived(app.projects.find((p) => p.id === project.id) ?? project);
  const liveRepos = $derived(live.repos ?? []);
  const goneRepos = $derived(live.reposGone ?? []);
  let rescanning = $state(false);
  async function rescan() {
    rescanning = true;
    await app.run(api.refreshRepos(project.id));
    rescanning = false;
  }

  /** « Nettoyer »: the worktrees its archived agents left, while it runs, then what it did (ESC-143). */
  let cleaning = $state(false);
  let cleaned = $state<{ projectId: string; report: LeftoverReport } | null>(null);
  async function cleanLeftovers() {
    const projectId = project.id;
    cleaning = true;
    const report = await app.run(api.cleanWorktrees(projectId));
    cleaning = false;
    if (report) cleaned = { projectId, report };
  }
  const report = $derived(cleaned?.projectId === project.id ? cleaned.report : null);

  /** What the field says of a « Dossier des worktrees » that cannot be. */
  const DIR_PROBLEMS = {
    itself: 'settings.project.worktreeDirItself',
    contains: 'settings.project.worktreeDirContains',
    git: 'settings.project.worktreeDirGit',
    partial: 'settings.project.worktreeDirPartial',
  } as const satisfies Record<WorktreesProblem, string>;
  /** What is wrong with where its worktrees go and how their branches are named, as typed. */
  const problems = $derived(worktreesProblems(project.path, draft));
  /** Where its next worktree would go, as typed. */
  const nextWorktree = $derived(tildify(joinPath(worktreesRoot(project.path, draft.worktreeDir), t('settings.project.nextName'))));

  /** Who writes the commits of the files panel's « Commit… » and « Commit tout… ». */
  const COMMIT_MODES = $derived<{ value: CommitMode; label: string }[]>(
    (['agent', 'direct'] as const).map((value) => ({ value, label: t(`settings.project.commitModes.${value}`) })),
  );

  // What Claude proposed, read here in full before it replaces anything of the draft.
  const proposal = $derived(settingsForm.proposal[project.id]);
  let suggestButton = $state<HTMLButtonElement>();
</script>

{#snippet stepsOf(title: string, steps: WorktreeStep[])}
  <section class="part">
    <h4 class="section-label">{title}</h4>
    {#if steps.length}
      <ol class="steps">
        {#each steps as s (s.id)}
          <li>
            <pre class="command mono">{revealHidden(s.command)}</pre>
            <Facts shell={s.shell} dir={s.cwd.trim() && s.cwd.trim() !== '.' ? revealHidden(s.cwd) : t('settings.project.worktreeRoot')} />
          </li>
        {/each}
      </ol>
    {:else}
      <p class="none">{t('settings.project.noCommands')}</p>
    {/if}
  </section>
{/snippet}

{#snippet prefixRow(kind: 'agent' | 'ticket', value: string, invalid: boolean, set: (v: string) => void)}
  {@const label = t(`settings.project.${kind}Prefix`)}
  <Row
    {label}
    desc={t(`settings.project.${kind}PrefixDesc`, { prefix: value.trim() })}
    descId="{kind}-prefix-desc"
    wide
    shared="git.{kind}_branch_prefix"
  >
    <input
      class="field input mono setting"
      spellcheck="false"
      aria-label={label}
      aria-describedby="{kind}-prefix-desc{invalid ? ` ${kind}-prefix-error` : ''}"
      aria-invalid={invalid}
      {value}
      oninput={(e) => set(e.currentTarget.value)}
    />
    {#if invalid}<p class="error" id="{kind}-prefix-error">{t('settings.project.prefixInvalid', { at: '@{' })}</p>{/if}
  </Row>
{/snippet}

<Group title="Git" anchor="git">
  <Row label={t('settings.project.worktreePerAgent')} desc={t('settings.project.worktreePerAgentDesc')} shared="worktrees.per_agent">
    <Switch label={t('settings.project.worktreePerAgent')} bind:on={draft.worktreePerAgent} />
  </Row>
  <Row
    label={t('settings.project.worktreeAtFirstMessage')}
    desc={t('settings.project.worktreeAtFirstMessageDesc')}
    shared="worktrees.at_first_message"
  >
    <Switch label={t('settings.project.worktreeAtFirstMessage')} bind:on={draft.worktreeAtFirstMessage} />
  </Row>
  <Row label={t('settings.project.worktreeBase')} desc={t('settings.project.worktreeBaseDesc')} wide shared="worktrees.base">
    <WorktreeBaseField
      bind:value={draft.worktreeBase}
      projectId={project.id}
      target={draft.board.target || app.git[project.id]?.branch || 'main'}
    />
  </Row>
  <Row
    label={t('settings.project.worktreeDir')}
    desc={t('settings.project.worktreeDirDesc')}
    descId="worktree-dir-desc"
    wide
    shared="worktrees.dir"
  >
    <input
      class="field input mono setting"
      spellcheck="false"
      placeholder={DEFAULT_WORKTREES}
      aria-label={t('settings.project.worktreeDir')}
      aria-describedby="worktree-dir-desc worktree-dir-at"
      aria-invalid={!!problems.dir}
      bind:value={draft.worktreeDir}
    />
    {#if problems.dir}
      <p class="error" id="worktree-dir-at">
        {t(DIR_PROBLEMS[problems.dir])}
      </p>
    {:else}
      <p class="at mono" id="worktree-dir-at">{t('settings.project.worktreeDirAt', { path: nextWorktree })}</p>
    {/if}
  </Row>
  {@render prefixRow('agent', draft.agentBranchPrefix, problems.agentPrefix, (v) => (draft.agentBranchPrefix = v))}
  {@render prefixRow('ticket', draft.ticketBranchPrefix, problems.ticketPrefix, (v) => (draft.ticketBranchPrefix = v))}
  <Row label={t('settings.project.copy')} desc={t('settings.project.copyDesc')} descId="copy-desc" wide shared="worktrees.copy">
    <textarea
      class="field mono copy"
      rows="3"
      placeholder=".env*"
      aria-label={t('settings.project.copy')}
      aria-describedby="copy-desc"
      bind:value={draft.copy}
    ></textarea>
  </Row>
  <Row label={t('settings.project.commit')} desc={t('settings.project.commitDesc')} shared="git.commit_mode">
    <Chips label={t('settings.project.commit')} options={COMMIT_MODES} bind:value={draft.commitMode} />
  </Row>
  <Row label={t('docs.branch.label')} desc={t('docs.branch.desc')} wide>
    <DocsBranchField bind:value={draft.docsBranch} />
  </Row>
</Group>

<Group title={t('settings.project.worktrees')} anchor="worktrees" note={t('settings.project.worktreesNote')}>
  <div class="sub-h">
    <b>{t('settings.project.setupTitle')}</b><span>{t('settings.project.setupDesc')}</span><SharedMark keys="worktrees.setup" />
  </div>
  <StepList bind:steps={draft.worktreeSetup} kind="setup" onadd={() => settingsForm.addStep('setup')} />
  <div class="sub-h">
    <b>{t('settings.project.teardownTitle')}</b><span>{t('settings.project.teardownDesc')}</span><SharedMark keys="worktrees.teardown" />
  </div>
  <StepList bind:steps={draft.worktreeTeardown} kind="teardown" onadd={() => settingsForm.addStep('teardown')} />
  <Row label={t('settings.project.fill')} desc={t('settings.project.fillWorktreesDesc')} descId="worktrees-suggest-desc">
    <!-- Two buttons of the project's pages read the same: what each row says of itself tells them apart. -->
    <button
      class="btn suggest"
      aria-describedby="worktrees-suggest-desc"
      disabled={settingsForm.suggesting[project.id]}
      bind:this={suggestButton}
      onclick={() => ask(() => settingsForm.suggest(project.id), suggestButton, 'worktrees-proposal')}
    >
      <Icon name="sparkle" size={14} />{settingsForm.suggesting[project.id]
        ? t('settings.project.filling')
        : t('settings.project.fillAuto')}
    </button>
  </Row>
  {#if proposal}
    <Proposal
      id="worktrees-proposal"
      label={t('settings.project.proposedWorktrees')}
      notice={t('settings.project.proposedWorktreesNote')}
      take={() => settle(() => settingsForm.takeProposal(project.id), suggestButton)}
      ignore={() => settle(() => delete settingsForm.proposal[project.id], suggestButton)}
    >
      {@render stepsOf(t('settings.project.setupTitle'), proposal.setup)}
      {@render stepsOf(t('settings.project.teardownTitle'), proposal.teardown)}
    </Proposal>
  {/if}
  <Row label={t('settings.project.leftovers')} desc={t('settings.project.leftoversDesc')} descId="worktrees-leftovers-desc">
    <button class="btn" aria-describedby="worktrees-leftovers-desc" disabled={cleaning} onclick={cleanLeftovers}>
      {cleaning ? t('settings.project.leftoversRunning') : t('settings.project.leftoversButton')}
    </button>
  </Row>
  {#if report}
    <div class="leftovers" role="status">
      <p>
        {report.removed
          ? t('settings.project.leftoversRemoved', { count: report.removed })
          : report.kept.length
            ? ''
            : t('settings.project.leftoversNone')}
        {#if report.kept.length}{t('settings.project.leftoversKept', { count: report.kept.length })}{/if}
      </p>
      {#if report.kept.length}
        <ul>
          {#each report.kept as k (k.agentId)}
            <li><span class="name mono">{k.name}</span> · {k.reason}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</Group>

{#if liveRepos.length || goneRepos.length}
  <!-- A project of several repositories (ESC-169): those found, those gone, where its .escouade.yml is. -->
  <Group title={t('repos.settings.title')} anchor="repos">
    <Row label={t('repos.settings.count', { count: liveRepos.length })} desc={t('repos.settings.hint')}>
      <button class="btn" disabled={rescanning} onclick={rescan}><Icon name="restart" size={14} />{t('repos.settings.rescan')}</button>
    </Row>
    <ul class="repo-list">
      {#each liveRepos.filter((r) => !goneRepos.includes(r)) as r (r)}
        {@const g = app.git[project.id]?.repos?.find((x) => x.repo === r)}
        <li class="repo-row">
          <Icon name="branch" size={14} /><span class="rn">{repoLabel(r)}</span><span class="rb">{g?.branch ?? ''}</span>
        </li>
      {/each}
      {#each goneRepos as r (r)}
        <li class="repo-row gone">
          <Icon name="shield" size={14} /><span class="rn">{repoLabel(r)}</span><span class="rb">{t('repos.settings.gone')}</span>
          <button class="link" onclick={() => app.run(api.forgetRepo(project.id, r))}
            >{t('repos.settings.forget', { repo: repoLabel(r) })}</button
          >
          <span class="why">{t('repos.settings.goneHint')}</span>
        </li>
      {/each}
    </ul>
    {#if !liveRepos.includes('')}<p class="repo-note">{t('repos.settings.yml')}</p>{/if}
  </Group>
{/if}

<style>
  .input {
    width: 220px;
    max-width: 100%;
  }
  .setting {
    width: 320px;
  }
  .setting[aria-invalid='true'] {
    box-shadow: inset 0 0 0 1px var(--del);
  }
  .at,
  .error {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .at {
    color: var(--muted);
  }
  .error {
    color: var(--del);
  }
  /* A title inside a group, its explanation beside it. */
  .sub-h {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    margin-top: 18px;
  }
  .sub-h b {
    font-weight: 600;
  }
  .sub-h span {
    color: var(--muted);
    font-size: 12px;
  }
  .copy {
    width: 320px;
    max-width: 100%;
    resize: vertical;
    font-size: 12px;
  }
  .suggest {
    flex-shrink: 0;
  }
  .suggest :global(.icon) {
    color: var(--p-ink);
  }
  .leftovers {
    margin: -4px 0 12px;
    padding: 9px 12px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.03);
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .leftovers p,
  .leftovers ul {
    margin: 0;
  }
  .leftovers ul {
    padding-left: 18px;
    color: var(--muted);
  }
  /* The repositories of a project of several (ESC-169), one line each. */
  .repo-list {
    list-style: none;
    margin: 4px 0 10px;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .repo-row {
    display: grid;
    grid-template-columns: 16px auto minmax(0, 1fr) auto;
    align-items: center;
    gap: 4px 10px;
    padding: 8px 12px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 12.5px;
  }
  .repo-row .rn {
    color: var(--text);
    font: 600 12px var(--mono);
  }
  .repo-row .rb {
    min-width: 0;
    overflow: hidden;
    font: 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .repo-row.gone {
    background: oklch(from var(--del) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .repo-row.gone :global(svg),
  .repo-row.gone .rb {
    color: var(--del);
  }
  .repo-row .link {
    color: var(--p-ink);
    font-size: 12px;
  }
  .repo-row .why {
    grid-column: 2 / -1;
    color: var(--soft);
    font-size: 12px;
  }
  .repo-note {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
</style>

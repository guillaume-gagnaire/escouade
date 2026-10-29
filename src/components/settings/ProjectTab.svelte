<script lang="ts">
  import Icon from '../Icon.svelte';
  import { openPath } from '../../lib/opener';
  import { tick, type Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { anyCan, choosesEngine, engineName, engineOptions } from '../../lib/engines';
  import { accountName, accountsFor, accountsOf, dedicated, defaultEngine, isPrincipal, serves, sharedBy } from '../../lib/accounts';
  import { api } from '../../lib/ipc';
  import { fList } from '../../lib/format';
  import { askCloseProject } from '../../lib/project-actions';
  import { joinPath, tildify } from '../../lib/format';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm, worktreesProblems } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { projectPalette } from '../../lib/plugin-theme.svelte';
  import { cleanLabel, cleanLetters, initials, tileText } from '../../lib/project-rail';
  import TileFace from '../TileFace.svelte';
  import type { Account, CommitMode, LeftoverReport, Project, WorktreeStep } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import SharedMark from './SharedMark.svelte';
  import SharedSection from './SharedSection.svelte';
  import CodeIndexSection from './CodeIndexSection.svelte';
  import MemorySection from './MemorySection.svelte';
  import StepList from './StepList.svelte';
  import Switch from './Switch.svelte';
  import WorkflowSection from './WorkflowSection.svelte';
  import WorktreeBaseField from './WorktreeBaseField.svelte';
  import DocsBranchField from './DocsBranchField.svelte';
  import { tip } from '../../lib/tip';
  import { repoLabel } from '../../lib/repos';
  import { SHARED_DIR } from '../../lib/sharing';
  import { gitNameOf } from '../../lib/share-actions';
  import { DEFAULT_WORKTREES, worktreesRoot, type WorktreesProblem } from '../../lib/worktrees';

  // A project's own settings: who it is, its worktrees and what runs in them, its launch commands (its draft: the
  // modal's project).
  let { project }: { project: Project } = $props();
  // The name git knows in its folder (ESC-243): what « Ton nom dans ce projet » falls back on after the app's.
  $effect(() => void gitNameOf(project.id));

  const draft = $derived(settingsForm.project!);
  /** Its shared tickets (ESC-190): all of them, and those of this machine's user. */
  const sharedCount = $derived.by(() => {
    const list = Object.values(app.tickets).filter((x) => x.projectId === settingsForm.projectId && x.shared && !x.shared.gone);
    return { all: list.length, mine: list.filter((x) => x.shared?.mine).length };
  });
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
  /**
   * « Compte préféré »: « Automatique », then each active account of its default harness (the project's own, even switched
   * off since).
   */
  const ACCOUNTS = $derived([
    { value: '', label: t('accounts.project.auto'), title: t('accounts.project.autoTitle') },
    ...accountsOf(draft.engine ?? undefined)
      .filter((a) => (a.active && serves(a, project.id)) || a.id === draft.account)
      .map((a) => ({ value: a.id, label: a.active ? accountName(a) : t('accounts.project.inactive', { name: accountName(a) }) })),
  ]);
  /**
   * A private project's accounts (ESC-141), as drafted: those of its default harness reserved to it alone, and those its
   * agents may still go to that it shares with other projects (sessions, memory and history common there).
   */
  const privateEngine = $derived(draft.engine ?? defaultEngine());
  const asDrafted = $derived({ id: project.id, account: draft.account, private: draft.private, sharedOnLimit: draft.sharedOnLimit });
  const ownAccounts = $derived(accountsFor(asDrafted, privateEngine).filter((a) => dedicated(a, project.id)));
  const sharedAccounts = $derived(sharedBy(asDrafted, privateEngine));
  const quotedNames = (list: Account[]) => fList(list.map((a) => t('accounts.tab.quoted', { name: accountName(a) })));

  /** « Réserver <compte> à ce projet »: saved at once, as Réglages › Comptes saves its accounts. */
  async function reserveHere(a: Account) {
    const list = await app.run(api.accountUpdate({ ...$state.snapshot(a), projects: [project.id] }));
    if (list) app.settings.accounts = list;
  }

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

  /** The setting for all agents, as drafted in this modal (not as saved), is on: it covers this project’s agents already. */
  const coveredByAll = $derived(!!settingsForm.settings.remoteControlAll);

  /** Who writes the commits of the files panel's « Commit… » and « Commit tout… ». */
  const COMMIT_MODES = $derived<{ value: CommitMode; label: string }[]>(
    (['agent', 'direct'] as const).map((value) => ({ value, label: t(`settings.project.commitModes.${value}`) })),
  );

  /** What keeps a private project's files from the other projects' agents, harness by harness (ESC-140). */
  const FENCE = [
    { id: 'claude', name: 'Claude Code', kind: 'guard' },
    { id: 'codex', name: 'Codex', kind: 'barrier' },
    { id: 'opencode', name: 'OpenCode', kind: 'guard' },
    { id: 'copilot', name: 'Copilot CLI', kind: 'guard' },
    { id: 'cursor', name: 'Cursor CLI', kind: 'guard' },
    { id: 'vibe', name: 'Mistral Vibe', kind: 'guard' },
    { id: 'agy', name: 'Antigravity CLI', kind: 'guard' },
    { id: 'gemini', name: 'Gemini CLI', kind: 'guard' },
  ] as const;

  /** Asks before closing it, then comes back to these settings, their draft as it was. */
  function askClose() {
    // Taken now: the tab is gone while the confirmation is open.
    const projectId = project.id;
    askCloseProject(project, () => {
      app.modal = { kind: 'settings', tab: 'projects', projectId, resume: true };
    });
  }

  // What Claude proposed, read here in full before it replaces anything of the draft.
  const proposal = $derived(settingsForm.proposal[project.id]);
  const launchProposal = $derived(settingsForm.launchProposal[project.id]);
  let suggestButton = $state<HTMLButtonElement>();
  let launchButton = $state<HTMLButtonElement>();

  /** A shell by its name, or its id when this machine has none such. */
  const shellName = (id: string) =>
    app.shells.find((s) => s.id === id)?.label ?? t('settings.fields.shellNotFound', { name: revealHidden(id) });

  /**
   * Asks Claude with `button`, then hands its proposal the focus, to be read from the keyboard: only when the focus is
   * still where the button left it (the reading takes minutes, the user may have gone on elsewhere).
   */
  async function ask(read: () => Promise<void>, button: HTMLButtonElement | undefined, proposalId: string) {
    await read();
    await tick();
    const at = document.activeElement;
    if (!at || at === document.body || at === button) document.getElementById(proposalId)?.focus();
  }

  /** Takes or ignores a proposal, the focus back on the button that asked for it. */
  async function settle(run: () => void, button: HTMLButtonElement | undefined) {
    run();
    await tick();
    button?.focus();
  }
</script>

{#snippet facts(shell: string, dir: string)}
  <dl class="facts">
    <dt>{t('settings.fields.shell')}</dt>
    <dd>{shellName(shell)}</dd>
    <dt>{t('common.folder')}</dt>
    <dd class="mono">{dir}</dd>
  </dl>
{/snippet}

{#snippet proposed(id: string, label: string, notice: string, list: Snippet, take: () => void, ignore: () => void)}
  <div {id} class="proposal" role="region" aria-label={label} tabindex="-1">
    <p class="notice">{notice}</p>
    {@render list()}
    <div class="actions">
      <button class="btn ghost" onclick={ignore}>{t('settings.project.ignore')}</button>
      <button class="btn primary" onclick={take}>{t('settings.project.replace')}</button>
    </div>
  </div>
{/snippet}

{#snippet stepsOf(title: string, steps: WorktreeStep[])}
  <section class="part">
    <h4 class="section-label">{title}</h4>
    {#if steps.length}
      <ol class="steps">
        {#each steps as s (s.id)}
          <li>
            <pre class="command mono">{revealHidden(s.command)}</pre>
            {@render facts(s.shell, s.cwd.trim() && s.cwd.trim() !== '.' ? revealHidden(s.cwd) : t('settings.project.worktreeRoot'))}
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

{#snippet proposedSteps()}
  {@render stepsOf(t('settings.project.setupTitle'), proposal!.setup)}
  {@render stepsOf(t('settings.project.teardownTitle'), proposal!.teardown)}
{/snippet}

{#snippet proposedLaunch()}
  <ul class="steps">
    {#each launchProposal! as c (c.id)}
      <li>
        <span class="name">{revealHidden(c.name.trim())}</span>
        <pre class="command mono">{revealHidden(c.command)}</pre>
        {@render facts(c.shell, c.cwd.trim() ? revealHidden(c.cwd) : t('settings.project.projectFolder'))}
      </li>
    {/each}
  </ul>
{/snippet}

<Group title={t('settings.project.identity')}>
  <Row label={t('settings.project.nameLabel')}>
    <input class="field input" aria-label={t('settings.project.nameLabel')} spellcheck="false" bind:value={draft.name} />
  </Row>
  <!-- From two harnesses found on: the one of its new agents and tickets (none chosen: Claude Code). -->
  {#if choosesEngine()}
    <Row label={t('settings.project.engine')} desc={t('settings.project.engineDesc')} shared="agents.engine">
      <Chips
        label={t('settings.project.engine')}
        options={engineOptions(draft.engine)}
        bind:value={() => draft.engine ?? 'claude', (v) => (draft.engine = v)}
      />
    </Row>
  {/if}
  {#if accountsOf(draft.engine ?? undefined).length > 1}
    <Row label={t('accounts.project.label')} desc={t('accounts.project.desc')}>
      <Chips label={t('accounts.project.label')} options={ACCOUNTS} bind:value={draft.account} />
    </Row>
  {/if}
  <Row label={t('wizard.reopen.label')} desc={t('wizard.reopen.desc')}>
    <!-- The assistant takes the place of these settings: it saves what it sets itself. -->
    <button class="btn" onclick={() => (app.modal = { kind: 'newProject', projectId: project.id })}>{t('wizard.reopen.button')}</button>
  </Row>
  <Row label={t('common.folder')}>
    {#snippet extra()}
      <span class="more path mono" use:tip={{ text: project.path, cut: true }}>{project.path}</span>
    {/snippet}
    <button class="btn" onclick={() => openPath(project.path).catch((err) => app.toast(String(err), 'error'))}
      ><Icon name="folder" size={14} />{t('common.open')}</button
    >
  </Row>
  <Row label={t('common.color')} desc={t('settings.project.colorDesc')}>
    <div class="swatches" role="group" aria-label={t('common.color')}>
      {#each projectPalette(draft.color) as c, i (c)}
        <button
          class="swatch"
          aria-label={t('settings.project.colorN', { n: i + 1 })}
          aria-pressed={draft.color === c}
          style:--c={c}
          onclick={() => (draft.color = c)}
        ></button>
      {/each}
    </div>
  </Row>
  <!-- The project's tile (ESC-167): as the rail will show it, its letters or its logo. -->
  <Row label={t('settings.project.lettersLabel')} desc={t('settings.project.lettersDesc')}>
    <div class="tile-set">
      <span class="tile-prev" style:--c={draft.color} role="img" aria-label={t('settings.project.tilePreview')}
        ><TileFace
          text={tileText({ name: draft.name.trim() || project.name, tileLetters: draft.tileLetters })}
          logo={settingsForm.logoOf(project.id)}
        /></span
      >
      <input
        class="field letters"
        aria-label={t('settings.project.lettersLabel')}
        spellcheck="false"
        autocomplete="off"
        placeholder={initials(draft.name.trim() || project.name)}
        value={draft.tileLetters}
        oninput={(e) => {
          const v = cleanLetters(e.currentTarget.value);
          e.currentTarget.value = v;
          draft.tileLetters = v;
        }}
      />
    </div>
  </Row>
  <Row label={t('settings.project.logoLabel')} desc={t('settings.project.logoDesc')}>
    <div class="tile-set">
      <button class="btn" onclick={() => settingsForm.chooseLogo(project.id)}
        ><Icon name="image" size={14} />{t('settings.project.logoChoose')}</button
      >
      {#if draft.logo}<button class="btn" onclick={() => (draft.logo = '')}>{t('settings.project.logoRemove')}</button>{/if}
    </div>
  </Row>
  <!-- Its label (ESC-168): under the tile when on; its text, beside the tile on a wide rail, on or not. -->
  <Row label={t('settings.project.labelLabel')} desc={t('settings.project.labelDesc')}>
    <div class="tile-set">
      <input
        class="field label-text"
        aria-label={t('settings.project.labelText')}
        spellcheck="false"
        autocomplete="off"
        placeholder={draft.name.trim() || project.name}
        value={draft.tileLabelText}
        oninput={(e) => {
          const v = cleanLabel(e.currentTarget.value);
          if (v !== e.currentTarget.value) e.currentTarget.value = v;
          draft.tileLabelText = v;
        }}
      />
      <Switch label={t('settings.project.labelLabel')} bind:on={draft.tileLabel} />
    </div>
  </Row>
</Group>

<!-- Only while a harness has Remote Control. -->
{#if anyCan('remoteControl')}
  <Group title={t('settings.project.remoteTitle')}>
    <Row label={t('settings.project.remote')} desc={t('settings.project.remoteDesc')}>
      {#snippet extra()}
        {#if coveredByAll}<span class="more" id="remote-covered">{t('settings.project.remoteGlobal')}</span>{/if}
      {/snippet}
      <!-- The setting for all agents covers every project: the switch shows it, the project’s own value stays as it is. -->
      <Switch
        label={t('settings.project.remote')}
        disabled={coveredByAll}
        describedby={coveredByAll ? 'remote-covered' : undefined}
        bind:on={() => coveredByAll || draft.remoteControl, (v) => (draft.remoteControl = v)}
      />
    </Row>
  </Group>
{/if}

<!-- « Projet privé »: what the agents of the other projects see of it through the MCP server; for a private one only
     (choice 28 of the 2.0 design), what keeps its files from them harness by harness, then its accounts. -->
<Group title={t('settings.project.privateTitle')} anchor="private">
  <Row label={t('settings.project.private')} desc={t('settings.project.privateDesc')} shared="private">
    <Switch label={t('settings.project.private')} bind:on={draft.private} />
  </Row>
  {#if draft.private}
    {@const harness = engineName(privateEngine)}
    <div class="sub-h"><b>{t('settings.project.fenceTitle')}</b><span>{t('settings.project.fenceDesc')}</span></div>
    <ul class="fence" aria-label={t('settings.project.fenceTitle')}>
      {#each FENCE as f (f.id)}
        <li>
          <span class="engine">{f.name}</span>
          <span class="kind smark" class:barrier={f.kind === 'barrier'}
            ><Icon name={f.kind === 'barrier' ? 'lock' : 'shield'} size={12} /><span>{t(`settings.project.fenceKind.${f.kind}`)}</span
            ></span
          >
          <span class="how">{t(`settings.project.fenceHow.${f.id}`)}</span>
        </li>
      {/each}
    </ul>
    <p class="note">{t('settings.project.fenceNote')}</p>
    <!-- Its accounts (ESC-141): reserved to it alone, or still shared, with what to do about it. -->
    <div class="sub-h" data-anchor="private-accounts"><b>{t('accounts.tab.list')}</b></div>
    <div class="private-accounts">
      {#if ownAccounts.length}
        <p class="acct-note" role="status">
          <Icon name="check-c" size={14} /><span class="grow"
            >{t('accounts.private.own', { count: ownAccounts.length, names: quotedNames(ownAccounts), harness })}
            {#if ownAccounts.some(isPrincipal)}{t('accounts.private.machineToo', { harness })}{/if}</span
          >
        </p>
      {/if}
      {#if sharedAccounts.length || !ownAccounts.length}
        <div class="acct-note warn">
          <Icon name="shield" size={14} />
          <span class="grow" role="status"
            >{#if sharedAccounts.length}{t('accounts.private.shared', {
                count: sharedAccounts.length,
                names: quotedNames(sharedAccounts),
                harness,
              })}{/if}</span
          >
          {#each sharedAccounts as a (a.id)}
            <button class="btn" use:tip={t('accounts.private.reserveTitle')} onclick={() => reserveHere(a)}>
              {t('accounts.private.reserve', { name: accountName(a) })}
            </button>
          {/each}
          <button class="btn" onclick={() => (app.modal = { kind: 'account', engine: privateEngine, reserveFor: project.id })}>
            {t('accounts.private.add')}
          </button>
        </div>
      {/if}
    </div>
    <Row label={t('accounts.private.sharedOnLimit')} desc={t('accounts.private.sharedOnLimitDesc')}>
      <Switch label={t('accounts.private.sharedOnLimit')} bind:on={draft.sharedOnLimit} />
    </Row>
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

<!-- Its workflows (ESC-195, #loops-settings): what was read, and the steps as corrected. -->
<WorkflowSection {project} />

<Group title="Git">
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
    <!-- Two buttons of this tab read the same: what each row says of itself tells them apart. -->
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
    {@render proposed(
      'worktrees-proposal',
      t('settings.project.proposedWorktrees'),
      t('settings.project.proposedWorktreesNote'),
      proposedSteps,
      () => settle(() => settingsForm.takeProposal(project.id), suggestButton),
      () => settle(() => delete settingsForm.proposal[project.id], suggestButton),
    )}
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

<Group title={t('settings.project.launch')} anchor="launch" note={t('settings.project.launchNote')}>
  <SharedMark keys="launch" />
  <!-- The launch commands on a grid (2.0 design): name, command, folder, shell. -->
  <div class="cmds">
    {#if draft.runCommands.length}
      <div class="cmd-row cmd-head" aria-hidden="true">
        <span>{t('common.name')}</span><span>{t('common.command')}</span><span>{t('settings.fields.subfolder')}</span><span
          >{t('settings.fields.shell')}</span
        ><span></span>
      </div>
    {/if}
    {#each draft.runCommands as c, i (c.id)}
      <fieldset class="cmd-row">
        <legend class="sr">{t('settings.project.commandN', { n: i + 1 })}</legend>
        <input class="field" aria-label={t('common.name')} bind:value={c.name} placeholder={t('settings.project.namePlaceholder')} />
        <input
          class="field mono"
          aria-label={t('common.command')}
          bind:value={c.command}
          placeholder={t('settings.project.commandPlaceholder')}
        />
        <input
          class="field mono"
          aria-label="{t('settings.fields.subfolder')} {t('settings.project.subfolderHint')}"
          use:tip={t('settings.project.subfolderHint')}
          bind:value={c.cwd}
          placeholder={t('settings.project.subfolderPlaceholder')}
        />
        <select class="field" aria-label={t('settings.fields.shell')} bind:value={c.shell}>
          {#each app.shells as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
          {#if !app.shells.some((s) => s.id === c.shell)}<option value={c.shell}
              >{t('settings.fields.shellNotFound', { name: c.shell })}</option
            >{/if}
        </select>
        <button
          class="icon-btn del"
          use:tip={t('common.delete')}
          aria-label={t('common.delete')}
          onclick={() => draft.runCommands.splice(i, 1)}><Icon name="trash" size={14} /></button
        >
      </fieldset>
    {:else}
      <p class="empty">{t('settings.project.noCommandsYet')}</p>
    {/each}
  </div>
  <div class="launch-acts">
    <button class="add-line" onclick={() => settingsForm.addCommand()}
      ><Icon name="plus" size={14} />{t('settings.fields.addLaunch')}</button
    >
    <button
      class="btn suggest"
      aria-describedby="launch-suggest-desc"
      use:tip={t('settings.project.fillLaunchDesc')}
      disabled={settingsForm.suggestingLaunch[project.id]}
      bind:this={launchButton}
      onclick={() => ask(() => settingsForm.suggestLaunch(project.id), launchButton, 'launch-proposal')}
    >
      <Icon name="sparkle" size={14} />{settingsForm.suggestingLaunch[project.id]
        ? t('settings.project.filling')
        : t('settings.project.fillAuto')}
    </button>
    <span id="launch-suggest-desc" hidden>{t('settings.project.fillLaunchDesc')}</span>
  </div>
  {#if launchProposal}
    {@render proposed(
      'launch-proposal',
      t('settings.project.proposedLaunch'),
      t('settings.project.proposedLaunchNote'),
      proposedLaunch,
      () => settle(() => settingsForm.takeLaunchProposal(project.id), launchButton),
      () => settle(() => delete settingsForm.launchProposal[project.id], launchButton),
    )}
  {/if}
</Group>

<!-- Its .escouade.yml after what it sets (2.0 design, #sg-yml): its commands that wait are also said on the Kanban. -->
<SharedSection {project} />

<CodeIndexSection {project} />
<MemorySection {project} />

<!-- The shared Kanban (ESC-190, choice 113): the name in this project, its shared tickets, « Partager chaque nouveau ticket ». -->
<Group title={t('share.settings.projectGroup')} anchor="share">
  <Row
    label={t('share.settings.projectName')}
    desc={settingsForm.ownName || !app.gitNames[project.id]
      ? t('share.settings.projectNameDesc', { name: settingsForm.ownName || t('share.settings.noName') })
      : t('share.settings.projectNameGit', { name: app.gitNames[project.id] })}
  >
    <input
      class="field input"
      aria-label={t('share.settings.projectName')}
      spellcheck="false"
      placeholder={settingsForm.ownName || app.gitNames[project.id] || ''}
      bind:value={draft.userName}
    />
  </Row>
  <Row
    label={t('share.settings.files')}
    desc={sharedCount.all
      ? t('share.settings.filesDesc', { count: sharedCount.all, dir: SHARED_DIR, mine: sharedCount.mine })
      : t('share.settings.filesNone', { dir: SHARED_DIR })}
  />
  <Row label={t('share.settings.shareNew')} desc={t('share.settings.shareNewDesc')}>
    <Switch label={t('share.settings.shareNew')} bind:on={draft.shareNew} />
  </Row>
</Group>

<Group title={t('settings.project.danger')}>
  <Row label={t('settings.project.close')} desc={t('settings.project.closeDesc')}>
    <button class="btn danger" onclick={askClose}>{t('settings.project.closeButton')}</button>
  </Row>
</Group>

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
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* The colours: a ring of the colour around the one chosen (2.0 design). */
  .swatches {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
  }
  .swatch {
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 7px;
    background: var(--c);
    cursor: pointer;
    transition: transform 0.15s var(--ease);
  }
  .swatch:hover {
    transform: scale(1.1);
  }
  .swatch[aria-pressed='true'] {
    box-shadow:
      0 0 0 2px var(--raise),
      0 0 0 4px var(--c);
  }
  /* The project's tile as the rail shows it (36 px), beside its letters. */
  .tile-set {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
  }
  .tile-prev {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--c);
    color: var(--on-tile);
    font: 650 13px/1 var(--font);
    letter-spacing: 0.01em;
  }
  .letters {
    width: 96px;
    text-align: center;
  }
  .label-text {
    width: 150px;
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
  .note {
    max-width: 64ch;
    margin: 8px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  /* « Ses fichiers », harness by harness (choice 28): « Garde-fou » neutral, « Barrière » green. */
  .fence {
    display: grid;
    margin: 10px 0 12px;
    padding: 0;
    list-style: none;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .fence li {
    display: grid;
    grid-template-columns: 110px 92px minmax(0, 1fr);
    align-items: baseline;
    gap: 10px;
    padding: 9px 12px;
    font-size: 12.5px;
  }
  .fence li + li {
    border-top: 1px solid var(--line);
  }
  .engine {
    font-weight: 560;
  }
  .smark {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    gap: 5px;
    width: fit-content;
    height: 19px;
    padding: 0 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .smark > :global(.icon) {
    color: var(--muted);
  }
  .smark.barrier {
    background: oklch(from var(--run) l c h / 0.1);
    color: var(--run);
  }
  .smark.barrier > :global(.icon) {
    color: currentColor;
  }
  .how {
    color: var(--muted);
    line-height: 1.45;
    text-wrap: pretty;
  }
  .private-accounts {
    display: grid;
    gap: 8px;
    margin-top: 10px;
  }
  .acct-note {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 8px;
    margin: 0;
    padding: 9px 11px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.03);
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .acct-note > :global(.icon) {
    flex: none;
    margin-top: 2px;
    color: var(--muted);
  }
  .acct-note.warn {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .acct-note .btn {
    flex: none;
    height: 26px;
  }
  .acct-note.warn > :global(.icon) {
    color: var(--wait);
  }
  .grow {
    flex: 1;
    min-width: 200px;
  }
  .acct-note.warn .grow {
    flex-basis: calc(100% - 30px);
  }
  .copy {
    width: 320px;
    max-width: 100%;
    resize: vertical;
    font-size: 12px;
  }
  /* The launch commands on a grid. */
  .cmds {
    display: grid;
    gap: 6px;
    margin-top: 12px;
  }
  .cmd-row {
    display: grid;
    grid-template-columns: 120px minmax(0, 1fr) 120px 128px 28px;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: none;
  }
  .cmd-row .field {
    width: 100%;
    min-width: 0;
  }
  .cmd-head {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .del:hover:not(:disabled) {
    color: var(--del);
  }
  .launch-acts {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 8px 0 12px;
  }
  .launch-acts .suggest {
    margin-left: auto;
  }
  .add-line {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
  }
  .add-line:hover {
    background: var(--hover);
    color: var(--text);
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
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .proposal {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 4px 0 12px;
    padding: 12px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: 7px;
    background: var(--wait-bg);
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .part {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h4 {
    margin: 0;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .steps {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  /* What is read here is what the shell reads, in that order: right-to-left letters must not reorder a « ; » or a « | ». */
  .name,
  .command,
  .facts dd {
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .name {
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never scrolled out of sight, however long. */
  .command {
    margin: 0;
    padding: 6px 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    margin: 0;
    font-size: 12px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  /* Narrow: the folder and the shell of a launch under its command. */
  @container settings (max-width: 600px) {
    .cmd-row {
      grid-template-columns: 110px minmax(0, 1fr) 28px;
    }
    .cmd-head {
      display: none;
    }
    .cmd-row .field:nth-of-type(3),
    .cmd-row select {
      grid-column: 2 / span 1;
    }
    .fence li {
      grid-template-columns: 90px minmax(0, 1fr);
    }
    .fence .how {
      grid-column: 1 / -1;
    }
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

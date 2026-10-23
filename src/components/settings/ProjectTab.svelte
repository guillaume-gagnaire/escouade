<script lang="ts">
  import { openPath } from '@tauri-apps/plugin-opener';
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
  import { PROJECT_COLORS } from '../../lib/theme';
  import type { Account, CommitMode, LeftoverReport, Project, WorktreeStep } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import SharedMark from './SharedMark.svelte';
  import SharedSection from './SharedSection.svelte';
  import StepList from './StepList.svelte';
  import Switch from './Switch.svelte';
  import WorktreeBaseField from './WorktreeBaseField.svelte';
  import { tip } from '../../lib/tip';
  import { DEFAULT_WORKTREES, worktreesRoot, type WorktreesProblem } from '../../lib/worktrees';

  // A project's own settings: who it is, its worktrees and what runs in them, its launch commands (its draft: the
  // modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);

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

<!-- Its .escouade.yml first: its commands may wait for the user to read them. -->
<SharedSection {project} />

<Group title={t('settings.project.identity')}>
  <Row label={t('common.name')}>
    <input class="field input" aria-label={t('settings.project.nameLabel')} spellcheck="false" bind:value={draft.name} />
  </Row>
  <Row label={t('common.folder')}>
    <span class="path mono" use:tip={{ text: project.path, cut: true }}>{project.path}</span>
    <button class="btn" onclick={() => openPath(project.path).catch((err) => app.toast(String(err), 'error'))}>{t('common.open')}</button>
  </Row>
  <Row label={t('common.color')}>
    <div class="colors" role="group" aria-label={t('common.color')}>
      {#each PROJECT_COLORS as c, i (c)}
        <button
          class="swatch"
          aria-label={t('settings.project.colorN', { n: i + 1 })}
          aria-pressed={draft.color === c}
          style:background={c}
          style:box-shadow={draft.color === c ? '0 0 0 2px var(--bg), 0 0 0 4px var(--text)' : 'none'}
          onclick={() => (draft.color = c)}
        ></button>
      {/each}
    </div>
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
</Group>

<!-- Only while a harness has Remote Control. -->
{#if anyCan('remoteControl')}
  <Group title={t('settings.project.remoteTitle')}>
    <Row label={t('settings.project.remote')} desc={t('settings.project.remoteDesc')}>
      <div class="remote">
        <!-- The setting for all agents covers every project: the switch shows it, the project’s own value stays as it is. -->
        <Switch
          label={t('settings.project.remote')}
          disabled={coveredByAll}
          describedby={coveredByAll ? 'remote-covered' : undefined}
          bind:on={() => coveredByAll || draft.remoteControl, (v) => (draft.remoteControl = v)}
        />
        {#if coveredByAll}<p class="why" id="remote-covered">{t('settings.project.remoteGlobal')}</p>{/if}
      </div>
    </Row>
  </Group>
{/if}

<!-- « Projet privé »: what the agents of the other projects see of it through the MCP server, what keeps its files
     from them, harness by harness (a guard or a barrier, said as it is), then, for a private one, its accounts. -->
<Group title={t('settings.project.privateTitle')}>
  <Row label={t('settings.project.private')} desc={t('settings.project.privateDesc')}>
    <Switch label={t('settings.project.private')} bind:on={draft.private} />
  </Row>
  <Row label={t('settings.project.fenceTitle')} desc={t('settings.project.fenceDesc')} wide>
    <ul class="fence" aria-label={t('settings.project.fenceTitle')}>
      {#each FENCE as f (f.id)}
        <li>
          <span class="engine">{f.name}</span>
          <span class="kind" class:barrier={f.kind === 'barrier'}>{t(`settings.project.fenceKind.${f.kind}`)}</span>
          <span class="how">{t(`settings.project.fenceHow.${f.id}`)}</span>
        </li>
      {/each}
    </ul>
    <p class="at">{t('settings.project.fenceNote')}</p>
  </Row>
  {#if draft.private}
    {@const harness = engineName(privateEngine)}
    <!-- Its accounts (ESC-141): reserved to it alone, or still shared, with what to do about it. -->
    <Row label={t('accounts.tab.list')} wide>
      <div class="private-accounts">
        {#if ownAccounts.length}
          <p class="said ok" role="status">
            {t('accounts.private.own', { count: ownAccounts.length, names: quotedNames(ownAccounts), harness })}
            {#if ownAccounts.some(isPrincipal)}{t('accounts.private.machineToo', { harness })}{/if}
          </p>
        {/if}
        {#if sharedAccounts.length}
          <p class="said warn" role="status">
            {t('accounts.private.shared', { count: sharedAccounts.length, names: quotedNames(sharedAccounts), harness })}
          </p>
        {/if}
        {#if sharedAccounts.length || !ownAccounts.length}
          <div class="actions start">
            <button class="btn" onclick={() => (app.modal = { kind: 'account', engine: privateEngine, reserveFor: project.id })}>
              {t('accounts.private.add')}
            </button>
            {#each sharedAccounts as a (a.id)}
              <button class="btn ghost" use:tip={t('accounts.private.reserveTitle')} onclick={() => reserveHere(a)}>
                {t('accounts.private.reserve', { name: accountName(a) })}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </Row>
    <Row label={t('accounts.private.sharedOnLimit')} desc={t('accounts.private.sharedOnLimitDesc')}>
      <Switch label={t('accounts.private.sharedOnLimit')} bind:on={draft.sharedOnLimit} />
    </Row>
  {/if}
</Group>

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
</Group>

<Group title={t('settings.project.worktrees')} anchor="worktrees" note={t('settings.project.worktreesNote')}>
  <Row label={t('settings.project.fill')} desc={t('settings.project.fillWorktreesDesc')} descId="worktrees-suggest-desc">
    <!-- Two buttons of this tab read the same: what each row says of itself tells them apart. -->
    <button
      class="btn suggest"
      aria-describedby="worktrees-suggest-desc"
      disabled={settingsForm.suggesting[project.id]}
      bind:this={suggestButton}
      onclick={() => ask(() => settingsForm.suggest(project.id), suggestButton, 'worktrees-proposal')}
    >
      {settingsForm.suggesting[project.id] ? t('settings.project.filling') : t('settings.project.fillButton')}
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
  <Row label={t('settings.project.setupTitle')} desc={t('settings.project.setupDesc')} wide shared="worktrees.setup">
    <StepList bind:steps={draft.worktreeSetup} kind="setup" onadd={() => settingsForm.addStep('setup')} />
  </Row>
  <Row label={t('settings.project.teardownTitle')} desc={t('settings.project.teardownDesc')} wide shared="worktrees.teardown">
    <StepList bind:steps={draft.worktreeTeardown} kind="teardown" onadd={() => settingsForm.addStep('teardown')} />
  </Row>
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
            <li><span class="name">{k.name}</span> · {k.reason}</li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</Group>

<Group title={t('settings.project.launch')} anchor="launch" note={t('settings.project.launchNote')}>
  <div class="fill">
    <div class="shared"><SharedMark keys="launch" /></div>
    <Row label={t('settings.project.fill')} desc={t('settings.project.fillLaunchDesc')} descId="launch-suggest-desc">
      <button
        class="btn suggest"
        aria-describedby="launch-suggest-desc"
        disabled={settingsForm.suggestingLaunch[project.id]}
        bind:this={launchButton}
        onclick={() => ask(() => settingsForm.suggestLaunch(project.id), launchButton, 'launch-proposal')}
      >
        {settingsForm.suggestingLaunch[project.id] ? t('settings.project.filling') : t('settings.project.fillButton')}
      </button>
    </Row>
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
  </div>
  {#each draft.runCommands as c, i (c.id)}
    <fieldset class="cmd">
      <legend class="sr">{t('settings.project.commandN', { n: i + 1 })}</legend>
      <div class="line">
        <label class="f grow"
          ><span>{t('common.name')}</span><input
            class="field"
            bind:value={c.name}
            placeholder={t('settings.project.namePlaceholder')}
          /></label
        >
        <label class="f"
          ><span>{t('settings.fields.shell')}</span>
          <select class="field" bind:value={c.shell}>
            {#each app.shells as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
            {#if !app.shells.some((s) => s.id === c.shell)}<option value={c.shell}
                >{t('settings.fields.shellNotFound', { name: c.shell })}</option
              >{/if}
          </select>
        </label>
        <button class="del" use:tip={t('common.delete')} aria-label={t('common.delete')} onclick={() => draft.runCommands.splice(i, 1)}
          >×</button
        >
      </div>
      <label class="f"
        ><span>{t('common.command')}</span><input
          class="field mono"
          bind:value={c.command}
          placeholder={t('settings.project.commandPlaceholder')}
        /></label
      >
      <label class="f"
        ><span>{t('settings.fields.subfolder')} <em>{t('settings.project.subfolderHint')}</em></span><input
          class="field mono"
          bind:value={c.cwd}
          placeholder={t('settings.project.subfolderPlaceholder')}
        /></label
      >
    </fieldset>
  {:else}
    <div class="empty">{t('settings.project.noCommandsYet')}</div>
  {/each}
  <div class="add"><button class="btn" onclick={() => settingsForm.addCommand()}>{t('settings.fields.addCommand')}</button></div>
</Group>

<Group title={t('wizard.reopen.label')}>
  <Row label={t('wizard.reopen.label')} desc={t('wizard.reopen.desc')}>
    <!-- The assistant takes the place of these settings: it saves what it sets itself. -->
    <button class="btn" onclick={() => (app.modal = { kind: 'newProject', projectId: project.id })}>{t('wizard.reopen.button')}</button>
  </Row>
</Group>

<Group title={t('settings.project.danger')}>
  <Row label={t('settings.project.close')} desc={t('settings.project.closeDesc')}>
    <button class="btn danger" onclick={askClose}>{t('settings.project.closeButton')}</button>
  </Row>
</Group>

<style>
  .input {
    flex: 1;
    min-width: 0;
    max-width: 340px;
    height: 32px;
    padding: 0 10px;
    background: var(--panel);
    font-weight: 600;
  }
  .setting {
    font-weight: 400;
    font-size: 12.5px;
  }
  .setting[aria-invalid='true'] {
    border-color: var(--del);
  }
  .at,
  .error {
    margin: 0;
    font-size: 11.5px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .at {
    color: var(--dim);
  }
  .error {
    color: var(--del);
  }
  .path {
    flex: 1;
    min-width: 0;
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 9px;
    max-width: 60%;
  }
  .swatch {
    width: 24px;
    height: 24px;
    border: none;
    border-radius: 7px;
    padding: 0;
    cursor: pointer;
  }
  .fence {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .private-accounts {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .fence li {
    display: grid;
    grid-template-columns: 96px 84px 1fr;
    align-items: baseline;
    gap: 10px;
    font-size: 12px;
    line-height: 1.45;
  }
  .engine {
    font-weight: 600;
  }
  .kind {
    justify-self: start;
    padding: 1px 7px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-size: 11px;
    color: var(--muted);
  }
  .kind.barrier {
    color: var(--text);
    border-color: var(--accent);
  }
  .how {
    color: var(--muted);
    text-wrap: pretty;
  }
  .said {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted);
  }
  .said.ok {
    color: var(--ok);
  }
  .said.warn {
    color: var(--wait);
  }
  .actions.start {
    justify-content: flex-start;
    flex-wrap: wrap;
  }
  .remote {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    flex-shrink: 0;
  }
  .why {
    margin: 0;
    max-width: 230px;
    font-size: 11.5px;
    line-height: 1.45;
    text-align: right;
    color: var(--muted);
    text-wrap: pretty;
  }
  .copy {
    height: auto;
    resize: vertical;
    padding: 8px 10px;
    background: var(--panel);
    font-size: 12px;
  }
  .cmd {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 14px 16px;
    border: none;
    border-bottom: 1px solid var(--line);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .line {
    display: flex;
    align-items: flex-end;
    gap: 10px;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: 5px;
    font-size: 12px;
    color: var(--muted);
  }
  .grow {
    flex: 1;
  }
  em {
    color: var(--dim);
    font-style: normal;
  }
  .cmd .field {
    height: 32px;
    background: var(--panel);
    font-size: 12.5px;
  }
  select.field {
    min-width: 170px;
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
  .leftovers {
    font-size: 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .leftovers p,
  .leftovers ul {
    margin: 0;
  }
  .leftovers ul {
    padding-left: 18px;
  }
  .leftovers .name {
    color: var(--text);
    font-weight: 600;
  }
  .empty {
    padding: 14px 16px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
    color: var(--dim);
  }
  .add {
    padding: 12px 16px;
  }
  .suggest {
    flex-shrink: 0;
  }
  .shared:not(:empty) {
    padding: 10px 16px 0;
  }
  /* Its row stands apart from the commands below, whose boxes have no border above. */
  .fill {
    border-bottom: 1px solid var(--line);
  }
  .proposal {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 16px;
    border-top: 1px solid var(--line);
  }
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-left: 2px solid var(--wait);
    background: var(--wait-soft);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text);
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
    font-size: 12px;
    color: var(--dim);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
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
    font-size: 13px;
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never scrolled out of sight, however long. */
  .command {
    margin: 0;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--elev);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    font-size: 11.5px;
  }
  .facts dt {
    color: var(--dim);
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
</style>

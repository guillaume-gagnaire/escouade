<script lang="ts">
  import { openPath } from '@tauri-apps/plugin-opener';
  import { tick, type Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { askCloseProject } from '../../lib/project-actions';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { PROJECT_COLORS } from '../../lib/theme';
  import type { CommitMode, Project, WorktreeStep } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import StepList from './StepList.svelte';
  import Switch from './Switch.svelte';

  // A project's own settings: who it is, its worktrees and what runs in them, its launch commands (its draft: the
  // modal's project).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  /** Who writes the commits of the files panel's « Commit… » and « Commit tout… ». */
  const COMMIT_MODES = $derived<{ value: CommitMode; label: string }[]>(
    (['agent', 'direct'] as const).map((value) => ({ value, label: t(`settings.project.commitModes.${value}`) })),
  );

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
  <Row label={t('common.name')}>
    <input class="field input" aria-label={t('settings.project.nameLabel')} spellcheck="false" bind:value={draft.name} />
  </Row>
  <Row label={t('common.folder')}>
    <span class="path mono" title={project.path}>{project.path}</span>
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
</Group>

<Group title="Git">
  <Row label={t('settings.project.worktreePerAgent')} desc={t('settings.project.worktreePerAgentDesc')}>
    <Switch label={t('settings.project.worktreePerAgent')} bind:on={draft.worktreePerAgent} />
  </Row>
  <Row label={t('settings.project.copy')} desc={t('settings.project.copyDesc')} descId="copy-desc" wide>
    <textarea
      class="field mono copy"
      rows="3"
      placeholder=".env*"
      aria-label={t('settings.project.copy')}
      aria-describedby="copy-desc"
      bind:value={draft.copy}
    ></textarea>
  </Row>
  <Row label={t('settings.project.commit')} desc={t('settings.project.commitDesc')}>
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
  <Row label={t('settings.project.setupTitle')} desc={t('settings.project.setupDesc')} wide>
    <StepList bind:steps={draft.worktreeSetup} kind="setup" onadd={() => settingsForm.addStep('setup')} />
  </Row>
  <Row label={t('settings.project.teardownTitle')} desc={t('settings.project.teardownDesc')} wide>
    <StepList bind:steps={draft.worktreeTeardown} kind="teardown" onadd={() => settingsForm.addStep('teardown')} />
  </Row>
</Group>

<Group title={t('settings.project.launch')} anchor="launch" note={t('settings.project.launchNote')}>
  <div class="fill">
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
        <button class="del" title={t('common.delete')} aria-label={t('common.delete')} onclick={() => draft.runCommands.splice(i, 1)}
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

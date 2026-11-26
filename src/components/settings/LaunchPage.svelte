<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../Icon.svelte';
  import { customPageOf, customPageProblem, relativeToProject } from '../../lib/custom-page';
  import { t } from '../../lib/i18n';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import { runUrlProblem } from '../../lib/run-url';
  import type { PagesOpen, Project, RunOpen } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Facts from './Facts.svelte';
  import Group from './Group.svelte';
  import Proposal, { ask, settle } from './Proposal.svelte';
  import Row from './Row.svelte';
  import SharedMark from './SharedMark.svelte';

  // Projets › <projet> › Lancement: where the pages of its launches open, its launch commands, and Claude to fill them.
  // The ⚙ of the Lancement panel, « Configurer » and « Proposer des commandes » open here (annexe B).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  /** What of its fields cannot be saved as typed (ESC-380), said on their row: what was saved stays. */
  const held = $derived(settingsForm.projectProblems(project.id));

  /** « Ouvrir les pages des lancements » (ESC-311): in Escouade (a browser's row, ESC-657) or the system's browser. */
  const PAGES_OPEN = $derived<{ value: PagesOpen; label: string }[]>(
    (['agent', 'system'] as const).map((value) => ({ value, label: t(`settings.project.pagesOpens.${value}`) })),
  );

  /** « Où l'ouvrir » of a launch command's address (ESC-560). */
  const OPEN_INS = $derived<{ value: RunOpen; label: string }[]>(
    (['page', 'browser'] as const).map((value) => ({ value, label: t(`settings.project.openIns.${value}`) })),
  );
  /** « Ouvrir à son lancement » ticked, by command: its address may still be empty. */
  let opening = $state<Record<string, boolean>>({});
  const opens = (c: { id: string; url?: string }) => opening[c.id] ?? !!c.url?.trim();
  /** An address that follows a worktree's ports, as the help of the field shows it. */
  const EXAMPLE = 'http://localhost:${ESCOUADE_PORT_BASE:-3000}';

  const launchProposal = $derived(settingsForm.launchProposal[project.id]);

  /** « Page à soi » (AP-7): this machine's first, then the one .escouade.yml proposes once trusted. */
  const customPage = $derived(customPageOf(project, draft.customPage));
  const customProblem = $derived(customPageProblem(draft.customPage ?? ''));
  async function chooseCustomPage() {
    const path = await open({
      multiple: false,
      title: t('settings.project.customPagePickTitle'),
      defaultPath: project.path,
      filters: [{ name: t('settings.project.customPageFilter'), extensions: ['html', 'htm'] }],
    });
    if (typeof path === 'string') draft.customPage = relativeToProject(path, project.path);
  }
  let launchButton = $state<HTMLButtonElement>();
</script>

<Group title={t('settings.project.launch')} anchor="launch" note={t('settings.project.launchNote')}>
  <SharedMark keys="launch" />
  <Row label={t('settings.project.pagesOpen')} desc={t('settings.project.pagesOpenDesc')} wide>
    <Chips label={t('settings.project.pagesOpen')} options={PAGES_OPEN} bind:value={draft.pagesOpen} />
  </Row>
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
      {@const problem = runUrlProblem(c.url ?? '')}
      <fieldset class="cmd-row">
        <legend class="sr">{t('settings.project.commandN', { n: i + 1 })}</legend>
        <input
          class="field"
          aria-label={t('common.name')}
          aria-invalid={!c.name.trim()}
          aria-describedby={held.commands ? 'commands-error' : undefined}
          bind:value={c.name}
          placeholder={t('settings.project.namePlaceholder')}
        />
        <input
          class="field mono"
          aria-label={t('common.command')}
          aria-invalid={!c.command.trim()}
          aria-describedby={held.commands ? 'commands-error' : undefined}
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
        <div class="open">
          <label class="check"
            ><input
              type="checkbox"
              checked={opens(c)}
              onchange={(e) => {
                opening[c.id] = e.currentTarget.checked;
                if (e.currentTarget.checked) c.openIn ??= 'page';
                else c.url = '';
              }}
            />{t('settings.project.openOnLaunch')}</label
          >
          {#if opens(c)}
            <input
              class="field mono url"
              aria-label={t('settings.project.openUrl')}
              aria-invalid={!!problem}
              aria-describedby="url-help-{c.id}"
              spellcheck="false"
              bind:value={c.url}
              placeholder={t('settings.project.openUrlPlaceholder')}
            />
            <Chips label={t('settings.project.openIn')} options={OPEN_INS} bind:value={() => c.openIn ?? 'page', (v) => (c.openIn = v)} />
            <p class="hint" class:error={!!problem} id="url-help-{c.id}">
              {problem ?? t('settings.project.openUrlHint', { example: EXAMPLE })}
            </p>
          {/if}
        </div>
      </fieldset>
    {:else}
      <p class="empty">{t('settings.project.noCommandsYet')}</p>
    {/each}
    {#if held.commands}<p class="error" id="commands-error">{t('settings.problem.command')}</p>{/if}
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
    <Proposal
      id="launch-proposal"
      label={t('settings.project.proposedLaunch')}
      notice={t('settings.project.proposedLaunchNote')}
      take={() => settle(() => settingsForm.takeLaunchProposal(project.id), launchButton)}
      ignore={() => settle(() => delete settingsForm.launchProposal[project.id], launchButton)}
    >
      <ul class="steps">
        {#each launchProposal as c (c.id)}
          <li>
            <span class="name">{revealHidden(c.name.trim())}</span>
            <pre class="command mono">{revealHidden(c.command)}</pre>
            <Facts shell={c.shell} dir={c.cwd.trim() ? revealHidden(c.cwd) : t('settings.project.projectFolder')} />
          </li>
        {/each}
      </ul>
    </Proposal>
  {/if}
</Group>

<Group title={t('settings.project.agentPagesGroup')} anchor="custom-page">
  <Row
    label={t('settings.project.customPage')}
    desc={t('settings.project.customPageDesc')}
    wide
    error={customProblem ? t('settings.project.customPageNotHtml') : ''}
    errorId="custom-page-error"
  >
    <div class="custom">
      <input
        class="field mono"
        aria-label={t('settings.project.customPage')}
        aria-invalid={!!customProblem}
        aria-describedby={customProblem ? 'custom-page-error' : 'custom-page-from'}
        spellcheck="false"
        placeholder={t('settings.project.customPagePlaceholder')}
        value={draft.customPage ?? ''}
        oninput={(e) => (draft.customPage = e.currentTarget.value)}
      />
      <button class="btn" onclick={chooseCustomPage}>{t('settings.project.customPagePick')}</button>
      {#if draft.customPage?.trim()}
        <button class="btn" onclick={() => (draft.customPage = '')}>{t('settings.project.customPageClear')}</button>
      {/if}
    </div>
    <p class="hint" id="custom-page-from" data-testid="custom-page-from" data-from={customPage.from}>
      {#if customPage.from === 'machine'}
        {customPage.file ? t('settings.project.customPageAhead', { path: customPage.file }) : ''}
      {:else if customPage.from === 'repo'}
        {t('settings.project.customPageRepo', { path: customPage.path })}
      {:else if customPage.from === 'waiting'}
        {t('settings.project.customPageWaiting', { path: customPage.path })}
      {:else}
        {t('settings.project.customPageNone')}
      {/if}
    </p>
  </Row>
</Group>

<style>
  .custom {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .custom .field {
    flex: 1 1 220px;
    min-width: 0;
  }
  .error {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
    color: var(--del);
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
  /* « Ouvrir à son lancement », under the command it belongs to. */
  .open {
    grid-column: 2 / -1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    min-width: 0;
    margin-bottom: 6px;
  }
  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12.5px;
    cursor: pointer;
  }
  .open .url {
    flex: 1 1 220px;
    width: auto;
    min-width: 0;
  }
  .hint {
    flex-basis: 100%;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .hint.error {
    color: var(--del);
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
  .empty {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
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
    .cmd-row select,
    .open {
      grid-column: 2 / span 1;
    }
  }
</style>

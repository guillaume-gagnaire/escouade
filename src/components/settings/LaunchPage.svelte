<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { PagesOpen, Project } from '../../lib/types';
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

  /** « Ouvrir les pages des lancements » (ESC-311). */
  const PAGES_OPEN = $derived<{ value: PagesOpen; label: string }[]>(
    (['agent', 'window', 'system'] as const).map((value) => ({ value, label: t(`settings.project.pagesOpens.${value}`) })),
  );

  const launchProposal = $derived(settingsForm.launchProposal[project.id]);
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

<style>
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
    .cmd-row select {
      grid-column: 2 / span 1;
    }
  }
</style>

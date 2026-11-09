<script lang="ts">
  import Icon from '../Icon.svelte';
  import { openPath } from '../../lib/opener';
  import { t } from '../../lib/i18n';
  import { choosesEngine, engineOptions } from '../../lib/engines';
  import { accountName, accountsOf, serves } from '../../lib/accounts';
  import { askCloseProject } from '../../lib/project-actions';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { projectPalette } from '../../lib/plugin-theme.svelte';
  import { cleanLabel, cleanLetters, initials, tileText } from '../../lib/project-rail';
  import { tip } from '../../lib/tip';
  import type { Project } from '../../lib/types';
  import TileFace from '../TileFace.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import ProposedPlugins from './ProposedPlugins.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // Projets › <projet> › Projet (direction A, design/settings-2-2/index.html#a-projet): who the project is (its name,
  // folder, colour, tile), its default harness and account, the setup assistant, the plugins its .escouade.yml proposes;
  // « Avancé » › « Fermer le projet… ».
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);
  /** The project as the backend last found it, whatever the draft says. */
  const live = $derived(app.projects.find((p) => p.id === project.id) ?? project);
  /** What of its fields cannot be saved as typed (ESC-380), said on their row: what was saved stays. */
  const held = $derived(settingsForm.projectProblems(project.id));

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

  /** Asks before closing it, then comes back to this page, its draft as it was. */
  function askClose() {
    // Taken now: the page is gone while the confirmation is open.
    const projectId = project.id;
    askCloseProject(project, () => {
      app.modal = { kind: 'settings', page: 'project', projectId, resume: true };
    });
  }
</script>

<!-- The plugins its .escouade.yml proposes and this machine lacks (PLG-14): read from the project as the backend has it. -->
<ProposedPlugins project={live} />

<Group title={t('settings.project.identity')}>
  <Row
    label={t('settings.project.nameLabel')}
    error={held.name ? t('settings.problem.projectName', { name: live.name }) : ''}
    errorId="project-name-error"
  >
    <input
      class="field input"
      aria-label={t('settings.project.nameLabel')}
      aria-invalid={held.name}
      aria-describedby={held.name ? 'project-name-error' : undefined}
      spellcheck="false"
      bind:value={draft.name}
    />
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
  <!-- From two harnesses that can be used on (ESC-391), or the one chosen that cannot any more beside one, saying why: the
       one of its new agents and tickets (none chosen: Claude Code). -->
  {#if choosesEngine(draft.engine ?? 'claude')}
    <Row label={t('settings.project.engine')} desc={t('settings.project.engineDesc')} shared="agents.engine">
      <Chips
        label={t('settings.project.engine')}
        options={engineOptions(draft.engine ?? 'claude')}
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

<Group>
  <Row label={t('wizard.reopen.label')} desc={t('wizard.reopen.desc')}>
    <!-- The assistant takes the place of these settings: it saves what it sets itself. -->
    <button class="btn" onclick={() => (app.modal = { kind: 'newProject', projectId: project.id })}>{t('wizard.reopen.button')}</button>
  </Row>
</Group>

<!-- « Zone sensible » of the 2.0 window: one action, that no alarming title announces (PROPOSITION.md §2, « Les noms »). -->
<Group advanced="project">
  <Row label={t('settings.project.close')} desc={t('settings.project.closeDesc')}>
    <button class="btn danger" onclick={askClose}>{t('settings.project.closeButton')}</button>
  </Row>
</Group>

<style>
  .input {
    width: 220px;
    max-width: 100%;
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
</style>

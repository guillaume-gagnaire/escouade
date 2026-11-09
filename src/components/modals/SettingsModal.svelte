<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';
  import { tileText } from '../../lib/project-rail';
  import { pageInfo, SAVE_PAUSE_MS, settingsForm, type RowInfo, type SettingsPage } from '../../lib/settings.svelte';
  import { rowElement } from '../../lib/settings-search';
  import { app } from '../../lib/state.svelte';
  import { applyTheme } from '../../lib/theme';
  import AboutPage from '../settings/AboutPage.svelte';
  import AccountsTab from '../settings/AccountsTab.svelte';
  import AdvancedPage from '../settings/AdvancedPage.svelte';
  import AgentsPage from '../settings/AgentsPage.svelte';
  import BoardTab from '../settings/BoardTab.svelte';
  import ClosedProjects from '../settings/ClosedProjects.svelte';
  import GitPage from '../settings/GitPage.svelte';
  import GeneralPage from '../settings/GeneralPage.svelte';
  import LaunchPage from '../settings/LaunchPage.svelte';
  import NewProjects from '../settings/NewProjects.svelte';
  import NotificationsPage from '../settings/NotificationsPage.svelte';
  import PluginsPage from '../settings/PluginsPage.svelte';
  import PrivacyPage from '../settings/PrivacyPage.svelte';
  import ProjectPage from '../settings/ProjectPage.svelte';
  import ServicesPage from '../settings/ServicesPage.svelte';
  import SettingsNav from '../settings/SettingsNav.svelte';
  import TeamPage from '../settings/TeamPage.svelte';
  import TicketsPage from '../settings/TicketsPage.svelte';
  import VoiceTab from '../settings/VoiceTab.svelte';
  import Icon from '../Icon.svelte';
  import TileFace from '../TileFace.svelte';

  // All the settings, the app's and each project's, in one place (direction A, design/settings-2-2/index.html#a-*):
  // the bar of the pages on the left, a page at a time on the right. Each setting is saved as it changes (ESC-380): a
  // choice at once, a text when it loses the focus or after a pause in the typing; no « Enregistrer ». Each page has its
  // file; a link of the app gives the page (« Projets › demo-api › Lancement ») and, at times, the group it is about.
  let {
    page,
    projectId,
    at,
    resume = false,
    suggest = false,
  }: {
    page?: SettingsPage;
    projectId?: string;
    /** The group of the page it opens on (its `anchor`). */
    at?: string;
    resume?: boolean;
    suggest?: boolean;
  } = $props();

  // Read again from the app each time the modal opens, unless it comes back from a modal it opened. Asked for it, Claude reads
  // the project to fill the launch commands of the draft (only a fresh one: coming back is not asking again).
  untrack(() => {
    if (resume) return settingsForm.resume({ page, projectId });
    settingsForm.open({ page, projectId });
    if (suggest && settingsForm.projectId) void settingsForm.suggestLaunch(settingsForm.projectId);
  });

  const info = $derived(pageInfo(settingsForm.page));
  const project = $derived(app.projects.find((p) => p.id === settingsForm.projectId));
  const draft = $derived(settingsForm.project);
  /** A page of a project shows that project: its tile beside the title, its colour on the window. */
  const ofProject = $derived(info.scope === 'project' && !!project && !!draft);

  // The tint follows the page: Escouade's teal on its pages, the project's colour (as in the draft) on the project's.
  // The app's own tint comes back when the window closes.
  $effect(() => applyTheme(ofProject && draft ? draft.color : null));
  // A change of the fields: typed (an `input`), saved after a pause in the typing; else (a switch, a choice, a command
  // added, a shortcut recorded, a proposal taken) at once. A `change` (a field left, a menu chosen) or a click saves
  // what waits at once. Closed, what waits is saved.
  let typing = false;
  $effect(() => {
    JSON.stringify(settingsForm.settings);
    JSON.stringify(settingsForm.projects);
    untrack(() => {
      settingsForm.schedule(typing ? SAVE_PAUSE_MS : 0);
      typing = false;
    });
  });
  const flush = () => {
    typing = false;
    void settingsForm.flush();
  };

  onDestroy(() => {
    flush();
    app.applyTheme();
  });

  let body = $state<HTMLElement>();

  // Each page, and each project's, starts at its top, or at the group the link asked for (on the page it opened on), or
  // at the row a search opened, lit.
  const opened = untrack(() => settingsForm.page);
  $effect(() => {
    const anchor = settingsForm.page === opened ? at : undefined;
    void settingsForm.projectId;
    const found = settingsForm.found;
    if (!body) return;
    if (found && found.row.page === settingsForm.page) {
      void light(body, found.row);
      return;
    }
    const target = anchor ? body.querySelector(`[data-anchor="${anchor}"]`) : null;
    if (target) target.scrollIntoView({ block: 'start' });
    else body.scrollTop = 0;
  });

  /**
   * The row found, brought into view and lit for a moment. Its page may still be reading what it shows (the accounts,
   * the plugins): looked for along a few frames, the top of the page meanwhile.
   */
  async function light(panel: HTMLElement, row: RowInfo) {
    panel.scrollTop = 0;
    await tick();
    for (let i = 0; i < 30; i++) {
      const el = rowElement(panel, row);
      if (el) {
        el.scrollIntoView({ block: 'center' });
        el.classList.remove('s-found');
        void el.offsetWidth;
        el.classList.add('s-found');
        setTimeout(() => el.classList.remove('s-found'), 2400);
        return;
      }
      await new Promise((r) => requestAnimationFrame(r));
    }
  }

  // Only a click on the overlay closes it: not the end of a selection dragged out of a field.
  let downOnOverlay = false;

  const close = () => (app.modal = null);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="overlay"
  onmousedown={(e) => (downOnOverlay = e.target === e.currentTarget)}
  onclick={(e) => downOnOverlay && e.target === e.currentTarget && close()}
>
  <div
    class="settings"
    use:trapFocus
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('common.settings')}
    data-scope={info.scope}
    data-saving={settingsForm.saving}
    oninput={() => (typing = true)}
    onchange={flush}
    onclick={flush}
  >
    <SettingsNav />
    <div class="s-main">
      <div class="s-head">
        {#if ofProject && project && draft}
          <span class="tile" style:--c={draft.color} aria-hidden="true"
            ><TileFace
              text={tileText({ name: draft.name.trim() || project.name, tileLetters: draft.tileLetters })}
              logo={settingsForm.logoOf(project.id)}
            /></span
          >
        {/if}
        <div class="hd">
          <h1 id="settings-title">{ofProject && info.id === 'project' ? draft?.name.trim() || project?.name : info.label}</h1>
          <p>{info.desc}</p>
        </div>
        <button class="icon-btn close" onclick={close} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
      </div>
      <div class="s-body" bind:this={body} role="region" id="settings-panel" aria-labelledby="settings-title">
        <div class="inner">
          {#if info.scope === 'project'}
            {#if project && draft}
              {#if info.id === 'git'}
                <GitPage {project} />
              {:else if info.id === 'launch'}
                <LaunchPage {project} />
              {:else if info.id === 'board'}
                <BoardTab {project} />
              {:else if info.id === 'tickets'}
                <TicketsPage {project} />
              {:else if info.id === 'team'}
                <TeamPage {project} />
              {:else if info.id === 'privacy'}
                <PrivacyPage {project} />
              {:else}
                <ProjectPage {project} />
              {/if}
            {:else}
              <p class="none">{t('settings.modal.noProject')}</p>
            {/if}
          {:else if info.id === 'general'}
            <GeneralPage />
          {:else if info.id === 'agents'}
            <AgentsPage />
          {:else if info.id === 'accounts'}
            <AccountsTab />
          {:else if info.id === 'services'}
            <ServicesPage />
          {:else if info.id === 'voice'}
            <VoiceTab />
          {:else if info.id === 'notifications'}
            <NotificationsPage />
          {:else if info.id === 'plugins'}
            <PluginsPage />
          {:else if info.id === 'advanced'}
            <AdvancedPage />
          {:else if info.id === 'allProjects'}
            <NewProjects />
            <ClosedProjects />
          {:else}
            <AboutPage />
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  /* The window of direction A (design/settings-2-2/index.html#a-*): the bar of the pages on a darker side
     (settings/SettingsNav), the page's title, the settings in rows under hairlines (settings/Group, settings/Row).
     No foot: each setting is saved as it changes. 1120 × 780, or the room there is. */
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: var(--scrim);
    animation: fade 0.2s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px) scale(0.985);
    }
  }
  .settings {
    display: grid;
    grid-template-columns: 252px minmax(0, 1fr);
    width: min(1120px, 100vw - 32px);
    height: min(780px, 100vh - 32px);
    overflow: hidden;
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
    animation: rise 0.35s var(--ease);
  }
  .s-main {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .s-head {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 20px 14px 14px 32px;
    border-bottom: 1px solid var(--line);
  }
  .tile {
    display: grid;
    flex: none;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    background: var(--c);
    color: var(--on-tile);
    font: 700 11.5px/1 var(--font);
    overflow: hidden;
  }
  .hd {
    min-width: 0;
  }
  .s-head h1 {
    margin: 0;
    font-size: 17px;
    font-weight: 640;
    letter-spacing: -0.02em;
  }
  .s-head p {
    margin: 3px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .s-head .close {
    margin-left: auto;
  }
  .s-body {
    flex: 1;
    min-height: 0;
    padding: 0 40px 20px 32px;
    overflow: auto;
  }
  .inner {
    max-width: 720px;
    container: settings / inline-size;
  }
  .none {
    margin: 20px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  /* The menus of the settings, as the preview's `.select`: a ring on the window, not a well. */
  /* A path, a command: the mono of the 2.0 design is a size smaller. */
  .s-body :global(.field.mono) {
    font-size: 12px;
  }
  .s-body :global(select.field) {
    height: 30px;
    max-width: 100%;
    padding: 0 6px 0 9px;
    border-radius: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 13px;
    font-weight: 550;
    cursor: pointer;
  }
  .s-body :global(select.field:hover) {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .s-body :global(select.field option) {
    background: var(--raise-2);
    color: var(--text);
  }
  /* The row a search opened: lit in amber, then back to the page's (the tint of something waiting, `--wait`). */
  .s-body :global(.s-found) {
    border-radius: 8px;
    animation: found 2.4s ease-out;
  }
  @keyframes found {
    0%,
    40% {
      background: var(--wait-bg);
      box-shadow:
        0 0 0 6px var(--wait-bg),
        inset 0 0 0 1px var(--wait-line);
    }
    100% {
      background: transparent;
      box-shadow:
        0 0 0 6px transparent,
        inset 0 0 0 1px transparent;
    }
  }
  /* Still with prefers-reduced-motion: it is there at once; the row found stays lit without fading. */
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .settings {
      animation: none;
    }
    .s-body :global(.s-found) {
      animation: none;
      background: var(--wait-bg);
      box-shadow: 0 0 0 6px var(--wait-bg);
    }
  }
</style>

<script lang="ts">
  import { untrack } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { t } from '../../lib/i18n';
  import { SETTINGS_TABS, settingsForm, type SettingsTab } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import AccountsTab from '../settings/AccountsTab.svelte';
  import AppTab from '../settings/AppTab.svelte';
  import ApplicationTab from '../settings/ApplicationTab.svelte';
  import BoardTab from '../settings/BoardTab.svelte';
  import ClosedProjects from '../settings/ClosedProjects.svelte';
  import CodeIndexAllSection from '../settings/CodeIndexAllSection.svelte';
  import NewProjects from '../settings/NewProjects.svelte';
  import PrivateMark from '../PrivateMark.svelte';
  import IntegrationsTab from '../settings/IntegrationsTab.svelte';
  import McpSection from '../settings/McpSection.svelte';
  import ProjectTab from '../settings/ProjectTab.svelte';
  import VoiceTab from '../settings/VoiceTab.svelte';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  // All the settings, the app's and each project's, in one place: a tab at a time, saved together.
  let {
    tab,
    projectId,
    section,
    resume = false,
    suggest = false,
  }: { tab?: SettingsTab; projectId?: string; section?: 'launch' | 'sources'; resume?: boolean; suggest?: boolean } = $props();

  // A fresh draft each time the modal opens, unless it comes back from a modal it opened. Asked for it, Claude reads
  // the project to fill the launch commands of the draft (only a fresh one: coming back is not asking again).
  untrack(() => {
    if (resume) return settingsForm.resume({ tab, projectId });
    settingsForm.open({ tab, projectId });
    if (suggest && settingsForm.projectId) void settingsForm.suggestLaunch(settingsForm.projectId);
  });

  const current = $derived(SETTINGS_TABS.find((x) => x.id === settingsForm.tab) ?? SETTINGS_TABS[0]);
  const project = $derived(app.projects.find((p) => p.id === settingsForm.projectId));
  const draft = $derived(settingsForm.project);
  let body = $state<HTMLElement>();
  /** The section asked for, scrolled to once. */
  let anchor = untrack(() => section);

  // Each tab, and each project's, starts at its top, or at the section asked for.
  $effect(() => {
    void current.id;
    void settingsForm.projectId;
    if (!body) return;
    const target = anchor ? body.querySelector(`[data-anchor="${anchor}"]`) : null;
    anchor = undefined;
    if (target) target.scrollIntoView({ block: 'start' });
    else body.scrollTop = 0;
  });

  // Only a click on the overlay closes it: not the end of a selection dragged out of a field.
  let downOnOverlay = false;

  const close = () => (app.modal = null);

  async function save() {
    if (!(await settingsForm.save())) return;
    close();
    app.toast(t('settings.modal.saved'), 'ok');
  }

  /** Up and down go from tab to tab, Home and End to the first and the last. */
  function onTabKey(e: KeyboardEvent, i: number) {
    const n = SETTINGS_TABS.length;
    const to: Record<string, number> = { ArrowDown: (i + 1) % n, ArrowUp: (i - 1 + n) % n, Home: 0, End: n - 1 };
    const next = to[e.key];
    if (next === undefined) return;
    e.preventDefault();
    settingsForm.tab = SETTINGS_TABS[next].id;
    (e.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>('[role="tab"]')[next]?.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div
  class="overlay"
  onmousedown={(e) => (downOnOverlay = e.target === e.currentTarget)}
  onclick={(e) => downOnOverlay && e.target === e.currentTarget && close()}
>
  <div class="settings" use:trapFocus role="dialog" tabindex="-1" aria-modal="true" aria-label={t('common.settings')}>
    <nav class="s-nav">
      <span class="s-nav-h" aria-hidden="true">{t('common.settings')}</span>
      <div class="tabs" role="tablist" aria-orientation="vertical" aria-label={t('common.settings')}>
        {#each SETTINGS_TABS as entry, i (entry.id)}
          {@const on = entry.id === current.id}
          {@const changed = settingsForm.changed(entry.id)}
          {#if i > 0 && entry.group !== SETTINGS_TABS[i - 1].group}
            <span class="s-group" role="presentation"
              >{entry.group === 'project' ? t('settings.modal.groupProject') : t('settings.modal.groupSystem')}</span
            >
          {/if}
          <button
            class="s-tab"
            class:changed
            aria-describedby={changed ? 'settings-changed' : undefined}
            role="tab"
            id="settings-tab-{entry.id}"
            aria-selected={on}
            aria-controls="settings-panel"
            tabindex={on ? 0 : -1}
            onclick={() => (settingsForm.tab = entry.id)}
            onkeydown={(e) => onTabKey(e, i)}
          >
            <Icon name={entry.icon} /><span class="lbl">{entry.label}</span>
            <span class="chg" use:tip={t('settings.modal.changed')} aria-hidden="true"></span>
          </button>
        {/each}
      </div>
      <span id="settings-changed" hidden>{t('settings.modal.changed')}</span>
      <span class="s-ver">Escouade {app.version}</span>
    </nav>
    <div class="s-main">
      <div class="s-head">
        <div class="hd">
          <h1>{current.label}</h1>
          <p>{current.desc}</p>
        </div>
        <button class="icon-btn close" onclick={close} aria-label={t('common.close')}><Icon name="close" size={14} /></button>
      </div>
      {#if current.scoped && app.projects.length}
        <div class="s-scope">
          <span class="k">{t('common.project')}</span>
          <div class="projects" role="group" aria-label={t('common.project')}>
            {#each app.projects as p (p.id)}
              {@const d = settingsForm.projects[p.id]}
              <button class="proj-pill" aria-pressed={p.id === settingsForm.projectId} onclick={() => (settingsForm.projectId = p.id)}
                ><i style:background={d?.color ?? p.color}></i><span class="pn">{d?.name.trim() || p.name}</span
                >{#if d?.private ?? p.private}<PrivateMark />{/if}</button
              >
            {/each}
          </div>
        </div>
      {/if}
      <div class="s-body" bind:this={body} role="tabpanel" id="settings-panel" aria-labelledby="settings-tab-{current.id}">
        {#if current.id === 'projects' || current.id === 'board' || current.id === 'integrations'}
          {#if project && draft}
            {#if current.id === 'projects'}
              <ProjectTab {project} />
            {:else if current.id === 'board'}
              <BoardTab {project} />
            {:else}
              <IntegrationsTab {project} />
            {/if}
          {:else}
            <p class="none">{t('settings.modal.noProject')}</p>
          {/if}
          <!-- Also with no project open: that is when a folder's old history is most worth finding. -->
          {#if current.id === 'projects'}<CodeIndexAllSection /><NewProjects /><ClosedProjects />{/if}
        {:else if current.id === 'app'}
          <ApplicationTab />
        {:else if current.id === 'accounts'}
          <AccountsTab />
        {:else if current.id === 'voice'}
          <VoiceTab />
        {:else if current.id === 'plugins'}
          <section class="sg plugins-link">
            <p><b>{t('plugins.link.title')}</b> {t('plugins.link.body')}</p>
            <button
              class="btn primary"
              onclick={() => {
                close();
                app.openPlugins();
              }}><Icon name="puzzle" size={14} />{t('plugins.link.open')}</button
            >
          </section>
        {:else if current.id === 'mcp'}
          <McpSection />
        {:else}
          <AppTab tab={current.id} />
        {/if}
      </div>
      <div class="s-foot">
        {#if settingsForm.problem}<span class="problem">{settingsForm.problem}</span>{/if}
        <button class="btn cancel" onclick={close}>{t('common.cancel')}</button>
        <button class="btn primary" disabled={settingsForm.busy || !!settingsForm.problem} onclick={save}>{t('common.save')}</button>
      </div>
    </div>
  </div>
</div>

<style>
  /* The window of the 2.0 design (#settings): the tabs in three groups on a darker side, the tab's title, the project
     by pills, the settings in rows under hairlines (settings/Group, settings/Row), « Annuler » and « Enregistrer ». */
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
    grid-template-columns: 224px minmax(0, 1fr);
    width: min(980px, 100vw - 48px);
    height: min(700px, 100vh - 64px);
    overflow: hidden;
    border-radius: 14px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
    animation: rise 0.35s var(--ease);
  }
  .s-nav {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 16px 10px 14px;
    border-right: 1px solid var(--line);
    background: rgba(0, 0, 0, calc(0.2 * var(--shade)));
    overflow: auto;
  }
  .s-nav-h {
    padding: 2px 10px 10px;
    font-size: 15px;
    font-weight: 640;
    letter-spacing: -0.01em;
  }
  .tabs {
    display: flex;
    flex-direction: column;
  }
  .s-group {
    padding: 14px 10px 4px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .s-tab {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--soft);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .s-tab :global(.icon) {
    color: var(--muted);
  }
  .s-tab .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .s-tab:focus-visible {
    outline-offset: -2px;
  }
  .s-tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  .s-tab[aria-selected='true'] {
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  .s-tab[aria-selected='true'] :global(.icon) {
    color: var(--text);
  }
  .chg {
    flex: none;
    width: 6px;
    height: 6px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--p-ink);
    visibility: hidden;
  }
  .s-tab.changed .chg {
    visibility: visible;
  }
  .s-ver {
    margin-top: auto;
    padding: 12px 10px 0;
    color: var(--muted);
    font-size: 11.5px;
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
    padding: 20px 14px 14px 28px;
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
    width: 30px;
    height: 30px;
    margin-left: auto;
    padding: 0;
    border-radius: 6px;
  }
  .s-scope {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 0 28px 14px;
    border-bottom: 1px solid var(--line);
  }
  .s-scope .k {
    flex: none;
    margin-right: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  /* Many projects: they scroll sideways, on one line. */
  .projects {
    display: flex;
    gap: 6px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .proj-pill {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 7px;
    max-width: 220px;
    height: 28px;
    padding: 0 11px 0 9px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    font-weight: 550;
    white-space: nowrap;
    cursor: pointer;
  }
  .proj-pill i {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2.5px;
  }
  .proj-pill .pn {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .proj-pill:focus-visible {
    outline-offset: -2px;
  }
  .proj-pill:hover {
    background: var(--hover);
    color: var(--text);
  }
  .proj-pill[aria-pressed='true'] {
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  .s-body {
    flex: 1;
    min-height: 0;
    padding: 0 28px 28px;
    overflow: auto;
    container: settings / inline-size;
  }
  .plugins-link {
    display: grid;
    justify-items: start;
    gap: 12px;
    padding-top: 6px;
  }
  .plugins-link p {
    margin: 0;
    max-width: 560px;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.55;
  }
  .plugins-link b {
    color: var(--text);
    font-weight: 600;
  }
  .none {
    margin: 20px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .s-foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px 12px 28px;
    border-top: 1px solid var(--line);
  }
  .problem {
    min-width: 0;
    color: var(--del);
    font-size: 12.5px;
  }
  .s-foot .cancel {
    margin-left: auto;
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
  /* Still with prefers-reduced-motion: it is there at once. */
  @media (prefers-reduced-motion: reduce) {
    .overlay,
    .settings {
      animation: none;
    }
  }
</style>

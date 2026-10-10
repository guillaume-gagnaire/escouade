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
  import IntegrationsTab from '../settings/IntegrationsTab.svelte';
  import ProjectTab from '../settings/ProjectTab.svelte';

  // All the settings, the app's and each project's, in one place: a tab at a time, saved together.
  let {
    tab,
    projectId,
    section,
    resume = false,
    suggest = false,
  }: { tab?: SettingsTab; projectId?: string; section?: 'launch'; resume?: boolean; suggest?: boolean } = $props();

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
  <div class="modal" use:trapFocus role="dialog" tabindex="-1" aria-modal="true" aria-label={t('common.settings')}>
    <nav>
      <span class="title">{t('common.settings')}</span>
      <div class="tabs" role="tablist" aria-orientation="vertical" aria-label={t('common.settings')}>
        {#each SETTINGS_TABS as entry, i (entry.id)}
          {@const on = entry.id === current.id}
          {@const changed = settingsForm.changed(entry.id)}
          <button
            class="tab"
            class:on
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
            <span class="ic" aria-hidden="true">{entry.icon}</span>{entry.label}
            <span class="mark" title={t('settings.modal.changed')} aria-hidden="true"></span>
          </button>
        {/each}
      </div>
      <span id="settings-changed" hidden>{t('settings.modal.changed')}</span>
      <div style="flex:1"></div>
      <span class="version mono">Escouade {app.version}</span>
    </nav>
    <div class="main">
      <div class="head">
        <div class="hd">
          <span class="t">{current.label}</span>
          <span class="desc">{current.desc}</span>
        </div>
        <div style="flex:1"></div>
        <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={close} aria-label={t('common.close')}>×</button>
      </div>
      {#if current.scoped && app.projects.length}
        <div class="scope">
          <span class="k">{t('common.project')}</span>
          <div class="projects" role="group" aria-label={t('common.project')}>
            {#each app.projects as p (p.id)}
              {@const d = settingsForm.projects[p.id]}
              <button
                class="proj"
                class:on={p.id === settingsForm.projectId}
                aria-pressed={p.id === settingsForm.projectId}
                onclick={() => (settingsForm.projectId = p.id)}
                ><span class="sw" style:background={d?.color ?? p.color}></span>{d?.name.trim() || p.name}</button
              >
            {/each}
          </div>
        </div>
      {/if}
      <div class="body" bind:this={body} role="tabpanel" id="settings-panel" aria-labelledby="settings-tab-{current.id}">
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
        {:else if current.id === 'app'}
          <ApplicationTab />
        {:else if current.id === 'accounts'}
          <AccountsTab />
        {:else}
          <AppTab tab={current.id} />
        {/if}
      </div>
      <div class="foot">
        {#if settingsForm.problem}<span class="problem">{settingsForm.problem}</span>{/if}
        <div style="flex:1"></div>
        <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
        <button class="btn primary" disabled={settingsForm.busy || !!settingsForm.problem} onclick={save}>{t('common.save')}</button>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(12, 10, 8, 0.6);
    backdrop-filter: blur(3px);
    animation: fade 0.12s ease-out;
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  .modal {
    width: 900px;
    max-width: calc(100vw - 40px);
    height: 640px;
    max-height: calc(100vh - 60px);
    display: flex;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    animation: ccFadeIn 0.15s ease-out;
  }
  nav {
    width: 210px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 20px 10px 16px;
    background: var(--bg);
    border-right: 1px solid var(--line);
  }
  .title {
    padding: 0 10px 14px;
    font-size: 17px;
    font-weight: 700;
  }
  .tabs {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .tab {
    height: 34px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    text-align: left;
    cursor: pointer;
  }
  .tab:hover {
    background: var(--elev);
  }
  .tab.on {
    background: var(--elev2);
    color: var(--text);
  }
  .ic {
    width: 18px;
    text-align: center;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--dim);
  }
  .tab.on .ic {
    color: var(--accent);
  }
  .mark {
    display: none;
    width: 6px;
    height: 6px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--accent);
  }
  .tab.changed .mark {
    display: block;
  }
  .version {
    padding: 0 10px;
    font-size: 10.5px;
    color: var(--dim);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 20px 18px 14px 28px;
    border-bottom: 1px solid var(--line);
  }
  .hd {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .t {
    font-size: 18px;
    font-weight: 700;
  }
  .desc {
    font-size: 12.5px;
    color: var(--muted);
  }
  .scope {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 28px;
    border-bottom: 1px solid var(--line);
  }
  .k {
    font-size: 12px;
    color: var(--dim);
  }
  .projects {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }
  .proj {
    height: 28px;
    max-width: 220px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 11px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }
  .proj.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
  .sw {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 3px;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 26px;
    padding: 22px 28px 28px;
  }
  .none {
    margin: 0;
    font-size: 12.5px;
    color: var(--dim);
  }
  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 18px 16px 28px;
    border-top: 1px solid var(--line);
  }
  .problem {
    min-width: 0;
    font-size: 12px;
    color: var(--del);
  }
</style>

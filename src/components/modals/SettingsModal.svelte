<script lang="ts">
  import { untrack } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { SETTINGS_TABS, settingsForm, type SettingsTab } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import AppTab from '../settings/AppTab.svelte';
  import BoardTab from '../settings/BoardTab.svelte';
  import ProjectTab from '../settings/ProjectTab.svelte';

  // All the settings, the app's and each project's, in one place: a tab at a time, saved together.
  let {
    tab,
    projectId,
    section,
    resume = false,
  }: { tab?: SettingsTab; projectId?: string; section?: 'launch'; resume?: boolean } = $props();

  // A fresh draft each time the modal opens, unless it comes back from a modal it opened.
  untrack(() => (resume ? settingsForm.resume({ tab, projectId }) : settingsForm.open({ tab, projectId })));

  const current = $derived(SETTINGS_TABS.find((t) => t.id === settingsForm.tab) ?? SETTINGS_TABS[0]);
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
    app.toast('Réglages enregistrés', 'ok');
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
  <div class="modal" use:trapFocus role="dialog" tabindex="-1" aria-modal="true" aria-label="Réglages">
    <nav>
      <span class="title">Réglages</span>
      <div class="tabs" role="tablist" aria-orientation="vertical" aria-label="Réglages">
        {#each SETTINGS_TABS as t, i (t.id)}
          {@const on = t.id === current.id}
          {@const changed = settingsForm.changed(t.id)}
          <button
            class="tab"
            class:on
            class:changed
            aria-describedby={changed ? 'settings-changed' : undefined}
            role="tab"
            id="settings-tab-{t.id}"
            aria-selected={on}
            aria-controls="settings-panel"
            tabindex={on ? 0 : -1}
            onclick={() => (settingsForm.tab = t.id)}
            onkeydown={(e) => onTabKey(e, i)}
          >
            <span class="ic" aria-hidden="true">{t.icon}</span>{t.label}
            <span class="mark" title="Modifié, pas encore enregistré" aria-hidden="true"></span>
          </button>
        {/each}
      </div>
      <span id="settings-changed" hidden>Modifié, pas encore enregistré</span>
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
        <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={close} aria-label="Fermer">×</button>
      </div>
      {#if current.scoped && app.projects.length}
        <div class="scope">
          <span class="k">Projet</span>
          <div class="projects" role="group" aria-label="Projet">
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
        {#if current.id === 'projects' || current.id === 'board'}
          {#if project && draft}
            {#if current.id === 'projects'}
              <ProjectTab {project} />
            {:else}
              <BoardTab {project} />
            {/if}
          {:else}
            <p class="none">Aucun projet ouvert.</p>
          {/if}
        {:else}
          <AppTab tab={current.id} />
        {/if}
      </div>
      <div class="foot">
        {#if settingsForm.problem}<span class="problem">{settingsForm.problem}</span>{/if}
        <div style="flex:1"></div>
        <button class="btn ghost" onclick={close}>Annuler</button>
        <button class="btn primary" disabled={settingsForm.busy || !!settingsForm.problem} onclick={save}>Enregistrer</button>
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

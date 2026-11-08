<script lang="ts">
  import { tick } from 'svelte';
  import { t } from '../../lib/i18n';
  import { keyLabel } from '../../lib/platform';
  import { pageInfo, SETTINGS_PAGES, settingsForm, type SettingsPage } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import { shownVersion } from '../../lib/updater';
  import Icon from '../Icon.svelte';
  import PrivateMark from '../PrivateMark.svelte';

  // The bar of the settings window (direction A, design/settings-2-2/index.html#a-*): the search (inert until R8),
  // Escouade's pages, the projects with the one chosen unfolded on its pages, « Tous les projets », and « À propos »
  // with the version at its foot. Up and down go from entry to entry, Home and End to the first and the last; going
  // through a project opens it. A dot says a page has something to save.

  /** An entry of the bar: a page of Escouade, a project (folded or not), or a page of the project unfolded. */
  interface Entry {
    key: string;
    page?: SettingsPage;
    projectId?: string;
  }

  const APP = SETTINGS_PAGES.filter((p) => p.scope === 'app' && p.id !== 'about' && p.id !== 'allProjects');
  const PROJECT = SETTINGS_PAGES.filter((p) => p.scope === 'project');

  const scope = $derived(pageInfo(settingsForm.page).scope);
  /** The project unfolded: the one chosen, while a page of a project is on screen. */
  const open = $derived(scope === 'project' ? settingsForm.projectId : null);
  const current = $derived(open ? `${open}:${settingsForm.page}` : settingsForm.page);

  /** The entries in their order, as the keyboard goes through them. */
  const entries = $derived<Entry[]>([
    ...APP.map((p) => ({ key: p.id, page: p.id })),
    ...app.projects.flatMap((p) => [
      { key: `project:${p.id}`, projectId: p.id },
      ...(p.id === open ? PROJECT.map((x) => ({ key: `${p.id}:${x.id}`, page: x.id, projectId: p.id })) : []),
    ]),
    { key: 'allProjects', page: 'allProjects' },
    { key: 'about', page: 'about' },
  ]);

  /**
   * The entry the keyboard is on: the page on screen, until an arrow moves it (the first entry when the page on screen
   * is not in the bar: a page of a project with no project open).
   */
  let focused = $state<string | null>(null);
  const shown = (key: string | null) => !!key && entries.some((e) => e.key === key);
  const tabbable = $derived(shown(focused) ? focused : shown(current) ? current : entries[0].key);
  let list = $state<HTMLElement>();

  /** A project of the bar opens on the same page as the one on screen, else on « Projet ». */
  function go(e: Entry) {
    if (e.projectId) settingsForm.projectId = e.projectId;
    settingsForm.page = e.page ?? (scope === 'project' ? settingsForm.page : 'project');
  }

  async function onKey(ev: KeyboardEvent) {
    const at = entries.findIndex((e) => e.key === (ev.target as HTMLElement).dataset.key);
    if (at < 0) return;
    const n = entries.length;
    const to: Record<string, number> = { ArrowDown: Math.min(at + 1, n - 1), ArrowUp: Math.max(at - 1, 0), Home: 0, End: n - 1 };
    const next = to[ev.key];
    if (next === undefined) return;
    ev.preventDefault();
    const target = entries[next];
    go(target);
    focused = target.key;
    await tick();
    list?.querySelector<HTMLElement>(`[data-key="${CSS.escape(target.key)}"]`)?.focus();
  }

  const all = pageInfo('allProjects');
  const about = pageInfo('about');
  const allChanged = $derived(settingsForm.changed('allProjects'));
  const aboutChanged = $derived(settingsForm.changed('about'));
  const projectChanged = (id: string) => PROJECT.some((x) => settingsForm.changed(x.id, id));
</script>

{#snippet dot(changed: boolean)}
  {#if changed}<span class="chg" use:tip={t('settings.modal.changed')} aria-hidden="true"></span>{/if}
{/snippet}

<nav class="s-nav" aria-label={t('common.settings')}>
  <label class="search" use:tip={t('settings.nav.search')}>
    <Icon name="search" size={14} />
    <input type="search" disabled placeholder={t('settings.nav.search')} aria-label={t('settings.nav.search')} />
    <kbd aria-hidden="true">{keyLabel('Ctrl+F')}</kbd>
  </label>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="entries" bind:this={list} onkeydown={onKey}>
    <span class="s-h">Escouade</span>
    {#each APP as p (p.id)}
      {@const changed = settingsForm.changed(p.id)}
      <button
        class="s-i"
        data-key={p.id}
        aria-current={current === p.id ? 'page' : undefined}
        aria-describedby={changed ? 'settings-changed' : undefined}
        class:changed
        tabindex={tabbable === p.id ? 0 : -1}
        onclick={() => go({ key: p.id, page: p.id })}><Icon name={p.icon} /><span class="lbl">{p.label}</span>{@render dot(changed)}</button
      >
    {/each}
    {#if app.projects.length}<span class="s-h">{t('settings.nav.projects')}</span>{/if}
    {#each app.projects as p (p.id)}
      {@const d = settingsForm.projects[p.id]}
      {@const key = `project:${p.id}`}
      {@const changed = p.id !== open && projectChanged(p.id)}
      <button
        class="s-i project"
        data-key={key}
        aria-expanded={p.id === open}
        aria-describedby={changed ? 'settings-changed' : undefined}
        class:changed
        tabindex={tabbable === key ? 0 : -1}
        onclick={() => go({ key, projectId: p.id })}
        ><i class="sq" style:--c={d?.color ?? p.color}></i><span class="lbl">{d?.name.trim() || p.name}</span
        >{#if d?.private ?? p.private}<span class="end"><PrivateMark /></span>{/if}{@render dot(changed)}</button
      >
      {#if p.id === open}
        <div class="subs" role="group" aria-label={d?.name.trim() || p.name}>
          {#each PROJECT as x (x.id)}
            {@const sub = `${p.id}:${x.id}`}
            {@const changed = settingsForm.changed(x.id, p.id)}
            <button
              class="s-i sub"
              data-key={sub}
              aria-current={current === sub ? 'page' : undefined}
              aria-describedby={changed ? 'settings-changed' : undefined}
              class:changed
              tabindex={tabbable === sub ? 0 : -1}
              onclick={() => go({ key: sub, page: x.id, projectId: p.id })}><span class="lbl">{x.label}</span>{@render dot(changed)}</button
            >
          {/each}
        </div>
      {/if}
    {/each}
    <button
      class="s-i"
      data-key="allProjects"
      aria-current={current === 'allProjects' ? 'page' : undefined}
      aria-describedby={allChanged ? 'settings-changed' : undefined}
      class:changed={allChanged}
      tabindex={tabbable === 'allProjects' ? 0 : -1}
      onclick={() => go({ key: 'allProjects', page: 'allProjects' })}
      ><Icon name={all.icon} /><span class="lbl">{all.label}</span>{@render dot(allChanged)}</button
    >
    <div class="bot">
      <button
        class="s-i"
        data-key="about"
        aria-current={current === 'about' ? 'page' : undefined}
        aria-describedby={aboutChanged ? 'settings-changed' : undefined}
        class:changed={aboutChanged}
        tabindex={tabbable === 'about' ? 0 : -1}
        onclick={() => go({ key: 'about', page: 'about' })}
        ><Icon name={about.icon} /><span class="lbl">{about.label}</span>{@render dot(aboutChanged)}<span class="ver" aria-hidden="true"
          >{shownVersion(app.version)}</span
        ></button
      >
    </div>
  </div>
  <span id="settings-changed" hidden>{t('settings.modal.changed')}</span>
</nav>

<style>
  /* design/settings-2-2/index.html, `.a-nav`: the search, then the entries on a side darker than the page. */
  .s-nav {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    padding: 14px 10px 12px;
    border-right: 1px solid var(--line);
    background: rgba(0, 0, 0, calc(0.2 * var(--shade)));
  }
  .search {
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin-bottom: 8px;
    padding: 0 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 12.5px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .search input::placeholder {
    color: var(--muted);
    opacity: 1;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  kbd {
    display: inline-flex;
    flex: none;
    align-items: center;
    height: 18px;
    padding: 0 5px;
    border: 1px solid var(--line-2);
    border-radius: 4px;
    color: var(--muted);
    font: 500 11px/1 var(--font);
  }
  .entries {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-height: 0;
    overflow: auto;
    scrollbar-width: thin;
  }
  .subs {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .s-h {
    padding: 12px 10px 4px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .s-i {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--soft);
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .s-i :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .s-i .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .s-i:focus-visible {
    outline-offset: -2px;
  }
  .s-i:hover {
    background: var(--hover);
    color: var(--text);
  }
  .s-i[aria-current='page'] {
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
  }
  .s-i[aria-current='page'] :global(.icon) {
    color: var(--text);
  }
  .s-i.sub {
    height: 28px;
    padding-left: 30px;
    font-size: 12.5px;
  }
  .sq {
    flex: none;
    width: 8px;
    height: 8px;
    margin: 0 4px;
    border-radius: 2.5px;
    background: var(--c);
  }
  .end {
    display: inline-flex;
    margin-left: auto;
  }
  .chg {
    flex: none;
    width: 6px;
    height: 6px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--p-ink);
  }
  .end + .chg {
    margin-left: 8px;
  }
  .bot {
    margin-top: auto;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .ver {
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .chg + .ver {
    margin-left: 8px;
  }
</style>

<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { t } from '../../lib/i18n';
  import { keyLabel, primaryKey } from '../../lib/platform';
  import { pageInfo, SETTINGS_PAGES, settingsForm, type SettingsPage } from '../../lib/settings.svelte';
  import { settingsAlerts } from '../../lib/settings-alerts.svelte';
  import { searchSettings, type SearchHit } from '../../lib/settings-search';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import { shownVersion } from '../../lib/updater';
  import Icon from '../Icon.svelte';
  import PrivateMark from '../PrivateMark.svelte';

  // The bar of the settings window (direction A, design/settings-2-2/index.html#a-*): the search, Escouade's pages, the
  // projects with the one chosen unfolded on its pages, « Tous les projets », and « À propos » with the version at its
  // foot. Up and down go from entry to entry, Home and End to the first and the last; going through a project opens it.
  // Nothing waits to be saved (ESC-380): no dot of a change. An amber dot says a page that has a problem, its reason in
  // the tooltip (R8, lib/settings-alerts.svelte.ts).
  //
  // The search (R8, lib/settings-search.ts; Ctrl+F in the window) shows its hits in place of the entries: up and down
  // from the field go through them, Enter or a click opens one (its page, « Avancé » unfolded, the row lit), Escape
  // empties the field.

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

  // The search.
  let query = $state('');
  let field = $state<HTMLInputElement>();
  const searching = $derived(!!query.trim());
  const hits = $derived(searchSettings(query, app.projects, settingsForm.projectId));
  /** The hit the keyboard is on, from the field. */
  let at = $state(0);
  $effect(() => {
    void query;
    at = 0;
  });
  let opened = 0;

  function openHit(h: SearchHit) {
    if (h.projectId) settingsForm.projectId = h.projectId;
    settingsForm.page = h.page;
    settingsForm.found = h.row ? { row: h.row, n: ++opened } : null;
  }

  function onSearchKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && query) {
      e.stopPropagation();
      query = '';
    } else if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && hits.length) {
      e.preventDefault();
      at = (at + (e.key === 'ArrowDown' ? 1 : hits.length - 1)) % hits.length;
      document.getElementById(`hit-${hits[at].key}`)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter' && hits[at]) {
      e.preventDefault();
      openHit(hits[at]);
    }
  }

  /** Ctrl+F (⌘F) anywhere in the window goes to the field. */
  function onWindowKey(e: KeyboardEvent) {
    if (primaryKey(e) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      field?.focus();
      field?.select();
    }
  }

  /** Where a hit is: its page (its project's name before it), « › Avancé » when it is folded there. */
  function whereOf(h: SearchHit): string {
    const label = pageInfo(h.page).label;
    const page = h.row?.advanced ? t('settings.nav.inAdvanced', { page: label }) : label;
    const p = h.projectId ? app.projects.find((x) => x.id === h.projectId) : null;
    return p ? `${settingsForm.projects[p.id]?.name.trim() || p.name} › ${page}` : page;
  }

  // The amber dots: what the window reads when it opens (the code hosts' accounts, each plugin's process), then the app.
  onMount(() => void settingsAlerts.load());
  const alerts = $derived(settingsAlerts.all);
  const reason = (key: string) => alerts[key]?.join('\n') ?? '';

  const all = pageInfo('allProjects');
  const about = pageInfo('about');
</script>

<svelte:window onkeydown={onWindowKey} />

<!-- The amber dot of an entry that has a problem; its reason is the entry's tooltip and description. -->
{#snippet dot(key: string)}
  {#if reason(key)}<span class="dotw" data-alert={key} aria-hidden="true"></span>{/if}
{/snippet}

<nav class="s-nav" aria-label={t('common.settings')}>
  <label class="search">
    <Icon name="search" size={14} />
    <input
      type="search"
      bind:this={field}
      bind:value={query}
      placeholder={t('settings.nav.search')}
      aria-label={t('settings.nav.search')}
      aria-controls={searching ? 'settings-hits' : undefined}
      aria-autocomplete="list"
      aria-activedescendant={searching && hits[at] ? `hit-${hits[at].key}` : undefined}
      onkeydown={onSearchKey}
    />
    {#if !query}<kbd aria-hidden="true">{keyLabel('Ctrl+F')}</kbd>{/if}
  </label>
  {#if searching}
    <div class="entries hits" id="settings-hits" role="listbox" aria-label={t('settings.nav.results')}>
      {#each hits as h, i (h.key)}
        <!-- Chosen from the field (its arrows and Enter) or by a click: the focus stays in the field. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="hit"
          id="hit-{h.key}"
          role="option"
          tabindex="-1"
          aria-selected={i === at}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => {
            at = i;
            openHit(h);
          }}
        >
          <span class="hl">{h.row?.label ?? pageInfo(h.page).label}</span>
          <span class="hw">{whereOf(h)}</span>
        </div>
      {:else}
        <p class="nohit" role="status">{t('settings.nav.none', { query: query.trim() })}</p>
      {/each}
    </div>
  {:else}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="entries" bind:this={list} onkeydown={onKey}>
      <span class="s-h">Escouade</span>
      {#each APP as p (p.id)}
        <button
          class="s-i"
          data-key={p.id}
          aria-current={current === p.id ? 'page' : undefined}
          tabindex={tabbable === p.id ? 0 : -1}
          data-autofocus={tabbable === p.id || undefined}
          use:tip={reason(p.id) || null}
          onclick={() => go({ key: p.id, page: p.id })}><Icon name={p.icon} /><span class="lbl">{p.label}</span>{@render dot(p.id)}</button
        >
      {/each}
      {#if app.projects.length}<span class="s-h">{t('settings.nav.projects')}</span>{/if}
      {#each app.projects as p (p.id)}
        {@const d = settingsForm.projects[p.id]}
        {@const key = `project:${p.id}`}
        <button
          class="s-i project"
          data-key={key}
          aria-expanded={p.id === open}
          tabindex={tabbable === key ? 0 : -1}
          data-autofocus={tabbable === key || undefined}
          use:tip={reason(key) || null}
          onclick={() => go({ key, projectId: p.id })}
          ><i class="sq" style:--c={d?.color ?? p.color}></i><span class="lbl">{d?.name.trim() || p.name}</span
          >{#if d?.private ?? p.private}<span class="end"><PrivateMark /></span>{/if}{@render dot(key)}</button
        >
        {#if p.id === open}
          <div class="subs" role="group" aria-label={d?.name.trim() || p.name}>
            {#each PROJECT as x (x.id)}
              {@const sub = `${p.id}:${x.id}`}
              <button
                class="s-i sub"
                data-key={sub}
                aria-current={current === sub ? 'page' : undefined}
                tabindex={tabbable === sub ? 0 : -1}
                data-autofocus={tabbable === sub || undefined}
                use:tip={reason(sub) || null}
                onclick={() => go({ key: sub, page: x.id, projectId: p.id })}><span class="lbl">{x.label}</span>{@render dot(sub)}</button
              >
            {/each}
          </div>
        {/if}
      {/each}
      <button
        class="s-i"
        data-key="allProjects"
        aria-current={current === 'allProjects' ? 'page' : undefined}
        tabindex={tabbable === 'allProjects' ? 0 : -1}
        data-autofocus={tabbable === 'allProjects' || undefined}
        onclick={() => go({ key: 'allProjects', page: 'allProjects' })}><Icon name={all.icon} /><span class="lbl">{all.label}</span></button
      >
      <div class="bot">
        <button
          class="s-i"
          data-key="about"
          aria-current={current === 'about' ? 'page' : undefined}
          tabindex={tabbable === 'about' ? 0 : -1}
          data-autofocus={tabbable === 'about' || undefined}
          onclick={() => go({ key: 'about', page: 'about' })}
          ><Icon name={about.icon} /><span class="lbl">{about.label}</span><span class="ver" aria-hidden="true"
            >{shownVersion(app.version)}</span
          ></button
        >
      </div>
    </div>
  {/if}
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
  /* `.dotw` of the preview: the amber of what waits, as on a project's tile. */
  .dotw {
    flex: none;
    width: 7px;
    height: 7px;
    margin-left: auto;
    border-radius: 50%;
    background: var(--wait);
  }
  .end + .dotw {
    margin-left: 8px;
  }
  /* The hits of the search, in place of the entries: the label, then where it is. */
  .hit {
    display: flex;
    flex: none;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px;
    border-radius: 8px;
    cursor: pointer;
  }
  .hit:hover {
    background: var(--hover);
  }
  .hit[aria-selected='true'] {
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .hl {
    overflow: hidden;
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hw {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nohit {
    margin: 0;
    padding: 8px 10px;
    color: var(--muted);
    font-size: 12.5px;
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
</style>

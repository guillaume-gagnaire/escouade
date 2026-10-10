<script lang="ts">
  import { tick } from 'svelte';
  import { trapFocus } from '../../lib/focus';
  import { fInt } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { importedLabel, issueKey, SERVICE_IDS, SERVICES } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { ExternalIssue, IssuePage, Service } from '../../lib/types';

  // « Importer des tickets »: the tickets of the project's linked sources, searched and filtered by
  // their service, chosen across sources, then put at the end of « À faire ».
  let { projectId }: { projectId: string } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  /** The linked sources whose account is connected, in the services' order. */
  const sources = $derived(
    SERVICE_IDS.filter(
      (id) => project?.integrations.links.some((l) => l.service === id) && app.accounts.find((a) => a.service === id)?.connected,
    ),
  );
  let chosen = $state<Service | null>(null);
  const current = $derived(chosen && sources.includes(chosen) ? chosen : (sources[0] ?? null));
  const link = $derived(current ? project?.integrations.links.find((l) => l.service === current) : undefined);
  const account = $derived(app.accounts.find((a) => a.service === current));

  let text = $state('');
  /** The text as searched: 300 ms after the last key. */
  let query = $state('');
  /** The chips turned on, by service. */
  let filters = $state<Partial<Record<Service, string[]>>>({});
  /** The search's pages so far, one after the other (« Afficher plus »). */
  let page = $state<IssuePage | null>(null);
  let loading = $state(false);
  /** The next page is on its way. */
  let more = $state(false);
  let list = $state<HTMLDivElement>();
  let failure = $state<string | null>(null);
  let selected = $state<Record<string, ExternalIssue>>({});
  let maxLoops = $state(5);
  let importing = $state(false);
  /** Only the last search's answer counts. */
  let asked = 0;

  $effect(() => {
    const typed = text;
    const timer = setTimeout(() => (query = typed.trim()), 300);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const service = current;
    if (!service) return;
    const q = query;
    const f = [...(filters[service] ?? [])];
    const n = ++asked;
    loading = true;
    more = false;
    failure = null;
    api
      .integrationIssues(projectId, service, q, f)
      .then((p) => {
        if (n !== asked) return;
        page = p;
        loading = false;
      })
      .catch((e) => {
        if (n !== asked) return;
        page = null;
        failure = String(e);
        loading = false;
      });
  });

  const issues = $derived(page?.issues ?? []);
  /**
   * « 50 affichés sur 312 » whenever the service counts more than are listed (on its last page too: GitHub's search
   * stops at 1 000); else « 50 affichés » while it has more, « 3 résultats » once all are there.
   */
  const counted = $derived.by(() => {
    if (loading) return t('integrations.modal.searching');
    const shown = issues.length;
    // Jira's count is approximate: never fewer than are listed.
    const total = page?.total != null ? Math.max(shown, page.total) : null;
    if (total != null && total > shown) return t('integrations.modal.shownOf', { count: shown, n: fInt(shown), total: fInt(total) });
    return page?.next
      ? t('integrations.modal.shown', { count: shown, n: fInt(shown) })
      : t('integrations.modal.results', { count: shown, n: fInt(shown) });
  });
  const selectable = $derived(issues.filter((i) => !i.imported));
  const allOn = $derived(selectable.length > 0 && selectable.every((i) => selected[issueKey(i)]));
  const picked = $derived(Object.values(selected));
  const extract = $derived(app.settings.integrations?.extractCriteria ?? true);

  function close() {
    app.modal = null;
  }

  function manage() {
    app.modal = { kind: 'settings', tab: 'integrations', projectId };
  }

  function pickSource(id: Service) {
    if (id === current) return;
    chosen = id;
    text = '';
    query = '';
    page = null;
  }

  /** « Afficher plus »: the search's next page, after the tickets already listed (and chosen). */
  async function showMore() {
    const service = current;
    const next = page?.next;
    if (!service || !next || more) return;
    // A new search meanwhile wins: this page would belong to the one before.
    const n = asked;
    more = true;
    try {
      const p = await api.integrationIssues(projectId, service, query, [...(filters[service] ?? [])], next);
      if (n !== asked || !page) return;
      const listed = new Set(page.issues.map(issueKey));
      const first = page.issues.length;
      page = {
        ...page,
        issues: [...page.issues, ...p.issues.filter((i) => !listed.has(issueKey(i)))],
        next: p.next,
        // That of the last page: Jira counts only while the list is cut (none on its last page,
        // where they are all there), GitHub's search on every page.
        total: p.total,
      };
      if (!p.next) {
        // Its button goes: the focus goes to the first ticket it brought, not to the window.
        await tick();
        list?.querySelectorAll<HTMLElement>('.row')[first]?.focus();
      }
    } catch (e) {
      if (n === asked) app.toast(String(e), 'error');
    } finally {
      if (n === asked) more = false;
    }
  }

  function toggleFilter(id: string) {
    if (!current) return;
    const on = filters[current] ?? [];
    filters[current] = on.includes(id) ? on.filter((f) => f !== id) : [...on, id];
  }

  function toggle(i: ExternalIssue) {
    if (i.imported) return;
    const k = issueKey(i);
    if (selected[k]) delete selected[k];
    else selected[k] = i;
  }

  function toggleAll() {
    const on = !allOn;
    for (const i of selectable) {
      if (on) selected[issueKey(i)] = i;
      else delete selected[issueKey(i)];
    }
  }

  async function doImport() {
    if (!picked.length || importing) return;
    importing = true;
    const made = await app.run(api.integrationImport(projectId, $state.snapshot(picked), maxLoops));
    importing = false;
    if (!made) return;
    for (const ticket of made) app.tickets[ticket.id] = ticket;
    close();
    if (made.length) app.toast(importedLabel(made.map((ticket) => ({ service: ticket.external?.service ?? 'jira' }))), 'ok');
    else app.toast(t('integrations.modal.allKnown'), 'info');
  }

  /** The color of a ticket's type, by its code: its name changes with the language. */
  function kindColor(code: string) {
    if (code === 'bug') return 'var(--del)';
    if (code === 'story') return 'var(--ok)';
    if (code === 'task') return 'oklch(0.74 0.12 235)';
    return 'var(--muted)';
  }

  const countOf = (id: Service) => picked.filter((i) => i.service === id).length;
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && close()} />

{#snippet badge(id: Service)}
  <span class="badge" style:background={SERVICES[id].color} style:color={SERVICES[id].ink} aria-hidden="true">{SERVICES[id].letter}</span>
{/snippet}

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={close}>
  <div
    class="modal"
    use:trapFocus
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-label={t('integrations.modal.title')}
  >
    <div class="head">
      <div class="hd">
        <span class="t">{t('integrations.modal.title')}</span>
        <span class="sub">{t('integrations.modal.sub', { project: project?.name ?? '' })}</span>
      </div>
      <div style="flex:1"></div>
      <button class="icon-btn" style="width:28px;height:28px;font-size:16px" onclick={close} aria-label={t('common.close')}>×</button>
    </div>
    {#if !current || !link}
      <div class="none">
        <span class="nt">{t('integrations.modal.noSource')}</span>
        <span class="nd">{t('integrations.modal.noSourceBody', { project: project?.name ?? t('integrations.modal.thisProject') })}</span>
        <button class="btn primary" onclick={manage}>{t('integrations.modal.linkSource')}</button>
      </div>
    {:else}
      <div class="sources" role="tablist" aria-label={t('integrations.modal.sources')}>
        {#each sources as id (id)}
          {@const l = project?.integrations.links.find((x) => x.service === id)}
          {@const n = countOf(id)}
          <button class="src" class:on={id === current} role="tab" aria-selected={id === current} onclick={() => pickSource(id)}>
            {@render badge(id)}
            <span class="sn"><span class="n">{SERVICES[id].name}</span><span class="c mono">{l?.name}</span></span>
            {#if n}<span class="count mono" aria-label={t('integrations.modal.pickedCount', { count: n, n: fInt(n) })}>{fInt(n)}</span>{/if}
          </button>
        {/each}
        <div style="flex:1"></div>
        <button class="manage" onclick={manage}>{t('integrations.modal.manage')}</button>
      </div>
      <div class="search">
        <div class="box">
          <span class="ic" aria-hidden="true">⌕</span>
          <input bind:value={text} placeholder={SERVICES[current].placeholder} spellcheck="false" aria-label={t('common.search')} />
        </div>
        {#if page?.filters.length}
          <div class="filters" role="group" aria-label={t('integrations.modal.filters')}>
            {#each page.filters as f (f.id)}
              {@const on = (filters[current] ?? []).includes(f.id)}
              <button class="chip" class:on aria-pressed={on} onclick={() => toggleFilter(f.id)}>{f.label}</button>
            {/each}
          </div>
        {/if}
      </div>
      <div class="bar">
        <button
          class="cb"
          class:on={allOn}
          role="checkbox"
          aria-checked={allOn}
          aria-label={t('integrations.modal.selectAll')}
          onclick={toggleAll}>{allOn ? '✓' : ''}</button
        >
        <span class="count-l">{counted}</span>
        <div style="flex:1"></div>
        <span class="ctx mono">{[account?.label.split(' · ')[0], link.name].filter(Boolean).join(' · ')}</span>
      </div>
      <div class="list" bind:this={list} aria-busy={loading || more}>
        {#if failure}
          <p class="empty error" role="alert">{failure}</p>
        {:else if !loading && !issues.length}
          <p class="empty">{t('integrations.modal.noMatch')}</p>
        {/if}
        {#each issues as i (issueKey(i))}
          {@const on = !!selected[issueKey(i)] || i.imported}
          <div
            class="row"
            class:on={!!selected[issueKey(i)]}
            class:imported={i.imported}
            role="checkbox"
            aria-checked={on}
            aria-disabled={i.imported}
            aria-label="{i.key} {i.title}"
            tabindex={i.imported ? -1 : 0}
            onclick={() => toggle(i)}
            onkeydown={(e) => {
              if (e.key === ' ' || e.key === 'Enter') {
                e.preventDefault();
                toggle(i);
              }
            }}
          >
            <span class="cb" class:on aria-hidden="true">{on ? '✓' : ''}</span>
            <div class="main">
              <div class="line"><span class="key mono">{i.key}</span><span class="title">{i.title}</span></div>
              <div class="meta">
                <span class="kind" style:color={kindColor(i.kindCode)}>{i.kind}</span>
                {#if i.meta.length}<span>{i.meta.join('  ·  ')}</span>{/if}
                {#if extract && i.criteria.length}
                  <span class="crit" title={i.criteria.join(' · ')}
                    >{t('integrations.modal.criteriaFound', { count: i.criteria.length })}</span
                  >
                {/if}
              </div>
            </div>
            {#if i.imported}<span class="already">{t('integrations.modal.alreadyImported')}</span>{/if}
          </div>
        {/each}
        {#if page?.next && !loading && !failure}
          <!-- Not disabled while the page comes: the focus stays on it (showMore asks once). -->
          <button class="btn ghost more" aria-disabled={more} onclick={showMore}>{t('integrations.modal.showMore')}</button>
        {/if}
      </div>
      <div class="foot">
        <span class="sel"
          >{picked.length
            ? t('integrations.modal.picked', { count: picked.length, n: fInt(picked.length) })
            : t('integrations.modal.nonePicked')}</span
        >
        <div style="flex:1"></div>
        <span class="ml">{t('integrations.modal.maxLoops')}</span>
        <div class="loops" role="group" aria-label={t('integrations.modal.maxLoops')}>
          {#each [3, 5, 8] as n (n)}
            <button class="mono" class:on={maxLoops === n} aria-pressed={maxLoops === n} onclick={() => (maxLoops = n)}>{n}</button>
          {/each}
        </div>
        <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
        <button class="btn primary" disabled={!picked.length || importing} onclick={doImport}
          >{picked.length
            ? t('integrations.modal.importCount', { count: picked.length, n: fInt(picked.length) })
            : t('common.import')}</button
        >
      </div>
    {/if}
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
    width: 860px;
    max-width: calc(100vw - 40px);
    height: 660px;
    max-height: calc(100vh - 60px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
    animation: ccFadeIn 0.15s ease-out;
  }
  .head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 20px 18px 14px 24px;
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
  .sub {
    font-size: 12.5px;
    color: var(--muted);
  }
  .none {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 40px;
    text-align: center;
  }
  .nt {
    font-size: 15px;
    font-weight: 700;
  }
  .nd {
    max-width: 380px;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--muted);
    text-wrap: pretty;
  }
  .none .btn {
    margin-top: 8px;
  }
  .badge {
    min-width: 20px;
    height: 20px;
    padding: 0 3px;
    flex: none;
    border-radius: 5px;
    font-size: 9px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .sources {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 24px 12px;
  }
  .src {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 12px;
    border: 1px solid transparent;
    border-radius: var(--r);
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  .src:hover {
    background: var(--elev);
  }
  .src.on {
    border-color: var(--line2);
    background: var(--elev);
    color: var(--text);
  }
  .sn {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
  }
  .sn .n {
    font-size: 13px;
    font-weight: 700;
  }
  .sn .c {
    font-size: 10px;
    color: var(--dim);
  }
  .count {
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 10.5px;
    font-weight: 700;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .manage {
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .manage:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .search {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 24px 12px;
    border-top: 1px solid var(--line);
  }
  .box {
    height: 38px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
  }
  .box:focus-within {
    border-color: var(--accent);
  }
  .box .ic {
    font-size: 14px;
    color: var(--dim);
  }
  .box input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13.5px;
  }
  .box input::placeholder {
    color: var(--dim);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    height: 26px;
    padding: 0 11px;
    border: 1px solid var(--line2);
    border-radius: 99px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 24px 8px 38px;
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
    background: var(--bg);
  }
  .cb {
    width: 16px;
    height: 16px;
    flex: none;
    padding: 0;
    border-radius: 4px;
    border: 1.5px solid var(--line2);
    background: transparent;
    color: var(--accent-ink);
    font-size: 11px;
    font-weight: 800;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .cb.on {
    border-color: var(--accent);
    background: var(--accent);
  }
  .count-l {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .ctx {
    font-size: 10.5px;
    color: var(--dim);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 6px 10px;
  }
  .empty {
    margin: 0;
    padding: 40px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  .empty.error {
    color: var(--del);
    overflow-wrap: anywhere;
  }
  .row {
    flex: none;
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 11px 14px 11px 28px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .row:hover {
    background: var(--elev);
  }
  .row.on {
    background: color-mix(in oklch, var(--accent) 8%, transparent);
  }
  .row.imported {
    opacity: 0.5;
    cursor: default;
  }
  .row .cb {
    margin-top: 2px;
    cursor: inherit;
  }
  .main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: 9px;
  }
  .key {
    flex: none;
    font-size: 11px;
    color: var(--dim);
  }
  .title {
    font-size: 13.5px;
    font-weight: 600;
    text-wrap: pretty;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .kind {
    font-size: 10.5px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 3px;
    background: var(--elev2);
  }
  .crit {
    color: var(--ok);
  }
  .more {
    flex: none;
    align-self: center;
    margin: 8px 0 6px;
  }
  .more[aria-disabled='true'] {
    opacity: 0.5;
    cursor: default;
  }
  .already {
    flex: none;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 99px;
    background: var(--elev2);
    color: var(--muted);
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px 16px 24px;
    border-top: 1px solid var(--line);
  }
  .sel {
    font-size: 12.5px;
    font-weight: 600;
  }
  .ml {
    font-size: 11.5px;
    color: var(--dim);
  }
  .loops {
    display: flex;
    gap: 2px;
    padding: 2px;
    border-radius: var(--r-sm);
    background: var(--bg);
    border: 1px solid var(--line);
  }
  .loops button {
    height: 24px;
    min-width: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .loops button.on {
    background: var(--elev2);
    color: var(--text);
  }
</style>

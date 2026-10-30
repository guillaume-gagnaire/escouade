<script lang="ts">
  import { locale, t } from '../../lib/i18n';
  import { CATEGORIES, textOf, type Category } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import { bestOf, categoriesOfCatalog, matches, sealed, type CatalogPlugin } from '../../lib/store';
  import Icon from '../Icon.svelte';
  import Seal from './Seal.svelte';
  import StoreRow from './StoreRow.svelte';
  import PluginTile from './PluginTile.svelte';

  // The store in the view Plugins (choices 215, 165, 174, 175): « Pour <projet> », three suggestions from what Escouade
  // sees on this machine and in the project open, each with its reason, nothing sent; the categories in chips; then
  // « Vérifiés par Escouade » and « Nouveaux » side by side, or, once searched or filtered, the lines found. `category`:
  // the sidebar's category, the chips then hidden.
  let { category = null }: { category?: Category | null } = $props();

  const lang = $derived(locale.ui);
  const view = $derived(plugins.store);
  let chip = $state<Category | null>(null);
  const cat = $derived(category ?? chip);
  const all = $derived(plugins.catalog);
  const found = $derived(
    all.filter(
      (p) =>
        matches(p, plugins.query, lang) &&
        (!cat || categoriesOfCatalog(p).includes(cat)) &&
        (!plugins.verifiedFilter || (view && sealed(bestOf(view, p)))),
    ),
  );
  const searching = $derived(!!plugins.query.trim() || !!cat || plugins.verifiedFilter);
  const verified = $derived(all.filter((p) => view && sealed(bestOf(view, p))).sort((a, b) => b.installs - a.installs));
  const newest = $derived(
    [...all].sort((a, b) => (bestOf(view!, b)?.published_at ?? '').localeCompare(bestOf(view!, a)?.published_at ?? '')),
  );
  const counts = $derived(
    Object.fromEntries(CATEGORIES.map((c) => [c, all.filter((p) => categoriesOfCatalog(p).includes(c)).length])) as Record<
      Category,
      number
    >,
  );
  let full = $state<'verified' | 'newest' | null>(null);
  const project = $derived(app.project);
  const hasCatalog = $derived(!!view?.catalog);
  const projectId = $derived(project?.id ?? null);
  $effect(() => {
    if (hasCatalog) void plugins.loadSuggestions(projectId);
  });
  const suggested = $derived(plugins.shownSuggestions);
</script>

{#if !view?.catalog}
  <div class="soon">
    <Icon name="sparkle" size={20} />
    <div>
      <b>{plugins.storeBusy ? t('plugins.store.reading') : t('plugins.store.unread')}</b>
      {#if view?.error}<p>{view.error}</p>{/if}
      <button class="btn" disabled={plugins.storeBusy} onclick={() => plugins.refreshStore()}
        >{plugins.storeBusy ? t('plugins.store.reading') : t('plugins.store.refresh')}</button
      >
    </div>
  </div>
{:else}
  {#if view.error}<p class="stale" role="status"><Icon name="info" size={14} /><span>{view.error}</span></p>{/if}
  {#if suggested.length && project && !category}
    <section class="for" aria-label={t('plugins.store.for', { project: project.name })}>
      <h4>{t('plugins.store.for', { project: project.name })} <span>{t('plugins.store.forSub')}</span></h4>
      <div class="cards3">
        {#each suggested as s (s.id)}
          {@const p = plugins.catalogPlugin(s.id)!}
          {@const best = bestOf(view, p)}
          <div class="sug">
            <PluginTile id={p.id} name={textOf(p.name, lang) || p.id} src={null} />
            <div>
              <button class="nm" onclick={() => plugins.openStorePlugin(p.id)}><b>{textOf(p.name, lang) || p.id}</b></button>
              {#if best && sealed(best)}<Seal version={best.version} />{/if}
              <div class="why">{s.reason}</div>
              <div class="later">
                <button class="lnk" onclick={() => plugins.dismiss(s.id, false)}>{t('plugins.store.notNow')}</button>
                <button class="lnk" onclick={() => plugins.dismiss(s.id, true)}>{t('plugins.store.neverSuggest')}</button>
              </div>
            </div>
            <button class="btn" onclick={() => plugins.fromStore(p.id)}>{t('plugins.store.install')}</button>
          </div>
        {/each}
      </div>
    </section>
  {/if}
  {#if !category}
    <div class="cats" role="group" aria-label={t('plugins.nav.categories')}>
      <button class:on={!chip} aria-pressed={!chip} onclick={() => (chip = null)}>{t('plugins.store.all')}<em>{all.length}</em></button>
      {#each CATEGORIES as c (c)}
        <button class:on={chip === c} aria-pressed={chip === c} onclick={() => (chip = chip === c ? null : c)}
          >{t(`plugins.store.cat.${c}`)}<em>{counts[c]}</em></button
        >
      {/each}
    </div>
  {/if}
  {#if searching || full}
    {@const rows = full === 'verified' ? verified : full === 'newest' ? newest : found}
    <div class="sec">
      <h3>
        {full ? t(`plugins.store.${full}`) : t('plugins.store.results', { count: rows.length })}
        {#if full}<button class="lnk" onclick={() => (full = null)}>{t('plugins.store.back')}</button>{/if}
      </h3>
      {#each rows as p (p.id)}<StoreRow {p} />{:else}<p class="hint">{t('plugins.store.none')}</p>{/each}
    </div>
  {:else}
    <div class="two">
      {#each [['verified', verified], ['newest', newest]] as const as [k, rows] (k)}
        <div class="sec">
          <h3>{t(`plugins.store.${k}`)} <button class="lnk" onclick={() => (full = k)}>{t('plugins.store.seeAll')}</button></h3>
          {#each (rows as CatalogPlugin[]).slice(0, 4) as p (p.id)}<StoreRow {p} />{:else}<p class="hint">
              {t('plugins.store.none')}
            </p>{/each}
        </div>
      {/each}
    </div>
  {/if}
{/if}

<style>
  .soon {
    margin-top: 14px;
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr);
    gap: 12px;
    max-width: 640px;
    padding: 16px 18px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .soon b {
    font-weight: 600;
  }
  .soon p {
    margin: 6px 0 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .soon .btn {
    margin-top: 12px;
  }
  .stale {
    margin: 6px 0 0;
    display: flex;
    gap: 8px;
    align-items: center;
    color: var(--warn-ink);
    font-size: 12px;
  }
  .for {
    margin-top: 14px;
    border-radius: 11px;
    padding: 14px 16px;
    background: linear-gradient(120deg, oklch(from var(--brand) l c h / 0.1), transparent 60%);
    box-shadow: inset 0 0 0 1px oklch(from var(--brand) l c h / 0.3);
    display: grid;
    gap: 10px;
  }
  .for h4 {
    margin: 0;
    font-size: 13px;
    font-weight: 650;
  }
  .for h4 span {
    color: var(--muted);
    font-weight: 450;
  }
  .cards3 {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }
  @container plugins (max-width: 860px) {
    .cards3 {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .sug {
    border-radius: 9px;
    padding: 11px 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
  }
  .sug > div {
    min-width: 0;
  }
  .sug .why {
    color: var(--muted);
    font-size: 11.5px;
  }
  .later {
    display: flex;
    gap: 10px;
  }
  .nm {
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .lnk {
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 11.5px;
    font-weight: 450;
    cursor: pointer;
  }
  .lnk:hover {
    color: var(--text);
  }
  .cats {
    margin-top: 14px;
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .cats button {
    padding: 4px 10px;
    border: none;
    border-radius: 7px;
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .cats button.on {
    background: var(--text);
    color: var(--frame);
    box-shadow: none;
  }
  .cats em {
    font-style: normal;
    color: var(--muted);
    margin-left: 4px;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 0 28px;
  }
  @container plugins (max-width: 860px) {
    .two {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .sec h3 {
    margin: 22px 0 4px;
    font-size: 13.5px;
    font-weight: 700;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .hint {
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
</style>

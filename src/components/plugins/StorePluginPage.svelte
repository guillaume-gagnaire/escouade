<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { fDate } from '../../lib/format';
  import { locale, t } from '../../lib/i18n';
  import { asksOf, textOf } from '../../lib/plugins';
  import { plugins, type StoreTab } from '../../lib/plugins-view.svelte';
  import { bestOf, categoriesOfCatalog, count, sealed, type CatalogPlugin, type CatalogVersion } from '../../lib/store';
  import Icon from '../Icon.svelte';
  import AskList from './AskList.svelte';
  import PluginTile from './PluginTile.svelte';
  import Seal from './Seal.svelte';
  import { tip } from '../../lib/tip';

  // The page of a plugin of the store, not installed (choices 176 and 216): its tile, its name, its seal, who made it,
  // its installations, its licence, its id, and « Installer » with what the version is worth; then Présentation,
  // Permissions, Versions (each with its seal, taken back or not); on the right what it can do, the review's summary,
  // where it comes from and « Signaler sur le store ». Installing goes through the install window, which checks it all.
  let { p, tab }: { p: CatalogPlugin; tab: StoreTab } = $props();

  const TABS: StoreTab[] = ['overview', 'permissions', 'versions'];
  const lang = $derived(locale.ui);
  const view = $derived(plugins.store!);
  const best = $derived(bestOf(view, p));
  const name = $derived(textOf(p.name, lang) || p.id);
  const category = $derived(categoriesOfCatalog(p)[0]);
  /** What a version asks for, as the core's words (`plugins::asks`), the strongest first. */
  const words = (v: CatalogVersion | null): string[] => {
    if (!v) return [];
    const x = v.permissions ?? {};
    return [
      ...(v.run ? ['run'] : []),
      ...(x.decide ? ['decide'] : []),
      ...(x.commands ?? []).map((w) => `commands:${w}`),
      ...(x.events ?? []).map((w) => `events:${w}`),
      ...(x.network ?? []).map((w) => `network:${w}`),
      ...(x.secrets ?? []).map((w) => `secrets:${w}`),
    ];
  };
  const asks = $derived(asksOf(words(best), null, lang));
  const summary = $derived(best?.review?.summary ? (lang === 'fr' ? best.review.summary.fr : best.review.summary.en) : null);
  const unverifiedWhy = $derived(best?.unverified ? (lang === 'fr' ? best.unverified.fr : best.unverified.en) : null);
  const storePage = $derived(`${view.url}${lang === 'fr' ? '' : '/en'}/plugins/${encodeURIComponent(p.id)}`);
</script>

<div class="page">
  <div class="phead">
    <button class="back" onclick={() => plugins.show({ kind: 'store' })}
      ><Icon name="down" size={12} />{t('plugins.store.title')}{#if category}<span> · {t(`plugins.cat.${category}`)}</span>{/if}</button
    >
    <div class="top">
      <PluginTile id={p.id} {name} src={null} size={64} />
      <div class="who">
        <h2>
          {name}
          {#if best && sealed(best)}<Seal version={best.version} long />{:else}<span class="ptag">{t('plugins.tag.unverified')}</span>{/if}
        </h2>
        <div class="meta">
          {#if p.author}<span class="soft">{p.author}</span>{/if}
          {#if category}<span>{t(`plugins.cat.${category}`)}</span>{/if}
          <span>{t('plugins.store.installs', { count: p.installs, n: count(p.installs, lang) })}</span>
          {#if p.license}<span>{p.license}</span>{/if}
          <span class="mono">{p.id}</span>
        </div>
        <p class="desc">{textOf(p.description, lang) || t('plugins.page.noDescription')}</p>
      </div>
      <div class="act">
        {#if best}
          <button class="btn primary" onclick={() => plugins.fromStore(p.id)}
            >{t('plugins.store.installVersion', { version: best.version })}</button
          >
          <small class="note" class:upd={!sealed(best)}
            >{sealed(best) ? t('plugins.store.verifiedNote') : t('plugins.store.unverifiedNote', { version: best.version })}</small
          >
        {:else}
          <small class="note upd">{t('plugins.store.noCompatible')}</small>
        {/if}
      </div>
    </div>
    {#if unverifiedWhy}<p class="reason" role="status">{t('plugins.withdrawn.unverifiedStore', { reason: unverifiedWhy })}</p>{/if}
  </div>
  <div class="tabs" role="tablist" aria-label={t('plugins.page.tabs.label')}>
    {#each TABS as k (k)}
      <button
        role="tab"
        aria-selected={tab === k}
        class:on={tab === k}
        onclick={() => plugins.show({ kind: 'storePlugin', id: p.id, tab: k })}
        >{t(`plugins.page.tabs.${k}`)}{#if k === 'permissions' && asks.length}<em>{asks.length}</em>{/if}{#if k === 'versions'}<em
            >{p.versions.length}</em
          >{/if}</button
      >
    {/each}
  </div>
  <div class="pgrid">
    <div class="main" role="tabpanel" aria-label={t(`plugins.page.tabs.${tab}`)}>
      {#if tab === 'overview'}
        <section class="box">
          <h5>{t('plugins.store.about')}</h5>
          <p class="text">{textOf(p.description, lang) || t('plugins.page.noDescription')}</p>
          <button class="btn lnkb" onclick={() => openUrl(storePage)}><Icon name="ext" size={14} />{t('plugins.store.readme')}</button>
        </section>
      {:else if tab === 'permissions'}
        <p class="hint">{t('plugins.store.permsIntro')}</p>
        {#if asks.length}<div class="box"><AskList {asks} /></div>{:else}<p class="hint">{t('plugins.perms.none')}</p>{/if}
      {:else}
        <div class="vers">
          {#each p.versions as v (v.version)}
            <div>
              <span class="vn mono">{v.version}</span>
              <span class="vd"
                >{fDate(Date.parse(v.published_at))}{#if v.yanked}
                  · {t('plugins.store.yanked')}{/if}{#if v.version === best?.version}
                  · {t('plugins.store.forThis')}{/if}</span
              >
              {#if sealed(v)}<Seal version={v.version} bare />{:else}<span class="ptag">{t('plugins.tag.unverified')}</span>{/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
    <aside class="pp-aside">
      <section class="box">
        <h5>{t('plugins.page.canDo')}</h5>
        {#if asks.length}<AskList {asks} />{:else}<p class="hint">{t('plugins.perms.none')}</p>{/if}
      </section>
      {#if best && sealed(best)}
        <section class="box review">
          <p>
            <b>{t('plugins.store.reviewed', { date: fDate(Date.parse(best.review?.signed_at ?? best.published_at)) })}</b>
            {summary ?? ''}
          </p>
        </section>
      {/if}
      <section class="box">
        <h5>{t('plugins.page.from')}</h5>
        <div class="chain">
          <span class="k">{t('plugins.origin.repo')}</span><span class="v mono" use:tip={{ text: p.repository, aria: false }}
            >{p.repository.replace(/^https:\/\/github\.com\//, '')}</span
          >
          {#if best}
            <span class="k">{t('plugins.origin.fingerprint')}</span><span class="v mono" use:tip={{ text: best.sha256, aria: false }}
              >{best.sha256.slice(0, 4)}…{best.sha256.slice(-4)}</span
            >
          {/if}
          <span class="k">{t('plugins.origin.signature')}</span>{#if best && sealed(best)}<span class="v"
              >{t('plugins.origin.checkedAtInstall')}</span
            >{:else}<span class="v no">{t('plugins.origin.storeUnsigned')}</span>{/if}
        </div>
      </section>
      <div class="btns">
        <button class="btn" onclick={() => openUrl(storePage)}>{t('plugins.store.report')}</button>
      </div>
    </aside>
  </div>
</div>

<style>
  .page {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .phead {
    padding: 16px 26px 12px;
    display: grid;
    gap: 10px;
  }
  .back {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .back :global(.icon) {
    transform: rotate(90deg);
  }
  .back:hover {
    color: var(--text);
  }
  .top {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: start;
  }
  h2 {
    margin: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 19px;
    font-weight: 650;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin-top: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .meta .soft {
    color: var(--soft);
  }
  .meta .mono {
    font-size: 11px;
  }
  .desc {
    margin: 8px 0 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .act {
    display: grid;
    gap: 6px;
    justify-items: end;
  }
  .note {
    max-width: 300px;
    text-align: right;
    color: var(--muted);
    font-size: 11.5px;
  }
  .note.upd {
    color: var(--warn-ink);
  }
  @container plugins (max-width: 900px) {
    .top {
      grid-template-columns: 64px minmax(0, 1fr);
    }
    .act {
      grid-column: 2;
      justify-items: start;
    }
    .note {
      max-width: none;
      text-align: left;
    }
  }
  .reason {
    margin: 0;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
  }
  .ptag {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 26px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    padding: 9px 12px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--muted);
    font: inherit;
    font-weight: 550;
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--p);
  }
  .tabs em {
    margin-left: 5px;
    font-style: normal;
    font-weight: 450;
    color: var(--muted);
  }
  .pgrid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 26px;
    padding: 18px 26px 22px;
    align-items: start;
  }
  @container plugins (max-width: 900px) {
    .pgrid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .main {
    display: grid;
    gap: 14px;
    align-content: start;
    min-width: 0;
  }
  .pp-aside {
    display: grid;
    gap: 12px;
  }
  .box {
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 12px 14px;
    display: grid;
    gap: 10px;
  }
  .box.review {
    background: color-mix(in oklch, var(--brand) 7%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 35%, transparent);
  }
  .box.review p,
  .text {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .lnkb {
    justify-self: start;
  }
  h5 {
    margin: 0;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    color: var(--muted);
    text-transform: uppercase;
  }
  .chain {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 6px 12px;
    font-size: 12px;
  }
  .chain .k {
    color: var(--dim);
  }
  .chain .v {
    color: var(--soft);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
  }
  .chain .v.no {
    color: var(--warn-ink);
    font-size: 12px;
  }
  .btns {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .vers {
    display: grid;
  }
  .vers > div {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 9px 0;
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
  }
  .vn {
    min-width: 60px;
    font-weight: 600;
  }
  .vd {
    color: var(--muted);
    flex: 1;
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
</style>

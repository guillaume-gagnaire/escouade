<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { categoriesOf, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import Icon from '../Icon.svelte';
  import InstallModal from './InstallModal.svelte';
  import PluginPage from './PluginPage.svelte';
  import PluginRows from './PluginRows.svelte';
  import PluginTile from './PluginTile.svelte';
  import StoreHome from './StoreHome.svelte';
  import StorePluginPage from './StorePluginPage.svelte';
  import { app } from '../../lib/state.svelte';
  import { fTime } from '../../lib/format';

  // The view Plugins (PLG-4; choices 213 to 218, design/plugins/): opened by its icon of the projects' rail, in place of a
  // project, its sidebar beside it (PluginsSide). Its pages on the work's panel; the install window over it.
  onMount(() => {
    void plugins.load();
    void plugins.loadStore();
  });

  const page = $derived(plugins.page);
  const builtin = (p: PluginView) => p.id.startsWith('dev.escouade.');
  const added = $derived(plugins.list.filter((p) => !builtin(p)));
  const shipped = $derived(plugins.list.filter(builtin));
  const current = $derived(page.kind === 'plugin' ? plugins.byId(page.id) : null);
  const listing = $derived(page.kind === 'storePlugin' ? plugins.catalogPlugin(page.id) : null);
  /** The versions Escouade withdrew, in a red band at the head of the installed ones (choices 180 and 218). */
  const withdrawn = $derived(plugins.list.filter((p) => p.withdrawn?.action === 'disable'));
  const readAt = $derived(plugins.store?.readAt ? fTime(Date.parse(plugins.store.readAt)) : null);
  const list = $derived(plugins.store?.revocations ?? null);
  let search = $state<HTMLInputElement | null>(null);
  function keys(e: KeyboardEvent) {
    if (page.kind === 'store' && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      search?.focus();
    }
  }
</script>

<svelte:window onkeydown={keys} />

<div class="plugins-work">
  {#if page.kind === 'plugin'}
    {#if current}
      <PluginPage plugin={current} tab={page.tab} />
    {:else if plugins.loaded}
      <div class="empty"><p>{t('plugins.page.gone')}</p></div>
    {/if}
  {:else if page.kind === 'storePlugin'}
    {#if listing && plugins.store}
      <StorePluginPage p={listing} tab={page.tab} />
    {:else}
      <div class="empty"><p>{t('plugins.store.gone')}</p></div>
    {/if}
  {:else}
    <div class="pane">
      <div class="head">
        <div class="row">
          {#if page.kind === 'store'}
            <h2>{t('plugins.store.title')}</h2>
            <span class="sub"
              >{readAt
                ? t('plugins.store.read', { time: readAt })
                : plugins.store?.catalog
                  ? t('plugins.store.kept')
                  : t('plugins.store.sub')}</span
            >
          {:else if page.kind === 'installed'}
            <h2>{t('plugins.installed.title')}</h2>
            <span class="sub"
              >{shipped.length
                ? t('plugins.installed.sub', { count: plugins.list.length, added: String(added.length), builtin: String(shipped.length) })
                : t('plugins.installed.subAdded', { count: added.length })}</span
            >
          {:else if page.kind === 'updates'}
            <h2>{t('plugins.updates.title')}</h2>
            <span class="sub">{t('plugins.updates.sub')}</span>
          {:else if page.kind === 'dev'}
            <h2>{t('plugins.dev.title')}</h2>
            <span class="sub">{t('plugins.dev.sub')}</span>
          {:else}
            <h2>{t(`plugins.cat.${page.category}`)}</h2>
          {/if}
          <span class="grow"></span>
          {#if page.kind === 'installed'}
            <label class="tog"
              ><button
                class="switch"
                class:on={!!app.settings.pluginsVerifiedOnly}
                role="switch"
                aria-checked={!!app.settings.pluginsVerifiedOnly}
                aria-label={t('plugins.installed.verifiedOnly')}
                onclick={() => plugins.setVerifiedOnly(!app.settings.pluginsVerifiedOnly)}
              ></button>{t('plugins.installed.verifiedOnly')}</label
            >
          {/if}
          {#if page.kind === 'installed' && plugins.updates.length}
            <button class="btn warn" onclick={() => plugins.updateAll()}
              >{t('plugins.installed.updateAll', { count: plugins.updates.length })}</button
            >
          {/if}
          {#if page.kind === 'updates'}
            <button class="btn" disabled={plugins.checking} onclick={() => plugins.checkUpdates()}
              >{plugins.checking ? t('plugins.updates.checking') : t('plugins.updates.check')}</button
            >
          {:else if page.kind === 'dev'}
            <button class="btn" onclick={() => plugins.fromFolder(true)}><Icon name="folder" size={14} />{t('plugins.loadDev')}</button>
          {:else}
            <button class="btn" onclick={() => plugins.fromFolder()}><Icon name="folder" size={14} />{t('plugins.fromFolder')}</button>
            <button class="btn" onclick={() => plugins.fromGit()}><Icon name="branch" size={14} />{t('plugins.fromGit')}</button>
          {/if}
        </div>
        {#if page.kind === 'store'}
          <div class="row">
            <label class="sbox"
              ><Icon name="search" size={14} /><input
                bind:this={search}
                bind:value={plugins.query}
                type="search"
                spellcheck="false"
                placeholder={t('plugins.store.search', { count: plugins.catalog.length })}
                aria-label={t('plugins.store.searchLabel')}
              /><span class="kbd">{t('plugins.store.searchKey')}</span></label
            >
            <label class="tog"
              ><button
                class="switch"
                class:on={plugins.verifiedFilter}
                role="switch"
                aria-checked={plugins.verifiedFilter}
                aria-label={t('plugins.store.verifiedOnly')}
                onclick={() => (plugins.verifiedFilter = !plugins.verifiedFilter)}
              ></button>{t('plugins.store.verifiedOnly')}</label
            >
          </div>
        {/if}
        {#if plugins.error}<p class="err" role="alert">{plugins.error}</p>{/if}
      </div>
      <div class="body">
        {#if page.kind === 'store'}
          <StoreHome />
        {:else if page.kind === 'installed'}
          {#each withdrawn as w (w.id)}
            <div class="band" role="alert">
              <span
                ><b>{t('plugins.withdrawn.title', { name: w.name, version: w.withdrawn!.version, date: w.withdrawn!.date })}</b>
                {w.withdrawn!.reason}
                {t('plugins.withdrawn.kept')}</span
              >
              <button class="btn" onclick={() => plugins.openPlugin(w.id)}>{t('plugins.page.uninstall')}</button>
            </div>
          {/each}
          {#if !plugins.list.length && plugins.loaded}
            <div class="soon">
              <Icon name="puzzle" size={20} />
              <div>
                <b>{t('plugins.installed.empty')}</b>
                <p>{t('plugins.installed.emptyHint')}</p>
              </div>
            </div>
          {:else}
            {#if added.length}<PluginRows rows={added} group={t('plugins.installed.added')} />{/if}
            {#if shipped.length}<PluginRows
                rows={shipped}
                group={t('plugins.installed.builtin')}
                note={t('plugins.installed.builtinNote')}
              />{/if}
          {/if}
          <p class="hint list">
            {list?.serial
              ? t('plugins.installed.revocations', {
                  time: list.readAt ? fTime(Date.parse(list.readAt)) : '—',
                  serial: String(list.serial),
                })
              : t('plugins.installed.revocationsNone')}
            {#if list?.error}<span class="warn">{list.error}</span>{/if}
            <button class="lnk" disabled={plugins.storeBusy} onclick={() => plugins.refreshStore()}
              >{plugins.storeBusy ? t('plugins.store.reading') : t('plugins.installed.readAgain')}</button
            >
          </p>
        {:else if page.kind === 'updates'}
          {#if plugins.updates.length}
            <div class="ups">
              {#each plugins.updates as p (p.id)}
                <div class="up">
                  <PluginTile id={p.id} name={p.name} installed />
                  <div>
                    <b>{p.name}</b><span class="mono">{t('plugins.updates.row', { from: p.version, to: p.update?.version ?? '' })}</span>
                  </div>
                  <button class="btn" onclick={() => plugins.openPlugin(p.id, 'versions')}>{t('plugins.page.tabs.versions')}</button>
                  <button class="btn primary" onclick={() => plugins.update(p)}>{t('plugins.updates.update')}</button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="hint">{t('plugins.updates.empty')}</p>
          {/if}
        {:else if page.kind === 'dev'}
          {#if plugins.dev.length}
            <PluginRows rows={plugins.dev} />
          {:else}
            <div class="soon">
              <Icon name="code" size={20} />
              <div>
                <b>{t('plugins.dev.empty')}</b>
                <p>{t('plugins.dev.emptyHint')}</p>
              </div>
            </div>
          {/if}
        {:else}
          {@const rows = plugins.list.filter((p) => categoriesOf(p).includes(page.category))}
          {#if plugins.store?.catalog}<StoreHome category={page.category} />{/if}
          {#if rows.length}<PluginRows {rows} group={t('plugins.nav.installed')} />{:else if !plugins.store?.catalog}<p class="hint">
              {t('plugins.installed.categoryEmpty')}
            </p>{/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if plugins.install}<InstallModal step={plugins.install} />{/if}

<style>
  .plugins-work {
    flex: 1;
    min-width: 0;
    display: flex;
    container: plugins / inline-size;
  }
  .pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .head {
    padding: 18px 26px 14px;
    border-bottom: 1px solid var(--line);
    display: grid;
    gap: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 12px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 650;
  }
  .sub {
    color: var(--muted);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  .btn.warn {
    color: var(--warn-ink);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .body {
    padding: 8px 26px 22px;
    display: grid;
    gap: 4px;
    align-content: start;
  }
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
  .soon :global(.icon) {
    color: var(--muted);
  }
  .soon b {
    font-weight: 600;
  }
  .soon p {
    margin: 6px 0 12px;
    color: var(--soft);
    font-size: 12.5px;
  }
  .ups {
    display: grid;
  }
  .up {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .up b {
    display: block;
    font-weight: 600;
  }
  .up .mono {
    color: var(--warn-ink);
    font-size: 11.5px;
  }
  .hint {
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .tog {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
    font-size: 12px;
    white-space: nowrap;
  }
  .sbox {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    height: 34px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .sbox input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    outline: none;
    color: var(--text);
    font: inherit;
    font-size: 13px;
  }
  .sbox:focus-within {
    box-shadow: inset 0 0 0 1px var(--p);
  }
  .kbd {
    font-size: 10.5px;
    padding: 1px 5px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
  }
  .band {
    margin-top: 10px;
    display: flex;
    gap: 14px;
    align-items: center;
    padding: 10px 14px;
    border-radius: 10px;
    background: color-mix(in oklch, var(--del) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 45%, transparent);
    color: var(--soft);
    font-size: 12.5px;
  }
  .band span {
    flex: 1;
  }
  .band b {
    color: var(--del);
  }
  .hint.list {
    margin-top: 18px;
    font-size: 11.5px;
  }
  .lnk {
    margin-left: 8px;
    padding: 0;
    border: none;
    background: none;
    color: var(--soft);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .warn {
    color: var(--warn-ink);
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
</style>

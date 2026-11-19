<script lang="ts">
  // « Un dossier du serveur » (ESC-538, docs/SERVER-UX.md § 4, design `*-projet`): the folders of the server the window
  // is on, served by its core (`fs_browse`), never this computer's. Each line says whether it is a repository and its
  // branch, or a project already (greyed: not chosen twice); the foot says the room left on the server's disk. A click
  // chooses a folder, a double click (or Entrée on it) goes into it.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { diskText, entryNote, type BrowseEntry, type BrowseListing } from '../../lib/server-browse';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  let {
    selected = '',
    onpick,
    onlisting,
  }: {
    selected?: string;
    onpick: (path: string) => void;
    /** The folder shown, each time it changes (the clone goes in it). */
    onlisting?: (l: BrowseListing) => void;
  } = $props();

  let listing = $state<BrowseListing | null>(null);
  let error = $state('');
  let loading = $state(false);
  const name = $derived(connection.server?.name ?? '');

  async function load(path?: string) {
    loading = true;
    error = '';
    try {
      listing = await api.fsBrowse(path);
      onlisting?.(listing);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
  void load();

  function pick(e: BrowseEntry) {
    if (!e.project) onpick(e.path);
  }

  function key(ev: KeyboardEvent, e: BrowseEntry) {
    if (ev.key === 'Enter') {
      ev.preventDefault();
      void load(e.path);
    }
  }

  /** The path's parent, then its last folder, as the head shows it: « /home/escouade/ projets ». */
  const head = $derived.by(() => {
    const p = listing?.path ?? '';
    const cut = Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\'));
    return cut >= 0 ? { dir: p.slice(0, cut + 1), last: p.slice(cut + 1) } : { dir: '', last: p };
  });
</script>

<div class="browser" data-testid="server-folders">
  <div class="head">
    <Icon name="server" size={12} />
    <span class="mono where" use:tip={{ text: listing?.path ?? '', cut: true }}>{name} : {head.dir}<b>{head.last}</b></span>
    <span class="sp"></span>
    <button
      class="btn small ghost"
      aria-label={t('moves.browse.up')}
      disabled={!listing?.parent || loading}
      onclick={() => listing?.parent && load(listing.parent)}><Icon name="up" size={12} /></button
    >
    <button class="btn small ghost" aria-label={t('moves.browse.home')} disabled={loading} onclick={() => listing && load(listing.home)}
      ><Icon name="user" size={12} /></button
    >
    {#if listing && !listing.project}
      <button class="btn small" onclick={() => listing && onpick(listing.path)}>{t('moves.browse.choose')}</button>
    {/if}
  </div>
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if !listing}
    <p class="note">{t('moves.browse.loading')}</p>
  {:else}
    <ul class="list" aria-label={t('moves.browse.list', { path: listing.path })}>
      {#each listing.entries as e (e.path)}
        <li>
          <button
            class="entry"
            class:on={selected === e.path}
            class:taken={!!e.project}
            aria-pressed={selected === e.path}
            aria-disabled={!!e.project}
            use:tip={e.project ? t('moves.browse.projectOf', { name: e.project }) : { text: e.path, cut: true }}
            onclick={() => pick(e)}
            ondblclick={() => load(e.path)}
            onkeydown={(ev) => key(ev, e)}
          >
            <Icon name="folder" size={13} />
            <span class="nm">{e.name}</span>
            <span class="sp"></span>
            {#if e.project}<span class="chip-sm">{t('moves.browse.project')}</span>{/if}
            <span class="note">{entryNote(e)}</span>
          </button>
          <button class="go" aria-label={t('moves.browse.open', { name: e.name })} onclick={() => load(e.path)}
            ><Icon name="right" size={12} /></button
          >
        </li>
      {:else}
        <li class="note empty">{t('moves.browse.empty')}</li>
      {/each}
    </ul>
    {#if listing.more}<p class="note">{t('moves.browse.more', { count: listing.more })}</p>{/if}
  {/if}
  <div class="foot">
    {#if listing && diskText(listing)}<span><Icon name="info" size={12} />{diskText(listing)}</span>{/if}
    <span class="sp"></span>
    {#if selected}<span class="mono chosen" use:tip={{ text: selected, cut: true }}>{selected}</span>{/if}
  </div>
</div>

<style>
  /* A column held to its height: the list scrolls between the head and the foot. */
  .browser {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 260px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: var(--well);
    overflow: hidden;
  }
  .head,
  .foot {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 6px 8px 6px 12px;
    font-size: 12px;
    color: var(--muted);
  }
  .head {
    border-bottom: 1px solid var(--line);
  }
  .foot {
    border-top: 1px solid var(--line);
    font-size: 11px;
  }
  .foot span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
  }
  .where,
  .chosen {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }
  .where b {
    color: var(--text);
  }
  .sp {
    flex: 1;
  }
  .list {
    margin: 0;
    padding: 2px 0;
    list-style: none;
    overflow-y: auto;
    flex: 1 1 auto;
    min-height: 0;
  }
  .head,
  .foot {
    flex: none;
  }
  .list li {
    display: flex;
    align-items: center;
  }
  .entry {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 7px 8px 7px 12px;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .entry:hover {
    background: var(--hover);
  }
  .entry.on {
    background: var(--p-soft);
    box-shadow: inset 0 0 0 1px var(--p);
  }
  .entry.taken {
    color: var(--muted);
    cursor: default;
  }
  .entry:focus-visible,
  .go:focus-visible {
    outline: 2px solid var(--focus, var(--p-ink));
    outline-offset: -2px;
  }
  .nm {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .note {
    color: var(--muted);
    font-size: 11px;
    white-space: nowrap;
  }
  .empty {
    padding: 10px 12px;
  }
  .go {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-right: 4px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .go:hover {
    background: var(--hover);
    color: var(--text);
  }
  .err {
    margin: 0;
    padding: 10px 12px;
    color: var(--del);
    font-size: 12px;
  }
</style>

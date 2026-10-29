<script lang="ts">
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';
  import { t } from '../../lib/i18n';
  import { docsStore } from '../../lib/docs.svelte';
  import { dropDocuments } from '../../lib/docs-drop';
  import { fileBadge, filesUnder, fileTitle, folderDepth, folderTrail, hasChildren, readState, visibleFolders } from '../../lib/docs';
  import { fAgo, fSize } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { DocsCheck, DocsDocHit, DocsDropped, DocsFile, DocsText, Project } from '../../lib/types';
  import SpaceHead from './SpaceHead.svelte';

  // « Documents » (ESC-260, design/factory/5-documents.png): the drive of the project. Its folders on the left, the
  // files of the one open on the right, each saying whether the agents read it and how many times it is cited. A file
  // dropped in a folder goes there; dropped on « Tous les documents », Escouade reads it and files it by itself, and
  // says where. A card dragged onto a folder moves there. Over 50 MB without an LFS rule of the repository, the window
  // asks before adding the file.
  let { project }: { project: Project } = $props();

  /** The MIME type of a card being dragged: its path. */
  const DOC = 'application/x-escouade-document';

  const files = $derived(docsStore.documents[project.id] ?? []);
  const folders = $derived(docsStore.folders[project.id] ?? []);
  let folder = $state('');
  let open = $state<Record<string, boolean>>({});
  const openSet = $derived(new Set(Object.keys(open).filter((k) => open[k])));
  const shown = $derived(filesUnder(files, folder));
  const here = $derived(folders.find((f) => f.path === folder));

  // The search.
  let searching = $state(false);
  let query = $state('');
  let hits = $state<DocsDocHit[] | null>(null);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let asked = 0;

  function search(q: string) {
    query = q;
    clearTimeout(timer);
    if (!q.trim()) {
      hits = null;
      busy = false;
      return;
    }
    busy = true;
    timer = setTimeout(async () => {
      const n = ++asked;
      const found = await api.docsDocumentsSearch(project.id, q).catch(() => []);
      if (n !== asked) return;
      hits = found;
      busy = false;
    }, 250);
  }

  // « Nouveau dossier »: under the folder open.
  let creating = $state(false);
  let newName = $state('');
  let newError = $state('');

  async function createFolder(e: SubmitEvent) {
    e.preventDefault();
    newError = '';
    const path = folder ? `${folder}/${newName.trim()}` : newName.trim();
    try {
      const made = await api.docsFolderAdd(project.id, path);
      creating = false;
      newName = '';
      if (folder) open[folder] = true;
      await docsStore.load(project.id);
      folder = made;
    } catch (err) {
      newError = String(err);
    }
  }

  // Dropping.
  interface Notice {
    id: number;
    name: string;
    dropped: DocsDropped | null;
    error?: string;
  }
  let pending = $state<{ id: number; name: string }[]>([]);
  let notices = $state<Notice[]>([]);
  let seq = 0;
  let large = $state<{ files: DocsCheck[]; answer: (yes: boolean) => void } | null>(null);
  let input = $state<HTMLInputElement>();
  let dragging = $state(false);
  let dragFolder = $state<string | null>(null);

  async function drop(list: File[], into: string | null) {
    if (!list.length) return;
    await dropDocuments(project.id, list, into, {
      ask: (f) => new Promise((answer) => (large = { files: f, answer })),
      started: (name) => (pending = [...pending, { id: ++seq, name }]),
      ended: (name, dropped, error) => {
        const i = pending.findIndex((p) => p.name === name);
        if (i >= 0) pending = pending.filter((_, j) => j !== i);
        notices = [{ id: ++seq, name, dropped, error }, ...notices].slice(0, 6);
        void docsStore.load(project.id);
      },
    });
    await docsStore.load(project.id);
  }

  function answerLarge(yes: boolean) {
    large?.answer(yes);
    large = null;
  }

  function onPick(e: Event) {
    const el = e.currentTarget as HTMLInputElement;
    const list = [...(el.files ?? [])];
    el.value = '';
    void drop(list, folder || null);
  }

  function hasFiles(e: DragEvent) {
    return [...(e.dataTransfer?.types ?? [])].includes('Files');
  }
  function hasDoc(e: DragEvent) {
    return [...(e.dataTransfer?.types ?? [])].includes(DOC);
  }

  function overMain(e: DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    dragging = true;
  }
  function dropMain(e: DragEvent) {
    dragging = false;
    if (!hasFiles(e)) return;
    e.preventDefault();
    void drop([...(e.dataTransfer?.files ?? [])], folder || null);
  }

  function overFolder(e: DragEvent, path: string) {
    if (!hasFiles(e) && !hasDoc(e)) return;
    e.preventDefault();
    e.stopPropagation();
    dragFolder = path;
  }
  async function dropFolder(e: DragEvent, path: string) {
    dragFolder = null;
    dragging = false;
    if (hasDoc(e)) {
      e.preventDefault();
      e.stopPropagation();
      const doc = e.dataTransfer?.getData(DOC);
      if (doc) await move(doc, path);
    } else if (hasFiles(e)) {
      e.preventDefault();
      e.stopPropagation();
      // On « Tous les documents »: filed by Escouade.
      void drop([...(e.dataTransfer?.files ?? [])], path || null);
    }
  }

  async function move(path: string, to: string, notice?: Notice) {
    const moved = await app.run(api.docsDocumentMove(project.id, path, to));
    if (!moved) return;
    if (notice?.dropped)
      notices = notices.map((n) =>
        n.id === notice.id && n.dropped
          ? { ...n, dropped: { ...n.dropped, file: moved, placement: { folder: to, new: false, by: 'chosen' } } }
          : n,
      );
    await docsStore.load(project.id);
  }

  function placedLine(n: Notice): string {
    const p = n.dropped!.placement;
    const f = p.folder || t('docs.documents.placed.root');
    if (p.new) return t('docs.documents.placed.newFolder', { name: n.name, folder: f });
    const keys = {
      chosen: 'docs.documents.placed.chosen',
      name: 'docs.documents.placed.name',
      sort: 'docs.documents.placed.sort',
      meaning: 'docs.documents.placed.meaning',
      words: 'docs.documents.placed.words',
      new: 'docs.documents.placed.new',
    } as const;
    return t(keys[p.by] ?? keys.new, { name: n.name, folder: f });
  }

  function readLine(f: DocsFile): string {
    const st = readState(f);
    const cited = f.cited ? t('docs.documents.cited', { count: f.cited }) : '';
    if (st === 'read') return cited ? `${t('docs.documents.read')} · ${cited}` : t('docs.documents.read');
    if (st === 'cited') return cited;
    return t(st === 'unread' ? 'docs.documents.notRead' : 'docs.documents.noText');
  }

  function toggle(path: string) {
    open[path] = !open[path];
  }

  function choose(path: string) {
    folder = path;
    if (path) open[path] = true;
    searching = false;
    hits = null;
    query = '';
  }

  // The preview of a file: its text as Escouade read it, or what it knows of it.
  let preview = $state<{ file: { path: string; name: string }; text: DocsText | null; error?: string } | null>(null);
  async function show(file: { path: string; name: string }) {
    preview = { file, text: null };
    try {
      const text = await api.docsDocumentText(project.id, file.path);
      if (preview?.file.path === file.path) preview = { file, text };
    } catch (e) {
      if (preview?.file.path === file.path) preview = { file, text: null, error: String(e) };
    }
  }

  const title = $derived(here?.name ?? t('docs.spaces.documents'));
</script>

<SpaceHead {title}>
  {#snippet line()}
    {#if folder}<nav class="crumb" aria-label={t('docs.documents.crumbLabel')}>
        <button class="link" onclick={() => choose('')}>{t('docs.spaces.documents')}</button>
        {#each folderTrail(folder) as c (c.path)}
          <span aria-hidden="true">›</span>
          <button class="link" onclick={() => choose(c.path)}>{c.name}</button>
        {/each}
      </nav>{:else}<span>{t('docs.documents.count', { count: files.length })}</span>{/if}
  {/snippet}
  {#snippet actions()}
    <button class="btn" class:on={searching} aria-pressed={searching} onclick={() => ((searching = !searching), search(''))}
      ><Icon name="search" size={14} />{t('docs.documents.search')}</button
    >
    <button class="btn" onclick={() => ((creating = !creating), (newError = ''))}
      ><Icon name="folder" size={14} />{t('docs.documents.newFolder')}</button
    >
    <button class="btn primary" onclick={() => input?.click()}><Icon name="plus" size={14} />{t('docs.documents.drop')}</button>
    <input bind:this={input} type="file" multiple hidden onchange={onPick} data-documents-input />
  {/snippet}
</SpaceHead>

{#if creating}
  <form class="new" onsubmit={createFolder} aria-label={t('docs.documents.newFolder')}>
    <label
      ><span>{t('docs.documents.folderName')}</span><input
        class="field input"
        bind:value={newName}
        placeholder={t('docs.documents.folderHint')}
        required
      /></label
    >
    <button class="btn primary" type="submit">{t('docs.documents.create')}</button>
    <button class="btn ghost" type="button" onclick={() => (creating = false)}>{t('docs.page.cancel')}</button>
    {#if newError}<p class="error" role="alert">{newError}</p>{/if}
  </form>
{/if}

{#if searching}
  <div class="searchbar">
    <Icon name="search" size={14} />
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="q"
      type="search"
      autofocus
      value={query}
      placeholder={t('docs.documents.searchHint')}
      aria-label={t('docs.documents.searchLabel')}
      oninput={(e) => search(e.currentTarget.value)}
    />
    {#if busy}<span class="muted">{t('docs.documents.searching')}</span>{/if}
  </div>
{/if}

<div class="split">
  <nav class="tree" aria-label={t('docs.documents.treeLabel')}>
    <ul>
      <li>
        <button
          class="fold root"
          class:on={folder === ''}
          class:target={dragFolder === ''}
          aria-current={folder === '' ? 'page' : undefined}
          data-folder=""
          onclick={() => choose('')}
          ondragover={(e) => overFolder(e, '')}
          ondragleave={() => (dragFolder = null)}
          ondrop={(e) => dropFolder(e, '')}
        >
          <span class="label">{t('docs.documents.all')}</span><span class="n">{files.length}</span>
        </button>
      </li>
      {#each visibleFolders(folders, openSet) as f (f.path)}
        {@const kids = hasChildren(folders, f.path)}
        <li style:--depth={folderDepth(f.path)}>
          <div class="frow">
            {#if kids}
              <button
                class="caret"
                aria-label={t(open[f.path] ? 'docs.spec.fold' : 'docs.spec.unfold', { name: f.name })}
                onclick={() => toggle(f.path)}><Icon name={open[f.path] ? 'down' : 'right'} size={11} /></button
              >
            {:else}
              <span class="caret" aria-hidden="true"></span>
            {/if}
            <button
              class="fold"
              class:on={folder === f.path}
              class:target={dragFolder === f.path}
              aria-current={folder === f.path ? 'page' : undefined}
              data-folder={f.path}
              onclick={() => choose(f.path)}
              ondragover={(e) => overFolder(e, f.path)}
              ondragleave={() => (dragFolder = null)}
              ondrop={(e) => dropFolder(e, f.path)}
            >
              <span class="label">{f.name}</span><span class="n">{f.files}</span>
            </button>
          </div>
        </li>
      {/each}
    </ul>
  </nav>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="files"
    class:dragging
    ondragover={overMain}
    ondragleave={(e) => {
      if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dragging = false;
    }}
    ondrop={dropMain}
  >
    {#if notices.length}
      <ul class="notices" aria-live="polite">
        {#each notices as n (n.id)}
          <li class="notice" class:bad={!n.dropped} data-notice={n.name}>
            {#if n.dropped}
              <Icon name="check" size={13} />
              <span class="what">{placedLine(n)}</span>
              {#if n.dropped.lfs}<span class="tag">{t('docs.documents.lfs')}</span>{/if}
              <label class="move">
                <span>{t('docs.documents.moveTo')}</span>
                <select
                  class="field"
                  aria-label={t('docs.documents.moveLabel', { name: n.name })}
                  value={n.dropped.file.folder}
                  onchange={(e) => move(n.dropped!.file.path, e.currentTarget.value, n)}
                >
                  <option value="">{t('docs.documents.all')}</option>
                  {#each folders as f (f.path)}<option value={f.path}>{f.path.replaceAll('/', ' › ')}</option>{/each}
                </select>
              </label>
            {:else}
              <span class="what">{t('docs.documents.failed', { name: n.name, error: n.error ?? '' })}</span>
            {/if}
            <button class="x" aria-label={t('docs.documents.dismiss')} onclick={() => (notices = notices.filter((x) => x.id !== n.id))}
              ><Icon name="close" size={12} /></button
            >
          </li>
        {/each}
      </ul>
    {/if}

    {#if hits}
      {#if !hits.length}
        <p class="muted">{t('docs.documents.noHit', { query })}</p>
      {:else}
        <ul class="hits" aria-label={t('docs.documents.searchLabel')}>
          {#each hits as h (h.path)}
            <li>
              <button class="hit" data-hit={h.path} onclick={() => show(h)}>
                <span class="h-head"
                  ><span class="badge">{fileBadge(h)}</span><b>{fileTitle(h.name)}</b><span class="muted"
                    >{h.folder.replaceAll('/', ' › ')}</span
                  ></span
                >
                <span class="excerpt">{h.excerpt}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {:else}
      {#if shown.length || pending.length}
        <ul class="grid">
          {#each shown as f (f.path)}
            {@const st = readState(f)}
            <li>
              <button
                class="fcard"
                draggable="true"
                data-document={f.path}
                onclick={() => show(f)}
                ondragstart={(e) => {
                  e.dataTransfer?.setData(DOC, f.path);
                  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
                }}
              >
                <span class="ic" class:img={f.kind === 'image'}>{fileBadge(f)}</span>
                <span class="nm" use:tip={f.name}>{fileTitle(f.name)}</span>
                <span class="mt"
                  >{t('docs.documents.meta', {
                    who: f.author,
                    size: fSize(f.size),
                    when: fAgo(f.modified / 1000, app.now),
                  })}{#if folder !== f.folder && f.folder}<span class="in">{` · ${f.folder.replaceAll('/', ' › ')}`}</span>{/if}</span
                >
                <span class="read s-{st}" data-read={st}>{readLine(f)}</span>
              </button>
            </li>
          {/each}
          {#each pending as p (p.id)}
            <li>
              <div class="fcard pending" data-pending={p.name}>
                <span class="ic">{fileBadge({ name: p.name, kind: '' })}</span>
                <span class="nm">{fileTitle(p.name)}</span>
                <span class="read">{t('docs.documents.reading')}</span>
              </div>
            </li>
          {/each}
        </ul>
      {:else if !files.length}
        <p class="muted">{t('docs.documents.empty')}</p>
      {/if}
      <button class="drop" class:dragging onclick={() => input?.click()} data-dropzone
        >{folder ? t('docs.documents.dropHere', { folder: here?.name ?? folder }) : t('docs.documents.dropAuto')}</button
      >
    {/if}
  </div>
</div>

{#if large}
  <Modal title={t('docs.documents.large.title')} width={520} onclose={() => answerLarge(false)}>
    <p class="intro">{t('docs.documents.large.intro')}</p>
    <ul class="large">
      {#each large.files as f (f.name)}<li><b>{f.name}</b> · {fSize(f.size)}</li>{/each}
    </ul>
    {#snippet footer()}
      <button class="btn ghost" onclick={() => answerLarge(false)}>{t('docs.documents.large.skip')}</button>
      <button class="btn primary" onclick={() => answerLarge(true)}>{t('docs.documents.large.add')}</button>
    {/snippet}
  </Modal>
{/if}

{#if preview}
  <Modal title={preview.file.name} width={720} tall onclose={() => (preview = null)}>
    {#if preview.error}
      <p class="error" role="alert">{preview.error}</p>
    {:else if !preview.text}
      <p class="muted">{t('docs.documents.reading')}</p>
    {:else if preview.text.text === null}
      {#if preview.text.meta}<p class="meta">{preview.text.meta}</p>{/if}
      <p class="muted">{t('docs.documents.preview.none')}</p>
    {:else}
      <pre class="text" data-preview>{preview.text.text}</pre>
      {#if preview.text.truncated}<p class="muted">{t('docs.documents.preview.more', { count: preview.text.total })}</p>{/if}
    {/if}
    {#snippet footer()}
      <button class="btn" onclick={() => (preview = null)}>{t('docs.documents.preview.close')}</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .crumb {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
  }
  .link:hover {
    color: var(--text);
  }
  .btn.on {
    background: var(--raise-2);
    color: var(--text);
  }
  .new {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 10px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--line);
  }
  .new label {
    display: grid;
    gap: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .error {
    width: 100%;
    margin: 0;
    color: var(--del);
  }
  .searchbar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
  }
  .q {
    flex: 1;
    min-width: 0;
    height: 32px;
    border: none;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(180px, 260px) minmax(0, 1fr);
  }
  .tree {
    min-height: 0;
    overflow: auto;
    padding: 14px 10px;
    border-right: 1px solid var(--line);
  }
  .tree ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
  }
  .tree li {
    padding-left: calc(var(--depth, 0) * 14px);
  }
  .frow {
    display: flex;
    align-items: center;
  }
  .caret {
    flex: none;
    width: 18px;
    height: 32px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    background: none;
    color: var(--dim);
  }
  .caret:hover {
    color: var(--text);
  }
  .fold {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--soft);
    font-size: 13.5px;
    text-align: left;
  }
  .fold.root {
    width: 100%;
    padding-left: 26px;
    color: var(--text);
  }
  .fold:hover {
    background: var(--hover);
  }
  .fold.on {
    background: var(--raise-2);
    color: var(--text);
  }
  .fold.target {
    box-shadow: inset 0 0 0 1px var(--p-ink);
    background: var(--hover);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    flex: none;
    color: var(--dim);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .files {
    min-width: 0;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 22px;
  }
  .grid {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 14px;
  }
  .fcard {
    width: 100%;
    height: 100%;
    display: grid;
    align-content: start;
    gap: 7px;
    padding: 12px;
    border: 1px solid var(--line-2);
    border-radius: 11px;
    background: var(--raise);
    color: var(--soft);
    text-align: left;
    font: inherit;
  }
  button.fcard:hover {
    border-color: var(--line-3, var(--line-2));
    background: var(--hover);
  }
  .fcard.pending {
    border-style: dashed;
    background: transparent;
  }
  .ic {
    height: 64px;
    display: grid;
    place-items: center;
    margin-bottom: 4px;
    border-radius: 7px;
    background: var(--raise-2);
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
  }
  .nm {
    overflow: hidden;
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mt {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .read {
    color: var(--muted);
    font-size: 12px;
  }
  .read.s-read,
  .read.s-cited {
    color: var(--run);
  }
  .drop {
    flex: none;
    padding: 18px;
    border: 1px dashed var(--line-2);
    border-radius: 10px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 13px;
    text-align: center;
  }
  .drop:hover,
  .drop.dragging,
  .files.dragging .drop {
    border-color: var(--p-ink);
    color: var(--text);
  }
  .notices {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .notice {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    padding: 8px 10px 8px 12px;
    border: 1px solid var(--line-2);
    border-radius: 9px;
    background: var(--raise);
    color: var(--soft);
    font-size: 13px;
  }
  .notice :global(svg) {
    color: var(--run);
  }
  .notice.bad {
    border-color: var(--del);
  }
  .what {
    flex: 1;
    min-width: 200px;
  }
  .tag {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--raise-2);
    color: var(--muted);
    font-size: 11.5px;
  }
  .move {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  .move select {
    height: 28px;
    max-width: 220px;
  }
  .x {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--dim);
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }
  .hits {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
  }
  .hit {
    width: 100%;
    display: grid;
    gap: 4px;
    padding: 10px 12px;
    border: none;
    border-radius: 9px;
    background: transparent;
    color: var(--soft);
    font: inherit;
    text-align: left;
  }
  .hit:hover {
    background: var(--hover);
  }
  .h-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    color: var(--text);
  }
  .badge {
    color: var(--dim);
    font-size: 11px;
    font-weight: 700;
  }
  .excerpt {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .muted {
    margin: 0;
    color: var(--muted);
  }
  .intro {
    margin: 0 0 12px;
    color: var(--muted);
    line-height: 1.5;
  }
  .large {
    margin: 0;
    padding-left: 18px;
    color: var(--soft);
  }
  .meta {
    margin: 0 0 8px;
    color: var(--soft);
  }
  .text {
    margin: 0;
    white-space: pre-wrap;
    font-family: inherit;
    color: var(--soft);
    font-size: 13px;
    line-height: 1.55;
  }
</style>

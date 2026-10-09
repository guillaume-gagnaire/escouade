<script lang="ts">
  import type { Extension } from '@codemirror/state';
  import { onDestroy, untrack } from 'svelte';
  import { saveActive, saveKey } from '../../lib/editor/actions';
  import { buffers, sourceAgent } from '../../lib/editor/buffers.svelte';
  import { lineChanges, type LineChanges } from '../../lib/editor/changes';
  import { newFileError, newFilePath } from '../../lib/editor/create';
  import { definitionResolver } from '../../lib/editor/definitions';
  import type { NavFollowed, NavFrom, NavTarget } from '../../lib/editor/goto';
  import { navHistory, type NavEntry } from '../../lib/editor/history';
  import { detectIndent } from '../../lib/editor/indent';
  import { languageLabel, loadLanguage } from '../../lib/editor/languages';
  import { DEFAULT_ALIASES, fileSet, linkResolvers, parseAliases, type Aliases } from '../../lib/editor/links';
  import { ancestors, treeRows, type FileStatus, type TreeRow } from '../../lib/editor/tree';
  import { trees } from '../../lib/editor/trees.svelte';
  import { basename, joinPath, plural, tildify } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { keyLabel } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import CodeEditor from './CodeEditor.svelte';
  import EditorTabs from './EditorTabs.svelte';
  import FileTree from './FileTree.svelte';
  import SourcePicker from './SourcePicker.svelte';
  import TargetPicker from './TargetPicker.svelte';

  let { project }: { project: Project } = $props();

  const NONE: LineChanges = { changed: [], deleted: [], count: 0 };

  const st = $derived(app.editor[project.id]);
  const source = $derived(st?.source ?? 'project');
  const place = $derived(st?.places[source]);
  const tree = $derived(trees.get(project.id, source));
  const srcAgent = $derived(source === 'project' ? null : (app.agents[source] ?? null));
  const activePath = $derived(place?.active ?? null);
  const activeKey = $derived(activePath ? buffers.key(project.id, source, activePath) : null);
  const buf = $derived(activeKey ? buffers.all[activeKey] : undefined);
  const dirty = $derived(buffers.isDirty(buf));

  let status = $state<Record<string, FileStatus>>({});
  let cursor = $state({ line: 1, col: 1 });
  let language = $state<Extension | null>(null);
  let changes = $state<LineChanges>(NONE);
  /** The folder getting a new file ('' for the root), with the source it was asked on. */
  let adding = $state<{ source: string; dir: string } | null>(null);
  /** The folder of the row last clicked: where "Nouveau fichier" creates, as VS Code does with its selection. */
  let lastDir = $state<{ source: string; dir: string } | null>(null);
  /** The import aliases of the source shown, from its tsconfig.json or jsconfig.json: read when a link is looked for. */
  let aliases: Aliases = DEFAULT_ALIASES;
  // The links first; else the definition of the identifier.
  const resolvers = [
    ...linkResolvers(() => aliases),
    definitionResolver({
      read: readSource,
      search: (q) => api.codeSearch(project.id, sourceAgent(source), q),
      aliases: () => aliases,
    }),
  ];

  /** A file of the source shown, as the editor has it when it is open (with what is not saved yet); null when unreadable. */
  async function readSource(path: string): Promise<string | null> {
    const b = buffers.all[buffers.key(project.id, source, path)];
    if (b?.kind === 'text') return b.text;
    const f = await api.fsRead(project.id, sourceAgent(source), path).catch(() => null);
    return f?.text ?? null;
  }

  /**
   * Tree, git status and open files of the source. Only the refresh `pick`ing, the one a source is shown with, opens
   * a first file when none is open and tells that the tree could not be read: a tab closed on purpose stays closed
   * while the agent works.
   */
  async function refresh(pid: string, src: string, pick: boolean) {
    const [t, files] = await Promise.all([
      trees.load(pid, src).catch((e) => {
        if (pick) app.toast(String(e), 'error');
        return undefined;
      }),
      api.gitFiles(pid, sourceAgent(src)).catch(() => []),
    ]);
    const current = () => alive && pid === project.id && src === source;
    if (!current()) return;
    if (pick && t) readAliases(pid, src, t.files);
    status = Object.fromEntries(
      files.filter((f) => (src === 'project' ? !f.inWorktree : f.inWorktree && f.agentId === src)).map((f) => [f.path, f.status]),
    );
    await buffers.refreshAll(pid, src);
    if (!pick || !current()) return;
    const p = app.editor[pid]?.places[src];
    if (t && p && !p.active && !p.open.length) {
      const first = t.files.find((f) => status[f]) ?? (t.files.includes('README.md') ? 'README.md' : null);
      if (first) await app.openEditor({ projectId: pid, source: src, path: first });
    }
  }

  /** The aliases of the tsconfig.json (else jsconfig.json) at the source's root, read each time the source is shown. */
  async function readAliases(pid: string, src: string, files: readonly string[]) {
    const name = ['tsconfig.json', 'jsconfig.json'].find((f) => files.includes(f));
    const f = name ? await api.fsRead(pid, sourceAgent(src), name).catch(() => null) : null;
    if (alive && pid === project.id && src === source) aliases = f?.text ? parseAliases(f.text) : DEFAULT_ALIASES;
  }

  // Right away for each source, then 300 ms after each git event of the project. A file being named in the tree of
  // the previous source is given up.
  $effect(() => {
    const pid = project.id;
    const src = source;
    untrack(() => {
      adding = null;
      aliases = DEFAULT_ALIASES;
      refresh(pid, src, true);
    });
  });
  let tick = untrack(() => app.gitTick);
  let gitTimer: ReturnType<typeof setTimeout> | undefined;
  /** False once the view is gone: a reply arriving late must not open a file or bring the project back. */
  let alive = true;
  onDestroy(() => {
    alive = false;
    clearTimeout(gitTimer);
  });
  $effect(() => {
    const t = app.gitTick;
    if (t === tick) return;
    tick = t;
    clearTimeout(gitTimer);
    gitTimer = setTimeout(() => refresh(project.id, source, false), 300);
  });

  // A tab shown again is read again: the agent may have changed its file meanwhile.
  $effect(() => {
    const p = activePath;
    const pid = project.id;
    const src = source;
    if (!p) return;
    untrack(() => {
      const k = buffers.key(pid, src, p);
      // A file that was missing or unreadable is read again by `open`: the agent may have created it since.
      if (buffers.all[k]?.kind === 'text') buffers.refresh(k);
      else buffers.open(pid, src, p);
    });
  });

  $effect(() => {
    const p = activePath;
    language = null;
    if (!p) return;
    let live = true;
    loadLanguage(p).then((l) => live && (language = l));
    return () => {
      live = false;
    };
  });

  const indent = $derived.by(() => {
    void buf?.key;
    void buf?.version;
    return untrack(() => detectIndent(buf?.saved ?? ''));
  });

  let changeTimer: ReturnType<typeof setTimeout> | undefined;
  /** The file `changes` were computed for: another one's marks and counts are not shown on the file now open. */
  let changesKey = $state<string | null>(null);
  $effect(() => {
    const b = buf;
    const text = b?.text ?? '';
    const base = b?.base;
    clearTimeout(changeTimer);
    if (!b || b.kind !== 'text' || !base) {
      changes = NONE;
      changesKey = null;
      return;
    }
    const key = b.key;
    untrack(() => {
      if (changesKey !== key) {
        changes = NONE;
        changesKey = null;
      }
    });
    changeTimer = setTimeout(() => {
      changes = lineChanges(base.text, text);
      changesKey = key;
    }, 300);
    return () => clearTimeout(changeTimer);
  });

  const addingDir = $derived(adding?.source === source ? adding.dir : null);
  const parentOf = (path: string) => ancestors(path).at(-1) ?? '';

  /** A file being created: no other field opens until it is, so the field `adding` is still its own, or none. */
  let creating = false;

  /** Opens the field naming a new file in `dir`, or in the nearest folder above it the tree shows. */
  function startNew(dir: string) {
    if (creating) return;
    const files = tree?.files ?? [];
    while (dir && !files.some((f) => f.startsWith(dir + '/'))) dir = parentOf(dir);
    app.expandEditorDir(project.id, source, dir);
    adding = { source, dir };
  }
  const newHere = () => startNew(lastDir?.source === source ? lastDir.dir : activePath ? parentOf(activePath) : '');

  /** Creates the file `name` the field `mine` names and opens it; what refused it otherwise, for the field to show. */
  async function create(mine: { source: string; dir: string } | null, name: string): Promise<string | null> {
    if (!mine) return null;
    const pid = project.id;
    const src = mine.source;
    const path = newFilePath(mine.dir, name);
    creating = true;
    try {
      await api.fsCreate(pid, sourceAgent(src), path);
    } catch (e) {
      // Its field gone meanwhile (the source changed), the refusal is told otherwise.
      if (adding !== mine) app.toast(`Création impossible : ${e}`, 'error');
      return String(e);
    } finally {
      creating = false;
    }
    adding = null;
    const before = trees.get(pid, src);
    await refresh(pid, src, false);
    if (!alive || pid !== project.id || src !== source) return null;
    // As the disk names it: `SRC/x.ts` typed is the `src/x.ts` the tree shows on Windows and macOS, not a second tab.
    const t = trees.get(pid, src);
    const fresh = t && t !== before && !t.truncated ? t : null;
    const lower = path.toLowerCase();
    const real = t?.files.includes(path) ? path : (t?.files.find((f) => f.toLowerCase() === lower) ?? path);
    lastDir = { source: src, dir: parentOf(real) };
    // Line 1 asked for: the cursor goes in the file, to type in it right away.
    await app.openEditor({ projectId: pid, source: src, path: real, line: 1 });
    if (fresh && !fresh.files.includes(real)) app.toast(`${basename(real)} est ignoré par git : l’arborescence ne le montre pas.`);
    return null;
  }

  const copy = (text: string) => navigator.clipboard.writeText(text).catch((e) => app.toast(`Copie impossible : ${e}`, 'error'));

  function treeMenu(e: MouseEvent, r: TreeRow | null) {
    const dir = !r ? '' : r.kind === 'dir' ? r.path : parentOf(r.path);
    const items: MenuItem[] = [{ label: 'Nouveau fichier…', onClick: () => startNew(dir) }];
    const root = tree?.root;
    if (r && root) {
      items.push(
        { label: '', separator: true },
        { label: 'Copier le chemin', onClick: () => copy(joinPath(root, r.path)) },
        { label: 'Copier le chemin relatif', onClick: () => copy(r.path) },
      );
    }
    menu.show(e, items);
  }

  const rows = $derived(tree ? treeRows(tree.files, place?.expanded ?? {}, status, addingDir) : []);
  const changedCount = $derived(Object.keys(status).length);
  const tabs = $derived(
    (place?.open ?? []).map((p) => ({
      path: p,
      name: basename(p),
      dirty: buffers.isDirty(buffers.all[buffers.key(project.id, source, p)]),
      status: status[p] ?? null,
      active: p === activePath,
    })),
  );

  const diffLabel = $derived.by(() => {
    const base = buf?.base;
    if (!buf || buf.kind !== 'text' || !base) return '';
    if (base.text === null) return `Nouveau fichier · absent de ${base.reference}`;
    if (changesKey !== buf.key) return '';
    return changes.count
      ? `${plural(changes.count, 'ligne modifiée', 'lignes modifiées')} vs ${base.reference}`
      : `Identique à ${base.reference}`;
  });

  const reveal = $derived(st?.reveal && st.reveal.path === activePath ? st.reveal : null);
  /** Done once: the tab shown again, or the view opened again, keeps the cursor where the user left it. */
  function revealed(seq: number) {
    const s = app.editor[project.id];
    if (s?.reveal?.seq === seq) s.reveal = null;
  }

  /** Where the cursor is (its column in characters): what a jump keeps in the history, to come back to. */
  const here = (): NavFrom | null => (activePath ? { path: activePath, line: cursor.line, col: cursor.col } : null);

  /** Opens `t` in the source shown (at its top without a line), the place left kept in the history: `from`, else the cursor. */
  function jump(t: NavTarget, from: NavFrom | null = here()) {
    if (from) navHistory.push({ projectId: project.id, source, ...from });
    app.openEditor({ projectId: project.id, source, path: t.path, line: t.line ?? 1, col: t.col });
  }

  /** The places a followed identifier may lead to, listed under it, and where it was followed from. */
  let picking = $state<{ targets: NavTarget[]; from: NavFrom; label: string; at: NavFollowed['rect'] } | null>(null);

  /**
   * A link or an identifier followed, to those of its targets that are files of the source (the file shown always is,
   * even one the tree leaves out): straight to a single one, to the one picked from a list of several.
   */
  function follow(targets: NavTarget[], from: NavFrom, spot: NavFollowed) {
    // The editor closed meanwhile is not opened again.
    if (!alive) return;
    const files = fileSet(tree?.files ?? []);
    const found = targets.filter((x) => x.path === from.path || files.has(x.path));
    picking = null;
    if (found.length === 1) jump(found[0], from);
    else if (found.length) picking = { targets: found, from, label: spot.label, at: spot.rect };
    else if (targets.length) app.toast(`Fichier introuvable : ${targets[0].path}`);
    else app.toast(`Aucune définition trouvée pour « ${spot.label} ».`);
  }

  /** The place picked from the list: a jump from where the identifier was followed. */
  function pick(t: NavTarget) {
    const p = picking;
    picking = null;
    if (p) jump(t, p.from);
  }

  // The list goes with the file or the source it was made in.
  $effect(() => {
    void activePath;
    void source;
    picking = null;
  });

  /** Back (or forward) in the history of the source shown, past the places whose file is gone from its tree. */
  function travel(back: boolean) {
    const from = here();
    if (!from) return;
    const at: NavEntry = { projectId: project.id, source, ...from };
    const exists = (e: NavEntry) => !!trees.get(e.projectId, e.source)?.files.includes(e.path);
    const e = back ? navHistory.back(at, exists) : navHistory.forward(at, exists);
    if (e) app.openEditor({ projectId: e.projectId, source: e.source, path: e.path, line: e.line, col: e.col });
  }

  const sizeMb = (n: number) => (n / (1024 * 1024)).toLocaleString('fr-FR', { maximumFractionDigits: 1 });

  function closeTab(path: string) {
    // The source now: the prompt may stay up while the view moves to another one.
    const src = source;
    const key = buffers.key(project.id, src, path);
    const drop = () => {
      buffers.close(key);
      app.closeEditorTab(project.id, src, path);
    };
    if (!buffers.isDirty(buffers.all[key])) return drop();
    app.modal = {
      kind: 'confirm',
      title: `Enregistrer « ${basename(path)} » ?`,
      body: 'Ses modifications seront perdues si tu ne les enregistres pas.',
      confirm: 'Enregistrer',
      alt: { label: 'Ne pas enregistrer', onClick: drop },
      onConfirm: async () => {
        if (await saveKey(key)) drop();
        // Refused: show the tab, the banner telling why is only drawn for the file on screen.
        else app.openEditor({ projectId: project.id, source: src, path });
      },
    };
  }

  const keep = (key: string) => buffers.keepMine(key).catch((e) => app.toast(`Enregistrement impossible : ${e}`, 'error'));
  // A deleted file without changes can only be written by creating it again, as its banner offers.
  const save = () => (buf && !dirty && buf.disk === 'deleted' ? keep(buf.key) : saveActive());
  const reload = (key: string) => buffers.reload(key).catch((e) => app.toast(String(e), 'error'));
</script>

<main class="editor">
  <header class="head">
    <button class="back" onclick={() => app.closeEditor(project.id)}>← Conversation</button>
    <span class="sep"></span>
    <SourcePicker
      {project}
      {source}
      agents={app.projectAgents.filter((a) => a.worktree)}
      git={app.git[project.id]}
      onpick={(s) => app.openEditor({ projectId: project.id, source: s })}
    />
    <div style="flex:1"></div>
    {#if buf?.kind === 'text'}
      <span class="hint mono" class:dirty>{dirty ? `● Non enregistré · ${keyLabel('Ctrl+S')}` : 'Enregistré'}</span>
    {/if}
    <button class="btn" class:primary={dirty} disabled={buf?.kind !== 'text' || (!dirty && buf.disk === 'ok')} onclick={save}
      >Enregistrer</button
    >
  </header>
  <div class="body">
    <aside class="files">
      <div class="ftitle">
        <div class="row">
          <span class="label">Fichiers</span>
          <div class="actions">
            <button class="act" aria-label="Nouveau fichier" title="Nouveau fichier" disabled={!tree} onclick={newHere}>
              <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
                ><path
                  d="M8.5 2H4.5A1.5 1.5 0 0 0 3 3.5v9A1.5 1.5 0 0 0 4.5 14H8M8.5 2 13 6.5M8.5 2v4.5H13M13 6.5V9M12 10.5v4M10 12.5h4"
                /></svg
              >
            </button>
            <button class="act" aria-label="Actualiser" title="Actualiser" onclick={() => refresh(project.id, source, false)}>
              <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"><path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5V5h-2.5" /></svg>
            </button>
            <button class="act" aria-label="Tout réduire" title="Tout réduire" onclick={() => app.collapseEditorDirs(project.id, source)}>
              <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
                ><rect x="5.5" y="5.5" width="8" height="8" rx="1" /><path d="M3 10.5V3.8a.8.8 0 0 1 .8-.8h6.7M7.5 9.5h4" /></svg
              >
            </button>
          </div>
        </div>
        <span class="mono dim root" title={tree?.root}
          >{srcAgent
            ? `.claude/worktrees/${srcAgent.worktree ? basename(srcAgent.worktree.path) : srcAgent.name}`
            : tildify(project.path)}</span
        >
        {#if tree}
          <span class="mono dim count"
            >{plural(tree.files.length, 'fichier', 'fichiers')} · {changedCount} modif.{tree.truncated ? ' · liste tronquée' : ''}</span
          >
        {/if}
      </div>
      <div class="scroll">
        <FileTree
          {rows}
          active={activePath}
          ontoggle={(d) => {
            lastDir = { source, dir: d };
            app.toggleEditorDir(project.id, source, d);
          }}
          onopen={(p) => {
            lastDir = { source, dir: parentOf(p) };
            app.openEditor({ projectId: project.id, source, path: p });
          }}
          onmenu={treeMenu}
          check={(name) => newFileError(name, addingDir ?? '', tree?.files ?? [])}
          oncreate={(name) => create(adding, name)}
          oncancel={() => (adding = null)}
        />
      </div>
    </aside>
    <section class="pane">
      <EditorTabs {tabs} onselect={(p) => app.openEditor({ projectId: project.id, source, path: p })} onclose={closeTab} />
      {#if !activePath}
        <div class="empty">Sélectionne un fichier dans l’arborescence.</div>
      {:else}
        <div class="crumbs mono">
          <span class="path">{activePath.split('/').join('  /  ')}</span>
          <div style="flex:1"></div>
          <span class:add={!!diffLabel && !diffLabel.startsWith('Identique')}>{diffLabel}</span>
        </div>
        {#if buf?.disk === 'changed'}
          <div class="banner" role="alert">
            Ce fichier a changé sur le disque.
            <button class="btn small" onclick={() => reload(buf.key)}>Recharger</button>
            <button class="btn small" onclick={() => keep(buf.key)}>Garder ma version</button>
          </div>
        {:else if buf?.disk === 'deleted'}
          <div class="banner" role="alert">
            Ce fichier a été supprimé.
            <button class="btn small" onclick={() => closeTab(buf.path)}>Fermer</button>
            <button class="btn small" onclick={() => keep(buf.key)}>Le recréer en enregistrant</button>
          </div>
        {/if}
        {#if !buf}
          <div class="empty"></div>
        {:else if buf.kind === 'binary'}
          <div class="empty">Fichier binaire : pas d’aperçu.</div>
        {:else if buf.kind === 'tooLarge'}
          <div class="empty">Fichier trop volumineux pour l’éditeur ({sizeMb(buf.size)} Mo).</div>
        {:else if buf.kind === 'missing'}
          <div class="empty">Ce fichier n’existe pas (ou plus).</div>
        {:else if buf.kind === 'error'}
          <div class="empty">{buf.error}</div>
        {:else}
          <CodeEditor
            docKey={buf.key}
            text={buf.text}
            version={buf.version}
            {language}
            {indent}
            {changes}
            {reveal}
            nav={{ path: activePath, files: tree?.files ?? [], resolvers }}
            onrevealed={revealed}
            onchange={(t) => buffers.edit(buf.key, t)}
            oncursor={(c) => (cursor = c)}
            ontargets={follow}
            onnaverror={(e) => alive && app.toast(`Navigation impossible : ${e}`, 'error')}
            onback={() => travel(true)}
            onforward={() => travel(false)}
          />
          {#if picking}
            <TargetPicker targets={picking.targets} label={picking.label} at={picking.at} onpick={pick} onclose={() => (picking = null)} />
          {/if}
          <div class="status mono">
            <span>Ln {cursor.line}, Col {cursor.col}</span>
            <span>{languageLabel(activePath)}</span>
            <span>UTF-8</span>
            <span>{buf.eol === 'crlf' ? 'CRLF' : 'LF'}</span>
            <span>{indent.tabs ? 'Tabulations' : `Espaces : ${indent.size}`}</span>
            <div style="flex:1"></div>
            <span>{srcAgent ? `worktree · ${srcAgent.name}` : `branche · ${app.git[project.id]?.branch || 'projet'}`}</span>
          </div>
        {/if}
      {/if}
    </section>
  </div>
</main>

<style>
  .editor {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 18px 0 14px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .back {
    height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .back:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .sep {
    width: 1px;
    height: 22px;
    background: var(--line2);
  }
  .hint {
    font-size: 11px;
    color: var(--dim);
  }
  .hint.dirty {
    color: var(--wait);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .files {
    width: 240px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--line);
    background: var(--bg);
  }
  .ftitle {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 14px 14px 10px 16px;
  }
  .ftitle .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .label {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
  }
  .act {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    padding: 0;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .act:hover:not(:disabled) {
    background: var(--elev2);
    color: var(--text);
  }
  .act:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .act svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dim {
    font-size: 11px;
    color: var(--dim);
  }
  .root,
  .count {
    font-size: 10.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
  .pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--term);
  }
  .crumbs {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
    font-size: 11px;
    color: var(--muted);
    white-space: nowrap;
  }
  .crumbs .path {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .crumbs .add {
    color: var(--add);
  }
  .banner {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    background: var(--wait-soft);
    border-bottom: 1px solid var(--line);
    color: var(--wait);
    font-size: 12.5px;
  }
  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted);
    font-size: 13px;
  }
  .status {
    height: 26px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 14px;
    border-top: 1px solid var(--line);
    background: var(--bg);
    font-size: 10.5px;
    color: var(--muted);
    white-space: nowrap;
  }
</style>

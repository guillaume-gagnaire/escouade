<script lang="ts">
  import type { Extension } from '@codemirror/state';
  import { onDestroy, onMount, tick as drawn, untrack } from 'svelte';
  import { saveActive, saveKey } from '../../lib/editor/actions';
  import { buffers, sourceAgent } from '../../lib/editor/buffers.svelte';
  import { lineChanges, type LineChanges } from '../../lib/editor/changes';
  import type { Comparison } from '../../lib/editor/compare';
  import { newFileError, newFilePath, renameError, type EntryKind, type Names } from '../../lib/editor/create';
  import { definitionResolver } from '../../lib/editor/definitions';
  import type { NavFollowed, NavFrom, NavTarget } from '../../lib/editor/goto';
  import { navHistory, type NavEntry } from '../../lib/editor/history';
  import { detectIndent } from '../../lib/editor/indent';
  import { languageLabel, loadLanguage } from '../../lib/editor/languages';
  import { readTreeWidth, TREE_DEFAULT, TREE_MIN, treeMax, writeTreeWidth } from '../../lib/editor/layout';
  import { DEFAULT_ALIASES, fileSet, linkResolvers, parseAliases, type Aliases } from '../../lib/editor/links';
  import { setEditorJump } from '../../lib/editor/quick-open';
  import { fileSearches, setFindInFiles } from '../../lib/editor/search.svelte';
  import { ancestors, movedPath, treeRows, type FileStatus, type TreeRow } from '../../lib/editor/tree';
  import { trees } from '../../lib/editor/trees.svelte';
  import { basename, joinPath, plural, tildify } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { IS_MAC, keyLabel } from '../../lib/platform';
  import { clamp, observeWidth } from '../../lib/resize';
  import { app } from '../../lib/state.svelte';
  import { terminalIn } from '../../lib/term-actions';
  import type { Project } from '../../lib/types';
  import Splitter from '../Splitter.svelte';
  import CodeEditor from './CodeEditor.svelte';
  import EditorTabs from './EditorTabs.svelte';
  import FileTree from './FileTree.svelte';
  import SearchPanel from './SearchPanel.svelte';
  import SourcePicker from './SourcePicker.svelte';
  import TargetPicker from './TargetPicker.svelte';

  let { project }: { project: Project } = $props();

  const NONE: LineChanges = { changed: [], deleted: [], count: 0 };

  /** The width of the editor's area, which the left column takes at most half of: the window's until it is measured. */
  let area = $state(window.innerWidth);
  /** The width of the left column chosen with its handle, the same for all projects. */
  let treeWidth = $state(readTreeWidth());
  const treeLimit = $derived(treeMax(area));
  /** What it is drawn at: too narrow an area for the width chosen draws it narrower, and the width chosen comes back with the room. */
  const treeShown = $derived(clamp(treeWidth, TREE_MIN, treeLimit));

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
  /** The folder getting a new file or folder ('' for the root), with the source it was asked on. */
  let adding = $state<{ source: string; dir: string; kind: EntryKind } | null>(null);
  /** The file or folder being renamed in the tree, with the source it was asked on. */
  let renaming = $state<{ source: string; kind: EntryKind; path: string } | null>(null);
  /**
   * The folders made in the source shown that hold no file yet: git lists none such, the tree shows them all the same,
   * until a file is in them or the source changes.
   */
  let madeDirs = $state<string[]>([]);
  /** The folder of the row last clicked: where "Nouveau fichier" creates, as VS Code does with its selection. */
  let lastDir = $state<{ source: string; dir: string } | null>(null);
  let fileTree = $state<ReturnType<typeof FileTree>>();
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
    if (!current(pid, src)) return;
    if (pick && t) readAliases(pid, src, t.files);
    if (t && madeDirs.length) {
      // A folder made empty that a file is in now is in the tree with it.
      const held = t.files.map((f) => f.toLowerCase());
      madeDirs = madeDirs.filter((d) => !held.some((f) => f.startsWith(d.toLowerCase() + '/')));
    }
    status = Object.fromEntries(
      files.filter((f) => (src === 'project' ? !f.inWorktree : f.inWorktree && f.agentId === src)).map((f) => [f.path, f.status]),
    );
    await buffers.refreshAll(pid, src);
    if (!pick || !current(pid, src)) return;
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

  // Right away for each source, then 300 ms after each git event of the project. A file being named or renamed in the
  // tree of the previous source is given up, and so are the empty folders made in it.
  $effect(() => {
    const pid = project.id;
    const src = source;
    untrack(() => {
      adding = null;
      renaming = null;
      madeDirs = [];
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
  /** What the source shown holds, for a name typed in the tree to be checked against. */
  const names = (): Names => ({ files: tree?.files ?? [], dirs: madeDirs, ignored: tree?.ignored });
  /** `dir` is a folder of the tree: one holding a file, or made empty. */
  const isDir = (dir: string) =>
    (tree?.files ?? []).some((f) => f.startsWith(dir + '/')) || madeDirs.some((d) => d === dir || d.startsWith(dir + '/'));
  const current = (pid: string, src: string) => alive && pid === project.id && src === source;

  /** A file or folder being created or renamed: no other field opens until it is, so the field open is still its own, or none. */
  let busy = false;

  /** Opens the field naming a new file (or folder) in `dir`, or in the nearest folder above it the tree shows. */
  function startNew(dir: string, kind: EntryKind = 'file') {
    if (busy) return;
    renaming = null;
    while (dir && !isDir(dir)) dir = parentOf(dir);
    app.expandEditorDir(project.id, source, dir);
    adding = { source, dir, kind };
  }
  const newHere = (kind: EntryKind) => startNew(lastDir?.source === source ? lastDir.dir : activePath ? parentOf(activePath) : '', kind);

  /** `path` as the disk spells its folders already there: `SRC/x` typed is in the `src` the tree shows on Windows and macOS. */
  function spelled(path: string): string {
    const dir = parentOf(path);
    if (!dir) return path;
    const lower = dir.toLowerCase() + '/';
    const known = [...(tree?.files ?? []), ...madeDirs.map((d) => d + '/')].find((f) => f.toLowerCase().startsWith(lower));
    return known ? `${known.slice(0, dir.length)}/${basename(path)}` : path;
  }

  /** Creates the file or folder `name` the field `mine` names; what refused it, for the field to show. */
  async function create(mine: typeof adding, name: string): Promise<string | null> {
    if (!mine) return null;
    const pid = project.id;
    const src = mine.source;
    const path = newFilePath(mine.dir, name);
    busy = true;
    try {
      await (mine.kind === 'dir' ? api.fsMkdir : api.fsCreate)(pid, sourceAgent(src), path);
    } catch (e) {
      // Its field gone meanwhile (the source changed), the refusal is told otherwise.
      if (adding !== mine) app.toast(`Création impossible : ${e}`, 'error');
      return String(e);
    } finally {
      busy = false;
    }
    adding = null;
    if (mine.kind === 'dir') {
      // Shown though git lists no empty folder, the next file made in it, the focus on it.
      if (!current(pid, src)) return null;
      const real = spelled(path);
      madeDirs = [...madeDirs, real];
      lastDir = { source: src, dir: real };
      app.expandEditorDir(pid, src, parentOf(real));
      fileTree?.focusPath('dir', real);
      return null;
    }
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

  /** A terminal in the folder `dir` of the source shown ('' for its root, named after the agent or the project). */
  function terminalHere(dir: string) {
    const agentId = sourceAgent(source);
    terminalIn(project.id, dir ? { agentId, subdir: dir } : { agentId }, dir ? basename(dir) : (srcAgent?.name ?? project.name));
  }

  /** Opens the field renaming the file or folder of the row `r`, in its place. */
  function startRename(r: TreeRow) {
    if (busy || r.kind === 'new') return;
    adding = null;
    renaming = { source, kind: r.kind, path: r.path };
  }

  /** Why the file or folder renamed cannot take `name`: as for a creation, and not over an unsaved file left open. */
  function renameCheck(name: string): string | null {
    const r = renaming;
    if (!r) return null;
    const problem = renameError(name, r.path, r.kind, names());
    if (problem || !name.trim()) return problem;
    // A tab whose file is gone from the disk can hold changes at the new path: they would be taken over.
    const left = buffers.inTheWay(project.id, r.source, r.path, newFilePath(parentOf(r.path), name));
    return left ? `« ${basename(left.path)} » est ouvert avec des modifications non enregistrées.` : null;
  }

  /** Renames the file or folder of the field `mine` to `name`, its tabs following; what refused it, for the field to show. */
  async function rename(mine: typeof renaming, name: string): Promise<string | null> {
    if (!mine) return null;
    const pid = project.id;
    const src = mine.source;
    const to = newFilePath(parentOf(mine.path), name);
    if (to === mine.path) {
      cancelRename();
      return null;
    }
    busy = true;
    try {
      await api.fsRename(pid, sourceAgent(src), mine.path, to);
    } catch (e) {
      if (renaming !== mine) app.toast(`Renommage impossible : ${e}`, 'error');
      return String(e);
    } finally {
      busy = false;
    }
    renaming = null;
    // Followed right away, not once the tree is read again: the tabs never show the old path missing meanwhile. Not
    // over an unsaved file typed in at the new path since the name was checked: its tab is left as it is.
    const real = spelled(to);
    try {
      app.renameEditorPath(pid, src, mine.path, real);
    } catch (e) {
      app.toast(`Les onglets restent à l’ancien nom : ${(e as Error).message}`, 'error');
    }
    if (current(pid, src)) {
      madeDirs = madeDirs.map((d) => movedPath(d, mine.path, real) ?? d);
      if (lastDir?.source === src) lastDir = { source: src, dir: movedPath(lastDir.dir, mine.path, real) ?? lastDir.dir };
    }
    app.expandEditorDir(pid, src, parentOf(real));
    const before = trees.get(pid, src);
    await refresh(pid, src, false);
    if (!current(pid, src)) return null;
    fileTree?.focusPath(mine.kind, real);
    // What git makes of the new name, as a file created is told: out of the tree, or no longer kept out of a commit.
    const t = trees.get(pid, src);
    const fresh = mine.kind === 'file' && t && t !== before && !t.truncated ? t : null;
    if (fresh && !fresh.files.includes(real)) app.toast(`${basename(real)} est ignoré par git : l’arborescence ne le montre pas.`);
    else if (fresh && before?.ignored?.includes(mine.path) && !fresh.ignored?.includes(real))
      app.toast(`${basename(real)} n’est plus ignoré par git : il peut être commité.`);
    return null;
  }

  /** The rename given up: the focus goes back to its row, unless something else took it (a click elsewhere). */
  async function cancelRename() {
    const r = renaming;
    renaming = null;
    await drawn();
    const free = !document.activeElement || document.activeElement === document.body;
    if (r && free && current(project.id, r.source)) fileTree?.focusPath(r.kind, r.path);
  }

  /**
   * Asks for each unsaved file of `paths` in turn whether to save it first, as closing its tab does, then goes on with
   * `then`, given the files whose changes the user gave up (their text then: one typed in since is not given up).
   * Cancelled, or a save refused (the file changed on disk: its tab shows why), it goes no further.
   */
  function askToSave(
    pid: string,
    src: string,
    paths: string[],
    then: (discarded: Map<string, string>) => void,
    discarded = new Map<string, string>(),
  ) {
    const [path, ...rest] = paths;
    if (path === undefined) return then(discarded);
    const key = buffers.key(pid, src, path);
    const next = () => askToSave(pid, src, rest, then, discarded);
    app.modal = {
      kind: 'confirm',
      title: `Enregistrer « ${basename(path)} » ?`,
      body: 'Ses modifications seront perdues si tu ne les enregistres pas.',
      confirm: 'Enregistrer',
      alt: {
        label: 'Ne pas enregistrer',
        onClick: () => {
          const b = buffers.all[key];
          if (b) discarded.set(key, b.text);
          next();
        },
      },
      onConfirm: async () => {
        if (await saveKey(key)) next();
        // Refused: show the tab, the banner telling why is only drawn for the file on screen.
        else app.openEditor({ projectId: pid, source: src, path });
      },
    };
  }

  /** Sends the file or folder of the row `r` to the trash once confirmed, its unsaved files saved first or not. */
  function remove(r: TreeRow) {
    if (r.kind === 'new') return;
    const kind = r.kind;
    const pid = project.id;
    const src = source;
    const gone = (p: string) => movedPath(p, r.path, r.path) !== null;
    const unsaved = Object.values(buffers.all)
      .filter((b) => b.projectId === pid && b.source === src && gone(b.path) && buffers.isDirty(b))
      .map((b) => b.path);
    // Counted in the tree the user sees: the files git ignores in it go too, uncounted.
    const n = (tree?.files ?? []).filter(gone).length;
    const body =
      kind === 'file'
        ? 'Il part dans la corbeille.'
        : n > 1
          ? `Le dossier et ses ${n} fichiers partent dans la corbeille.`
          : n
            ? 'Le dossier et son fichier partent dans la corbeille.'
            : 'Le dossier part dans la corbeille.';
    askToSave(pid, src, unsaved, (discarded) => {
      app.modal = {
        kind: 'confirm',
        title: `Supprimer « ${basename(r.path)} » ?`,
        body,
        confirm: 'Supprimer',
        danger: true,
        onConfirm: async () => {
          try {
            await api.fsDelete(pid, sourceAgent(src), r.path);
          } catch (e) {
            app.toast(`Suppression impossible : ${e}`, 'error');
            return;
          }
          // The dialog can be closed while the trash works: a file typed in meanwhile keeps its tab and its text.
          app.closeEditorPath(pid, src, r.path, discarded);
          if (current(pid, src)) {
            madeDirs = madeDirs.filter((d) => !gone(d));
            if (lastDir?.source === src && gone(lastDir.dir)) lastDir = null;
            if (renaming?.source === src && gone(renaming.path)) renaming = null;
            if (adding?.source === src && gone(adding.dir)) adding = null;
          }
          await refresh(pid, src, false);
          // Its row gone, the tree's Tab stop takes the focus.
          if (current(pid, src)) fileTree?.focusPath(kind, r.path);
        },
      };
    });
  }

  function treeMenu(e: MouseEvent, r: TreeRow | null) {
    const dir = !r ? '' : r.kind === 'dir' ? r.path : parentOf(r.path);
    const items: MenuItem[] = [
      { label: 'Nouveau fichier…', onClick: () => startNew(dir) },
      { label: 'Nouveau dossier…', onClick: () => startNew(dir, 'dir') },
      { label: 'Ouvrir un terminal ici', onClick: () => terminalHere(dir) },
    ];
    // The root of the source is neither renamed nor deleted.
    if (r) {
      items.push(
        { label: '', separator: true },
        { label: 'Renommer…', hint: 'F2', onClick: () => startRename(r) },
        { label: 'Supprimer', hint: IS_MAC ? '⌘⌫' : 'Suppr', danger: true, onClick: () => remove(r) },
      );
    }
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

  const rows = $derived(tree ? treeRows(tree.files, place?.expanded ?? {}, status, addingDir, tree.ignored, madeDirs) : []);
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

  /** The changes against the reference version shown in the text (« Voir les changements »), for each file that has one. */
  let changesShown = $state(false);
  /** The text of the reference version, null without one (a new file, or not a repository). */
  const reference = $derived(buf?.kind === 'text' && typeof buf.base?.text === 'string' ? buf.base.text : null);
  /** The file shown is compared with its version on disk (« Comparer »), until « Recharger » or « Garder ma version ». */
  const onDisk = $derived(buf?.kind === 'text' && buf.disk === 'changed' ? buf.onDisk : null);
  const compare = $derived<Comparison | null>(
    onDisk
      ? { original: onDisk.text, against: 'disk' }
      : changesShown && reference !== null
        ? { original: reference, against: 'reference' }
        : null,
  );

  const NEWER_ON_DISK = 'Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version.';
  /**
   * The file (its buffer's id, which a rename keeps) whose « Garder ma version » waits a second, a newer version having
   * just taken the place of the one compared on screen: a click aimed at the one shown before is not taken.
   */
  let settling = $state<number | null>(null);
  let settleTimer: ReturnType<typeof setTimeout> | undefined;
  /**
   * For each file compared with the disk (by its buffer's id: a rename keeps it), the version last seen on screen: a
   * newer one is told when it takes its place there, or when the file is shown again after it did meanwhile.
   */
  const seenOnDisk = new Map<number, string>();
  $effect(() => {
    const id = buf?.id ?? null;
    const hash = onDisk?.hash ?? null;
    const replaced = onDisk?.replaced ?? null;
    untrack(() => {
      if (id === null) return;
      const before = seenOnDisk.get(id);
      if (hash) seenOnDisk.set(id, hash);
      else seenOnDisk.delete(id);
      if (!replaced || !before || before === hash) return;
      app.toast(
        replaced === 'save'
          ? 'Le fichier a encore changé sur le disque : rien n’est enregistré, la comparaison montre sa nouvelle version.'
          : NEWER_ON_DISK,
      );
      settling = id;
      clearTimeout(settleTimer);
      settleTimer = setTimeout(() => (settling = null), 1000);
    });
  });
  onDestroy(() => clearTimeout(settleTimer));

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

  // « Ouvrir un fichier » (Ctrl+P) opens its file through here, for Alt+← to come back to the place left.
  onMount(() => setEditorJump((t) => jump(t)));

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

  /** The search through the source's files, kept while the editor is closed. */
  const search = fileSearches.of(untrack(() => project.id));
  let panel = $state<ReturnType<typeof SearchPanel>>();
  let code = $state<ReturnType<typeof CodeEditor>>();

  // What the source shown has: another source is searched again.
  $effect(() => {
    const src = source;
    untrack(() => search.setSource(src));
  });

  /** The search in the left column, its field focused. */
  async function showSearch() {
    search.shown = true;
    await drawn();
    panel?.focus();
  }

  /** Ctrl+Maj+F: the search, with the code's selection to find when the focus is on it and it holds on one line. */
  function findInFiles() {
    const selected = document.activeElement?.closest('.cm-editor') ? (code?.selectedText() ?? '') : '';
    if (selected && !/[\r\n]/.test(selected)) search.seed(selected);
    showSearch();
  }
  onMount(() => setFindInFiles(findInFiles));

  /** Maj+F12: where `name` is used in the source's files, the whole word in its case (« Trouver les références »). */
  function references(name: string) {
    search.ask({ text: name, wholeWord: true, caseSensitive: true, regex: false });
    showSearch();
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
    askToSave(project.id, src, [path], drop);
  }

  /** Saves what was typed over the disk; compared with it, over the version compared only (else it shows the newer one). */
  async function keep(key: string) {
    if (settling !== null && settling === buffers.all[key]?.id) return;
    const compared = !!buffers.all[key]?.onDisk;
    let kept: boolean;
    try {
      kept = await buffers.keepMine(key);
    } catch (e) {
      app.toast(`Enregistrement impossible : ${e}`, 'error');
      return;
    }
    // Refused, then found back at the version first read: the banner goes with nothing saved.
    if (!kept && compared && buffers.all[key]?.disk === 'ok')
      app.toast('Rien n’a été enregistré : le fichier est revenu à la version que tu avais ouverte.');
  }
  const compareDisk = (key: string) => buffers.compare(key).catch((e) => app.toast(`Comparaison impossible : ${e}`, 'error'));
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
  <div class="body" use:observeWidth={(w) => (area = w)}>
    <!-- The left column: the files' tree or the search, both kept to find them again as they were. Not `.side`, the
         sidebar's class: one locator must not find both. -->
    <aside class="editor-side" style:width="{treeShown}px">
      <div class="views">
        <div class="segmented" role="group" aria-label="Vue de la colonne">
          <button
            class:on={!search.shown}
            aria-pressed={!search.shown}
            aria-label="Fichiers"
            title="Fichiers"
            onclick={() => (search.shown = false)}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
              ><path d="M9 2.5H5A1.5 1.5 0 0 0 3.5 4v8A1.5 1.5 0 0 0 5 13.5h6a1.5 1.5 0 0 0 1.5-1.5V6M9 2.5 12.5 6M9 2.5V6h3.5" /></svg
            >
          </button>
          <button
            class:on={search.shown}
            aria-pressed={search.shown}
            aria-label="Rechercher dans les fichiers"
            title={`Rechercher dans les fichiers (${keyLabel('Ctrl+Maj+F')})`}
            onclick={showSearch}
          >
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
              ><circle cx="7" cy="7" r="4.2" /><path d="M10.2 10.2 13.5 13.5" /></svg
            >
          </button>
        </div>
      </div>
      <div class="view" hidden={search.shown}>
        <div class="ftitle">
          <div class="row">
            <span class="label">Fichiers</span>
            <div class="actions">
              <button class="act" aria-label="Nouveau fichier" title="Nouveau fichier" disabled={!tree} onclick={() => newHere('file')}>
                <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
                  ><path
                    d="M8.5 2H4.5A1.5 1.5 0 0 0 3 3.5v9A1.5 1.5 0 0 0 4.5 14H8M8.5 2 13 6.5M8.5 2v4.5H13M13 6.5V9M12 10.5v4M10 12.5h4"
                  /></svg
                >
              </button>
              <button class="act" aria-label="Nouveau dossier" title="Nouveau dossier" disabled={!tree} onclick={() => newHere('dir')}>
                <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true"
                  ><path
                    d="M8 13H3.5A1.5 1.5 0 0 1 2 11.5v-7A1.5 1.5 0 0 1 3.5 3h2.6l1.5 1.5h4.9A1.5 1.5 0 0 1 14 6v2.5M12 10.5v4M10 12.5h4"
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
            bind:this={fileTree}
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
            check={(name) => newFileError(name, addingDir ?? '', names(), adding?.kind)}
            oncreate={(name) => create(adding, name)}
            oncancel={() => (adding = null)}
            adding={adding?.kind}
            renaming={renaming?.source === source ? renaming : null}
            {renameCheck}
            onrename={(name) => rename(renaming, name)}
            onrenamecancel={cancelRename}
            onrenamerow={startRename}
            ondeleterow={remove}
          />
        </div>
      </div>
      <div class="view" hidden={!search.shown}>
        <SearchPanel bind:this={panel} {search} onopen={(m) => jump({ path: m.path, line: m.line, col: m.col })} />
      </div>
    </aside>
    <Splitter
      value={treeShown}
      min={TREE_MIN}
      max={treeLimit}
      reset={TREE_DEFAULT}
      label="Largeur de la colonne des fichiers"
      onresize={(w) => (treeWidth = w)}
      oncommit={writeTreeWidth}
    />
    <section class="pane">
      <EditorTabs {tabs} onselect={(p) => app.openEditor({ projectId: project.id, source, path: p })} onclose={closeTab} />
      {#if !activePath}
        <div class="empty">Sélectionne un fichier dans l’arborescence.</div>
      {:else}
        <div class="crumbs mono">
          <span class="path">{activePath.split('/').join('  /  ')}</span>
          <div style="flex:1"></div>
          <span class:add={!!diffLabel && !diffLabel.startsWith('Identique')}>{diffLabel}</span>
          <!-- The comparison with the disk takes the text while it lasts: its banner tells what is shown. -->
          {#if reference !== null && !onDisk}
            <!-- A toggle keeps its name, `aria-pressed` and its look say whether it is on. -->
            <button class="changes" aria-pressed={changesShown} onclick={() => (changesShown = !changesShown)}>Voir les changements</button>
          {/if}
        </div>
        {#if buf?.disk === 'changed'}
          <div class="banner" role="alert">
            {onDisk ? (onDisk.replaced ? NEWER_ON_DISK : 'Comparaison avec la version du disque.') : 'Ce fichier a changé sur le disque.'}
            <button class="btn small" onclick={() => reload(buf.key)}>Recharger</button>
            {#if !onDisk}
              <button class="btn small" onclick={() => compareDisk(buf.key)}>Comparer</button>
            {/if}
            <!-- Held back, not disabled: a disabled button would lose the focus it has. -->
            <button class="btn small" aria-disabled={settling === buf.id} onclick={() => keep(buf.key)}>Garder ma version</button>
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
            bind:this={code}
            docKey={`${buf.id}`}
            text={buf.text}
            version={buf.version}
            {language}
            {indent}
            {changes}
            {compare}
            {reveal}
            nav={{ path: activePath, files: tree?.files ?? [], resolvers }}
            onrevealed={revealed}
            onchange={(t) => buffers.edit(buf.key, t)}
            oncursor={(c) => (cursor = c)}
            ontargets={follow}
            onnaverror={(e) => alive && app.toast(`Navigation impossible : ${e}`, 'error')}
            onback={() => travel(true)}
            onforward={() => travel(false)}
            onreferences={references}
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
  .editor-side {
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .views {
    flex: none;
    display: flex;
    padding: 10px 14px 0 14px;
  }
  .views button {
    display: flex;
    align-items: center;
    padding: 0 6px;
  }
  .views svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .view {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .view[hidden] {
    display: none;
  }
  .ftitle {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 14px 10px 16px;
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
  .changes {
    height: 22px;
    flex: none;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-family: var(--ui);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .changes:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .changes[aria-pressed='true'] {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
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
  /* As the app's disabled buttons look. */
  .banner .btn[aria-disabled='true'] {
    opacity: 0.5;
    cursor: default;
    border-color: var(--line2);
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

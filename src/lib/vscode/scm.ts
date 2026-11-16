// The changes from the base in the VS Code view (ESC-476, A7), plugged on the workbench: VS Code's quick diff (a source
// control per folder of the workspace, with no input box nor view of its own: only its original resource), the base
// served under `escouade-base:` by a text document content provider (read-only by nature: nothing in VS Code writes
// one), the letters of the explorer (a file decoration provider, their folders marked by VS Code), the list
// « Changements » under the explorer (a deleted file shows and opens there), and the comparisons: with the base
// (VS Code's diff editor) and with the file saved (VS Code's own « Compare with Saved »).
//
// The view of VS Code's source control (its commit and push buttons) is not there: the service override taken is the
// quick diff's alone (`@codingame/monaco-vscode-scm-service-override`'s common part registers no view), and no source
// control here has an input box or a command to accept it. Escouade's commits and pushes stay in its panel « Git ».

import * as vscode from 'vscode';
import { onDiskChange, onGitChange } from '../disk-changes';
import { t } from '../i18n';
import { api } from '../ipc';
import { BASE_SCHEME, BaseChanges, changedLines, type Changed } from './changes';
import type { EscouadeFileSystem, Source } from './fs-provider';
import { viewShown } from './shown.svelte';

/** The commands, menus, view and colours the page's extension contributes to VS Code for them. */
export function contributions() {
  const category = t('editor.vscode.category');
  const file = 'resourceScheme == file';
  return {
    commands: [
      { command: 'escouade.compareWithBase', title: t('editor.vscode.compareBase'), category, icon: '$(git-compare)' },
      { command: 'escouade.compareWithSaved', title: t('editor.vscode.compareSaved'), category },
      { command: 'escouade.openChange', title: t('editor.vscode.compareBase'), category },
    ],
    menus: {
      commandPalette: [
        { command: 'escouade.compareWithBase', when: 'editorIsOpen' },
        { command: 'escouade.compareWithSaved', when: 'activeEditorIsDirty' },
        { command: 'escouade.openChange', when: 'false' },
      ],
      'editor/title/context': [
        { command: 'escouade.compareWithBase', when: file, group: '1_compare@1' },
        { command: 'escouade.compareWithSaved', when: `${file} && activeEditorIsDirty`, group: '1_compare@2' },
      ],
      'explorer/context': [{ command: 'escouade.compareWithBase', when: `${file} && !explorerResourceIsFolder`, group: '3_compare@1' }],
    },
    views: { explorer: [{ id: 'escouade.changes', name: t('editor.vscode.changes') }] },
    viewsWelcome: [{ view: 'escouade.changes', contents: t('editor.vscode.noChanges') }],
    // git's keys of VS Code, which its git extension (not here) registers: laid from Escouade's tokens (`theme.ts`).
    colors: [
      ['gitDecoration.addedResourceForeground', '#587c0c', '#81b88b'],
      ['gitDecoration.modifiedResourceForeground', '#895503', '#e2c08d'],
      ['gitDecoration.deletedResourceForeground', '#ad0707', '#c74e39'],
      ['gitDecoration.untrackedResourceForeground', '#007100', '#73c991'],
    ].map(([id, light, dark]) => ({ id, description: id, defaults: { light, dark, highContrast: dark, highContrastLight: light } })),
  };
}

const same = (a: Source, b: Source) => a.projectId === b.projectId && a.agentId === b.agentId;
const sourceName = (s: Source) => s.agentId ?? 'project';

let changes: BaseChanges<vscode.Uri> | null = null;
let fs: EscouadeFileSystem<vscode.Uri> | null = null;
const controls: { source: Source; control: vscode.SourceControl }[] = [];
/** The names of the sources' folders, for the list of the changes. */
const names = new Map<string, string>();
const keyOf = (s: Source) => `${s.projectId}\n${s.agentId ?? ''}`;
let refreshList: () => void = () => {};

const LETTER_KEYS = { A: 'added', M: 'modified', D: 'deleted' } as const;

/** The letter of a file, said: « Modifié par rapport à main ». */
const words = (letter: Changed['letter'] | 'folder', reference: string | null) =>
  t(`editor.vscode.letter.${letter === 'folder' ? 'folder' : LETTER_KEYS[letter]}`, { reference: reference ?? t('editor.vscode.theBase') });

/** The changes followed: the providers registered once, when the workbench starts. */
export function followChanges(files: EscouadeFileSystem<vscode.Uri>, fold: boolean) {
  fs = files;
  const c = (changes = new BaseChanges<vscode.Uri>(
    api,
    {
      uri: (u) => vscode.Uri.from({ scheme: u.scheme, authority: u.authority, path: u.path, query: u.query ?? '' }),
      sourceOf: (u) => files.sourceOf(u),
      uriOf: (s, rel) => files.uriOf(s, rel),
    },
    fold,
  ));

  const bases = new vscode.EventEmitter<vscode.Uri>();
  c.onDidChange((u) => bases.fire(u));
  vscode.workspace.registerTextDocumentContentProvider(BASE_SCHEME, {
    onDidChange: bases.event,
    provideTextDocumentContent: async (u) => (await c.text(u)) ?? '',
  });

  const decorated = new vscode.EventEmitter<vscode.Uri[]>();
  c.onDidChangeFiles((uris) => {
    // In both spellings of a drive (`/c:/`, `/C:/`): VS Code keeps the decorations it asked for by the URI it asked
    // with, and some of its URIs have the drive in capitals.
    decorated.fire(uris.flatMap((u) => [u, ...(fold ? driveFlipped(u) : [])]));
    refreshList();
  });
  vscode.window.registerFileDecorationProvider({
    onDidChangeFileDecorations: decorated.event,
    provideFileDecoration: (u) => {
      const d = c.decoration(u, words);
      if (!d) return undefined;
      const deco = new vscode.FileDecoration(d.badge, d.tooltip, new vscode.ThemeColor(d.color));
      deco.propagate = d.propagate;
      return deco;
    },
  });

  listChanges(c);

  vscode.commands.registerCommand('escouade.compareWithBase', (uri?: unknown) =>
    compareWithBase(uri instanceof vscode.Uri ? uri : undefined),
  );
  vscode.commands.registerCommand('escouade.openChange', (uri: vscode.Uri, deleted: boolean) => compareWithBase(uri, deleted));
  vscode.commands.registerCommand('escouade.compareWithSaved', (uri?: vscode.Uri) =>
    vscode.commands.executeCommand('workbench.files.action.compareWithSaved', uri ?? activeFile()),
  );
  viewShown.serve(() => compareWithBase());

  // The file on disk, or the project's git state, changed: the bases and the files changed read again, at most once in
  // a while for a project an agent writes into.
  const soon = new Map<string, ReturnType<typeof setTimeout>>();
  const changed = (projectId: string) => {
    clearTimeout(soon.get(projectId));
    soon.set(
      projectId,
      setTimeout(() => {
        soon.delete(projectId);
        void c.projectChanged(projectId);
      }, 300),
    );
  };
  onDiskChange((d) => changed(d.projectId));
  onGitChange(changed);

  // « n lignes modifiées par rapport à main », in the view's bar: the changes VS Code's quick diff found in the editor.
  const tell = () => {
    const ed = vscode.window.activeTextEditor;
    const at = ed && files.sourceOf(ed.document.uri);
    if (!ed || !at) return viewShown.set(null);
    let infos: DiffInformation[] | undefined;
    try {
      infos = (ed as { diffInformation?: DiffInformation[] }).diffInformation;
    } catch {
      infos = undefined;
    }
    const info = infos?.find((d) => d.original?.scheme === BASE_SCHEME);
    if (!info?.original) return viewShown.set(null);
    viewShown.set({
      projectId: at.source.projectId,
      source: sourceName(at.source),
      path: at.rel,
      count: changedLines(info.changes),
      reference: c.reference(at.source),
      isNew: c.isNew(info.original),
    });
  };
  vscode.window.onDidChangeActiveTextEditor(tell);
  try {
    (vscode.window as unknown as { onDidChangeTextEditorDiffInformation: vscode.Event<unknown> }).onDidChangeTextEditorDiffInformation(
      tell,
    );
  } catch (e) {
    // The proposal refused (`productConfiguration` in workbench.ts): the marks without their count, the view standing.
    console.error('[vscode] diff information', e);
  }
  tell();
}

/** VS Code's diff information of an editor (`textEditorDiffInformation`, a proposal the page's extension enables). */
interface DiffInformation {
  original?: vscode.Uri;
  changes: { modified: { startLineNumber: number; endLineNumberExclusive: number } }[];
}

/**
 * A folder of the workspace: its own source control, whose original resource of a file is its base (the files of a
 * worktree in the project's folder are their worktree's, not the project's), its files changed followed.
 */
export function addSource(source: Source, uri: vscode.Uri, name: string) {
  if (!changes || !fs || controls.some((c) => same(c.source, source))) return;
  const files = fs;
  const c = changes;
  names.set(keyOf(source), name);
  // No root given to VS Code: its quick diff compares a file with a root with the case of the path, and some of its
  // models write the drive in capitals (`/C:/`) where the folder has it small (`/c:/`). The original resource below is
  // the file's own source's alone, by the file system provider, the case of the path ignored where the system does.
  void uri;
  const control = vscode.scm.createSourceControl('escouade', name);
  control.inputBox.visible = false;
  control.quickDiffProvider = {
    provideOriginalResource: async (file: vscode.Uri) => {
      const at = files.sourceOf(file);
      if (!at || !same(at.source, source)) return undefined;
      const base = c.originalOf(file);
      // Out of a repository: no base, no marks.
      return base && (await c.text(base)) !== null ? base : undefined;
    },
  };
  controls.push({ source, control });
  void c.addSource(source).then(() => refreshList());
}

/** A closed project's sources no longer followed. */
export function removeProject(projectId: string) {
  for (let i = controls.length - 1; i >= 0; i--) {
    if (controls[i].source.projectId !== projectId) continue;
    controls[i].control.dispose();
    names.delete(keyOf(controls[i].source));
    controls.splice(i, 1);
  }
  changes?.removeProject(projectId);
  refreshList();
}

/** The file of the active editor, or of the tab shown (a binary file, a preview). */
function activeFile(): vscode.Uri | undefined {
  const ed = vscode.window.activeTextEditor?.document.uri;
  if (ed) return ed;
  const input = vscode.window.tabGroups.activeTabGroup.activeTab?.input;
  if (input instanceof vscode.TabInputText) return input.uri;
  if (input instanceof vscode.TabInputTextDiff) return input.modified;
  return undefined;
}

/** The file (the active one when none is given) in VS Code's diff editor, against its base; a file deleted against nothing. */
export async function compareWithBase(uri?: vscode.Uri, deleted = false) {
  let file = uri ?? activeFile();
  if (file?.scheme === BASE_SCHEME) file = changes?.fileOf(file) ?? undefined;
  const at = file && fs?.sourceOf(file);
  const pair = file && at && changes?.comparison(file, deleted);
  if (!file || !at || !pair) return console.warn('[vscode] nothing to compare with its base:', String(file));
  // The base read first: its name (« main ») goes in the title.
  await changes!.text(pair.left);
  const name = at.rel.split('/').pop() ?? at.rel;
  const reference = changes!.reference(at.source) ?? t('editor.vscode.theBase');
  await vscode.commands.executeCommand('vscode.diff', pair.left, pair.right, `${name} (${reference}) ↔ ${name}`, { preview: false });
}

type Node = { source: Source } | { source: Source; change: Changed };

/** « Changements » under the explorer: the files changed of each source shown, opened against their base. */
function listChanges(c: BaseChanges<vscode.Uri>) {
  const changedTree = new vscode.EventEmitter<void>();
  refreshList = () => changedTree.fire();
  const shown = () => c.followed().filter((s) => controls.some((x) => same(x.source, s)));
  const filesOf = (source: Source): Node[] => c.changed(source).map((change) => ({ source, change }));
  vscode.window.createTreeView<Node>('escouade.changes', {
    showCollapseAll: true,
    treeDataProvider: {
      onDidChangeTreeData: changedTree.event,
      getChildren: (n) => {
        if (n) return 'change' in n ? [] : filesOf(n.source);
        const sources = shown();
        if (sources.length === 1) return filesOf(sources[0]);
        return sources.filter((s) => c.changed(s).length).map((source) => ({ source }));
      },
      getTreeItem: (n) => {
        if (!('change' in n)) {
          const item = new vscode.TreeItem(names.get(keyOf(n.source)) ?? n.source.projectId, vscode.TreeItemCollapsibleState.Expanded);
          item.iconPath = new vscode.ThemeIcon('repo');
          return item;
        }
        const uri = fs!.uriOf(n.source, n.change.rel)!;
        const item = new vscode.TreeItem(uri, vscode.TreeItemCollapsibleState.None);
        const dir = n.change.rel.includes('/') ? n.change.rel.slice(0, n.change.rel.lastIndexOf('/')) : '';
        item.description = dir;
        item.tooltip = `${n.change.rel} · ${words(n.change.letter, c.reference(n.source))}`;
        item.contextValue = `change-${n.change.letter}`;
        item.command = {
          command: 'escouade.openChange',
          title: t('editor.vscode.compareBase'),
          arguments: [uri, n.change.letter === 'D'],
        };
        return item;
      },
    },
  });
}

/** The URI with its drive in the other case (`/c:/` ↔ `/C:/`), when it has one. */
function driveFlipped(u: vscode.Uri): vscode.Uri[] {
  const m = /^\/([a-zA-Z]):/.exec(u.path);
  if (!m) return [];
  const d = m[1] === m[1].toLowerCase() ? m[1].toUpperCase() : m[1].toLowerCase();
  return [u.with({ path: `/${d}${u.path.slice(2)}` })];
}

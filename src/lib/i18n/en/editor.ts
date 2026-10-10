import { defineZone } from '../types';

// The code editor: its tabs, the file tree, the searches.
export default defineZone('editor', {
  toast: {
    saveFailed: 'Could not save: {error}',
    createFailed: 'Could not create: {error}',
    copyFailed: 'Could not copy: {error}',
    renameFailed: 'Could not rename: {error}',
    deleteFailed: 'Could not delete: {error}',
    compareFailed: 'Could not compare: {error}',
    navigateFailed: 'Could not navigate: {error}',
    tabsKeepOldName: 'The tabs keep the old name: {error}',
    ignoredByGit: '{name} is ignored by git: the file tree doesn’t show it.',
    noLongerIgnored: '{name} is no longer ignored by git: it can be committed.',
    fileNotFound: 'File not found: {path}',
    noDefinition: 'No definition found for “{name}”.',
    newerOnDiskUnsaved: 'The file changed on disk again: nothing was saved, and the comparison shows its new version.',
    nothingSaved: 'Nothing was saved: the file went back to the version you opened.',
    changedAgainKept: 'The file changed on disk again: nothing was saved, and your changes are still here.',
  },

  head: {
    back: '← Conversation',
    unsaved: '● Unsaved · {key}',
    saved: 'Saved',
  },

  side: {
    viewLabel: 'Column view',
    searchFiles: 'Search in files',
    searchFilesKey: 'Search in files ({key})',
    newFile: 'New file',
    newFolder: 'New folder',
    widthLabel: 'Width of the files column',
  },

  menu: {
    newFile: 'New file…',
    newFolder: 'New folder…',
    openTerminal: 'Open a terminal here',
    rename: 'Rename…',
    deleteKey: 'Del',
    copyRelativePath: 'Copy relative path',
  },

  diff: {
    newFile: 'New file · not in {reference}',
    changedLines: { one: '{n} line changed vs {reference}', other: '{n} lines changed vs {reference}' },
    same: 'Same as {reference}',
    show: 'Show changes',
  },

  banner: {
    newerOnDisk: 'The file changed on disk again: the comparison shows its new version.',
    comparing: 'Comparing with the version on disk.',
    changed: 'This file changed on disk.',
    reload: 'Reload',
    compare: 'Compare',
    keepMine: 'Keep my version',
    deleted: 'This file was deleted.',
    recreate: 'Save to recreate it',
  },

  save: {
    title: 'Save “{name}”?',
    body: 'Its changes will be lost if you don’t save them.',
    dontSave: 'Don’t save',
  },

  remove: {
    title: 'Delete “{name}”?',
    bodyFile: 'It goes to the trash.',
    bodyFolderFiles: {
      one: 'The folder and its file go to the trash.',
      other: 'The folder and its {n} files go to the trash.',
    },
    bodyFolderEmpty: 'The folder goes to the trash.',
  },

  empty: {
    selectFile: 'Select a file in the file tree.',
    binary: 'Binary file: no preview.',
    tooLarge: 'File too large for the editor ({size}).',
    missing: 'This file doesn’t exist (or no longer exists).',
  },

  status: {
    position: 'Ln {line}, Col {col}',
    tabs: 'Tabs',
    spaces: 'Spaces: {size}',
  },

  tabs: {
    label: 'Open files',
    close: 'Close {name}',
    closeIn: 'Close {name} · {folder}',
    closeUnsaved: 'Close (unsaved)',
  },

  newField: {
    fileName: 'Name of the new file',
    folderName: 'Name of the new folder',
    rename: 'Rename “{name}”',
  },

  tree: {
    ignoredByGit: 'Ignored by git',
  },

  source: {
    noBranch: 'project',
    label: 'Source: {name}',
    kindWorktree: 'worktree',
    kindBranch: 'branch',
    projectBranch: 'Project branch · {path}',
    delta: 'Δ {count}',
    clean: 'clean',
    worktree: 'worktree · {name}',
    branch: 'branch · {name}',
  },

  count: {
    files: { one: '{n} file', other: '{n} files' },
    results: { one: '{n} result', other: '{n} results' },
    changes: { one: '{n} change', other: '{n} changes' },
    listTruncated: 'list truncated',
  },

  targets: {
    title: 'Definitions of “{label}”',
  },

  lossNotice: {
    one: '{count} unsaved file in the editor will be lost.',
    other: '{count} unsaved files in the editor will be lost.',
  },

  buffers: {
    diskNotText: 'the version on disk is not text',
  },

  rename: {
    openUnsaved: '“{name}” is open with unsaved changes.',
  },

  name: {
    startsWithSlash: 'A name can’t start with a slash.',
    endsWithFile: 'The name must end with a file name.',
    endsWithDir: 'The name must end with a folder name.',
    invalidFile: '“{name}” is not a valid file name.',
    invalidDir: '“{name}” is not a valid folder name.',
    intoItself: 'A folder can’t go inside itself.',
    isAFile: '“{name}” is a file.',
    exists: '“{name}” already exists here.',
    existsIgnored: '“{name}” already exists here (ignored by git).',
  },

  compare: {
    revertBlock: 'Revert this block',
    takeBlock: 'Take this block',
  },

  language: {
    text: 'Plain text',
  },

  search: {
    invalidRegex: 'Invalid regular expression.',
    title: 'Search',
    caseSensitive: 'Match case',
    wholeWord: 'Whole word',
    regex: 'Regular expression',
    results: 'Results',
    searching: 'Searching…',
    found: '{results} in {files}',
    none: 'No results.',
    timedOut: 'Search stopped after 10 s: partial results.',
    truncated: 'Results limited to the first {max}.',
    fileRow: '{name}, {results}',
    fileRowIn: '{name}, {dir}, {results}',
    lineRow: 'Line {line}: {text}',
  },

  quickOpen: {
    title: 'Open a file',
    placeholder: 'File name, or name:42 for a line',
    loading: 'Loading files…',
    noMatch: 'No file matches.',
    firstResults: 'The first {max} results: narrow down your search.',
    opensAtLine: 'opens at line {line}',
    listTruncated: 'list truncated',
    ignored: 'ignored by git',
    keyChoose: 'choose',
    keyOpen: 'open',
    keyClose: 'close',
  },

  // CodeMirror's own English, but for the word the app uses for a chunk (« block »).
  cm: {
    accept: 'Accept',
    reject: 'Reject',
    unchangedLines: '$ unchanged lines',
    find: 'Find',
    replace: 'Replace',
    next: 'next',
    previous: 'previous',
    all: 'all',
    matchCase: 'match case',
    byWord: 'by word',
    regexp: 'regexp',
    replaceVerb: 'replace',
    replaceAll: 'replace all',
    close: 'close',
    currentMatch: 'current match',
    replacedMatches: 'replaced $ matches',
    replacedOnLine: 'replaced match on line $',
    onLine: 'on line',
    goToLine: 'Go to line',
    go: 'go',
  },
});

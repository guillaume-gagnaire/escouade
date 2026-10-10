import { defineZone } from '../types';

// The code editor: its tabs, the file tree, the searches.
export default defineZone('editor', {
  toast: {
    saveFailed: 'Could not save: {error}',
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

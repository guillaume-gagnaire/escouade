import { defineZone } from '../types';

// The code editor: its tabs, the file tree, the searches.
export default defineZone('editor', {
  toast: {
    saveFailed: 'Could not save: {error}',
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

import { defineZone } from '../types';

// The words every zone needs (see the French file).
export default defineZone('common', {
  // Actions
  cancel: 'Cancel',
  close: 'Close',
  save: 'Save',
  delete: 'Delete',
  remove: 'Remove',
  rename: 'Rename',
  edit: 'Edit',
  add: 'Add',
  create: 'Create',
  open: 'Open',
  openInEditor: 'Open in editor',
  copy: 'Copy',
  copyPath: 'Copy path',
  retry: 'Retry',
  stop: 'Stop',
  stopAll: 'Stop all',
  browse: 'Browse',
  search: 'Search',
  searchEllipsis: 'Search…',
  refresh: 'Refresh',
  show: 'Show',
  collapse: 'Collapse',
  collapseAll: 'Collapse all',
  clear: 'Clear',
  import: 'Import',
  reply: 'Reply',
  later: 'Later',

  // States
  loading: 'Loading…',
  inProgress: 'In progress',
  error: 'Error',

  // Things
  settings: 'Settings',
  overview: 'Overview',
  editor: 'Editor',
  name: 'Name',
  project: 'Project',
  agent: 'Agent',
  agents: 'Agents',
  files: 'Files',
  folder: 'Folder',
  model: 'Model',
  cost: 'Cost',
  color: 'Color',
  command: 'Command',

  // Counts
  count: {
    files: { one: '{count} file', other: '{count} files' },
    lines: { one: '{count} line', other: '{count} lines' },
    results: { one: '{count} result', other: '{count} results' },
    agents: { one: '{count} agent', other: '{count} agents' },
    projects: { one: '{count} project', other: '{count} projects' },
    tickets: { one: '{count} ticket', other: '{count} tickets' },
    loops: { one: '{count} loop', other: '{count} loops' },
  },
});

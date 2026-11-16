// What the screen of the copies (ESC-487) and the band of the copies taken back say of a file of the data folder and of
// what a copy holds: names and counts, never a value of a field.
import { t } from './i18n';
import type { BackupFile, BackupSummary } from './types';

/** The kind of a file of the data folder, by its path in it. */
export type BackupKind = 'state' | 'settings' | 'plugins' | 'integrations' | 'ticketAccounts' | 'syncQueue' | 'closed' | 'tracks' | 'other';

export function kindOf(file: string): BackupKind {
  switch (file) {
    case 'state.json':
      return 'state';
    case 'settings.json':
      return 'settings';
    case 'plugins.json':
      return 'plugins';
    case 'integrations.json':
      return 'integrations';
    case 'ticket-accounts.json':
      return 'ticketAccounts';
    case 'sync-queue.json':
      return 'syncQueue';
  }
  if (file.startsWith('closed-projects/')) return 'closed';
  if (file.endsWith('/workflow-tracks.json')) return 'tracks';
  return 'other';
}

/** What a file is, as the screen names it: a closed project's by the name its newest readable copy gives. */
export function whatOf(f: Pick<BackupFile, 'file'> & Partial<Pick<BackupFile, 'copies' | 'today'>>): string {
  const kind = kindOf(f.file);
  if (kind === 'closed') {
    const name = f.today?.name || f.copies?.find((c) => c.summary.name)?.summary.name;
    return name ? t('backups.what.closed', { name }) : t('backups.what.closedUnnamed');
  }
  if (kind === 'other') return f.file;
  return t(`backups.what.${kind}`);
}

const COUNTS = ['projects', 'agents', 'tickets', 'accounts', 'plugins', 'entries'] as const;

/** What a copy holds, in counts (« 2 projets, 5 agents, 12 tickets »); « Illisible » when it does not read. */
export function holds(s: BackupSummary): string {
  if (!s.readable) return t('backups.unreadable');
  const parts = COUNTS.filter((k) => s[k] !== undefined).map((k) => t(`backups.count.${k}`, { count: s[k] ?? 0 }));
  return parts.length ? parts.join(', ') : t('backups.readable');
}

/** The files of the screen in their two groups: the files of the data folder, then the closed projects' histories. */
export function grouped(files: BackupFile[]): { main: BackupFile[]; closed: BackupFile[] } {
  // A file not there and without copies (a feature never used: no plugin, no ticket account) says nothing.
  const shown = files.filter((f) => f.exists || f.copies.length);
  return {
    main: shown.filter((f) => kindOf(f.file) !== 'closed'),
    closed: shown.filter((f) => kindOf(f.file) === 'closed'),
  };
}

// The folder of a plugin in its repository (ESC-313, docs/STORE.md §3.6): a repository may hold several plugins, each
// in its own folder, the root ('') included. A plugin is (repository, folder); its tags say which folder they are for.
// Shared by the server (the submission, the webhook) and the page (the tags shown for the folder picked).

/** The longest folder the store takes. */
export const PATH_MAX = 200;

export type PathProblem = 'absolute' | 'parent' | 'bad';

/**
 * A folder as an author types it, normalized: `./plugins/slack/` → `plugins/slack`, `''` or `.` → the root (`''`).
 * Refused: an absolute path (`/x`, `C:\x`, `~/x`), a `..` anywhere, a backslash, a control character, a path too long.
 */
export function normalizePath(raw: unknown): { path: string } | { problem: PathProblem } {
  if (raw === undefined || raw === null) return { path: '' };
  if (typeof raw !== 'string') return { problem: 'bad' };
  const s = raw.trim();
  if (s.startsWith('/') || s.startsWith('~') || /^[A-Za-z]:/.test(s) || s.startsWith('\\')) return { problem: 'absolute' };
  if (s.includes('\\') || /[\u0000-\u001f\u007f]/.test(s) || s.length > PATH_MAX) return { problem: 'bad' };
  const parts = s.split('/').filter((p) => p !== '' && p !== '.');
  if (parts.includes('..')) return { problem: 'parent' };
  return { path: parts.join('/') };
}

/** The last segment of a folder: the plugin's name in its tags (`plugins/slack` → `slack`); empty at the root. */
export const folderName = (path: string) => path.split('/').pop() ?? '';

const NUMBER = /^v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.+-]*)?)$/;

/** What a tag says: the version it names, and the folder or the name it is for (null both: a plain tag, the root's). */
export function parseTag(tag: string): { version: string; folder: string | null; name: string | null } | null {
  const plain = NUMBER.exec(tag);
  if (plain) return { version: plain[1]!, folder: null, name: null };
  // `plugins/slack/v1.2.3` (Go's way): the folder, then the number.
  const slash = tag.lastIndexOf('/');
  if (slash > 0) {
    const n = NUMBER.exec(tag.slice(slash + 1));
    return n ? { version: n[1]!, folder: tag.slice(0, slash), name: null } : null;
  }
  // `slack@1.2.3` (changesets' way) and `slack-v1.2.3`: the folder's last segment, then the number.
  const at = /^([^@\s]+)@(.+)$/.exec(tag);
  if (at) {
    const n = NUMBER.exec(at[2]!);
    return n ? { version: n[1]!, folder: null, name: at[1]! } : null;
  }
  const dash = /^(.+)-(v\d.*)$/.exec(tag);
  if (dash) {
    const n = NUMBER.exec(dash[2]!);
    return n ? { version: n[1]!, folder: null, name: dash[1]! } : null;
  }
  return null;
}

/**
 * The version `tag` names for the plugin of folder `path`, or null if the tag is not for it. At the root: `v1.2.3` or
 * `1.2.3`. In a folder: `<folder>/v1.2.3` (or `<folder>/1.2.3`), `<name>@1.2.3` or `<name>-v1.2.3`, the name being
 * the folder's last segment.
 */
export function tagFor(tag: string, path: string): string | null {
  const p = parseTag(tag);
  if (!p) return null;
  if (!path) return p.folder === null && p.name === null ? p.version : null;
  if (p.folder !== null) return p.folder === path ? p.version : null;
  if (p.name !== null) return p.name === folderName(path) ? p.version : null;
  return null;
}

/** The tags an author may push for version `version` of the plugin of `path`, the first one advised. */
export function tagsOf(version: string, path: string): string[] {
  if (!path) return [`v${version}`, version];
  return [`${path}/v${version}`, `${folderName(path)}@${version}`, `${folderName(path)}-v${version}`];
}

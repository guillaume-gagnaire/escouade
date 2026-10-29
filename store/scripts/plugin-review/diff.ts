import { spawnSync } from 'node:child_process';

// What changed since the last verified version (docs/STORE.md §4.2, step 5): the permissions that grow first, then
// the files' diff, the only code to read in detail. The diff is git's (`git diff --no-index`) between two extracted
// folders, with what could run something of the plugin's turned off (external diff, text conversion, attributes).

export interface Grown {
  /** `permissions.network`, `run`, `contributes.webhooks`… */
  what: string;
  /** What is new, in words. */
  added: string;
}

const list = (v: unknown): string[] => (Array.isArray(v) ? v.map(String) : []);

/** What `now` asks for that `before` did not: permissions, a program, contributions. */
export function grownPermissions(before: Record<string, unknown>, now: Record<string, unknown>): Grown[] {
  const out: Grown[] = [];
  const pb = (before.permissions ?? {}) as Record<string, unknown>;
  const pn = (now.permissions ?? {}) as Record<string, unknown>;
  for (const key of Object.keys(pn).sort()) {
    const a = pn[key];
    const b = pb[key];
    if (Array.isArray(a)) {
      const added = list(a).filter((x) => !list(b).includes(x));
      if (added.length) out.push({ what: `permissions.${key}`, added: added.join(', ') });
    } else if (a === true && b !== true) out.push({ what: `permissions.${key}`, added: 'oui' });
  }
  if (now.run && !before.run) out.push({ what: 'run', added: 'lance désormais un programme' });
  const cb = Object.keys((before.contributes ?? {}) as object);
  for (const key of Object.keys((now.contributes ?? {}) as object).sort())
    if (!cb.includes(key)) out.push({ what: `contributes.${key}`, added: 'nouvelle contribution' });
  return out;
}

export interface FileDiff {
  /** The unified diff, as git writes it; empty when nothing changed. */
  text: string;
  files: number;
  added: number;
  removed: number;
}

/** The diff of folder `before` to folder `now`, both named relative to `cwd` (outside any repository), and its counts. */
export function diffFolders(cwd: string, before: string, now: string): FileDiff {
  const r = spawnSync(
    'git',
    [
      '-c',
      'core.attributesFile=',
      '-c',
      'diff.noprefix=false',
      'diff',
      '--no-index',
      '--no-ext-diff',
      '--no-textconv',
      '--no-color',
      '--stat=200',
      '--patch',
      '--',
      before,
      now,
    ],
    { cwd, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, env: { ...process.env, GIT_CONFIG_NOSYSTEM: '1', GIT_EXTERNAL_DIFF: '' } },
  );
  // 1: the folders differ; 0: they are the same.
  if (r.status !== 0 && r.status !== 1) throw new Error(`git diff : ${r.stderr || r.error?.message}`);
  const text = r.stdout ?? '';
  const m = /(\d+) files? changed(?:, (\d+) insertions?\(\+\))?(?:, (\d+) deletions?\(-\))?/.exec(text);
  return { text, files: Number(m?.[1] ?? 0), added: Number(m?.[2] ?? 0), removed: Number(m?.[3] ?? 0) };
}

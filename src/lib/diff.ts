// Minimal unified-diff parser for the diff viewer.

export interface DiffLine {
  kind: 'add' | 'del' | 'ctx' | 'meta';
  text: string;
  oldNo: number | null;
  newNo: number | null;
}

export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

export interface DiffFile {
  path: string;
  status: 'A' | 'M' | 'D';
  binary: boolean;
  /** The backend did not send this file's diff: it is over the size it sends per file. */
  tooLarge: boolean;
  add: number;
  del: number;
  hunks: DiffHunk[];
}

/** The line `git.rs` writes, after a file's header, instead of the diff it did not send (`TOO_LARGE` there). */
const TOO_LARGE = 'Diff too large';

export function parseUnifiedDiff(text: string): DiffFile[] {
  const files: DiffFile[] = [];
  let file: DiffFile | null = null;
  let hunk: DiffHunk | null = null;
  let oldNo = 0;
  let newNo = 0;
  for (const raw of text.split('\n')) {
    const line = raw.replace(/\r$/, '');
    if (line.startsWith('diff --git ')) {
      const m = line.match(/ b\/(.*)$/);
      file = { path: m ? m[1] : line.slice(11), status: 'M', binary: false, tooLarge: false, add: 0, del: 0, hunks: [] };
      files.push(file);
      hunk = null;
      continue;
    }
    if (!file) continue;
    // File headers only come before the first hunk: inside a hunk, "--- x" is a deleted "-- x".
    if (!hunk) {
      if (line.startsWith('new file')) {
        file.status = 'A';
        continue;
      }
      if (line.startsWith('deleted file')) {
        file.status = 'D';
        continue;
      }
      if (line.startsWith('Binary files')) {
        file.binary = true;
        continue;
      }
      if (line === TOO_LARGE) {
        file.tooLarge = true;
        continue;
      }
      if (/^(--- |\+\+\+ |index |similarity|rename |old mode|new mode)/.test(line)) continue;
    }
    const h = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@(.*)$/);
    if (h) {
      oldNo = Number(h[1]);
      newNo = Number(h[2]);
      hunk = { header: line, lines: [] };
      file.hunks.push(hunk);
      continue;
    }
    if (!hunk) continue;
    if (line.startsWith('+')) {
      hunk.lines.push({ kind: 'add', text: line.slice(1), oldNo: null, newNo: newNo++ });
      file.add++;
    } else if (line.startsWith('-')) {
      hunk.lines.push({ kind: 'del', text: line.slice(1), oldNo: oldNo++, newNo: null });
      file.del++;
    } else if (line.startsWith('\\')) {
      hunk.lines.push({ kind: 'meta', text: line, oldNo: null, newNo: null });
    } else if (line.length || raw.length) {
      hunk.lines.push({ kind: 'ctx', text: line.slice(1), oldNo: oldNo++, newNo: newNo++ });
    }
  }
  return files;
}

/** Converts structuredPatch hunks (from Claude's Edit/Write results) to display lines. */
/** The lines of a parsed file, each hunk preceded by its header. */
export function fileLines(file: DiffFile): DiffLine[] {
  return file.hunks.flatMap((h) => [{ kind: 'meta', text: h.header, oldNo: null, newNo: null } as DiffLine, ...h.lines]);
}

export function patchLines(hunks: { oldStart: number; newStart: number; lines: string[] }[]): DiffLine[] {
  const out: DiffLine[] = [];
  for (const h of hunks) {
    let o = h.oldStart;
    let n = h.newStart;
    if (out.length) out.push({ kind: 'meta', text: '⋯', oldNo: null, newNo: null });
    for (const l of h.lines) {
      const c = l[0];
      if (c === '+') out.push({ kind: 'add', text: l.slice(1), oldNo: null, newNo: n++ });
      else if (c === '-') out.push({ kind: 'del', text: l.slice(1), oldNo: o++, newNo: null });
      else out.push({ kind: 'ctx', text: l.slice(1), oldNo: o++, newNo: n++ });
    }
  }
  return out;
}

/**
 * The line of the file where a patch first changes it: its first hunk starts a few lines of
 * context above the change. For a deletion, the line that now sits where the removed ones were.
 */
export function firstChangedLine(hunks: { newStart: number; lines: string[] }[] | undefined): number | null {
  const first = hunks?.[0];
  if (!first) return null;
  let context = 0;
  for (const l of first.lines) {
    if (l[0] !== ' ') break;
    context++;
  }
  return Math.max(1, first.newStart + context);
}

/** Pairs deletions and additions side by side for the split view. */
export function splitRows(lines: DiffLine[]): { left: DiffLine | null; right: DiffLine | null }[] {
  const rows: { left: DiffLine | null; right: DiffLine | null }[] = [];
  let i = 0;
  while (i < lines.length) {
    const l = lines[i];
    if (l.kind === 'del') {
      const dels: DiffLine[] = [];
      const adds: DiffLine[] = [];
      while (i < lines.length && lines[i].kind === 'del') dels.push(lines[i++]);
      while (i < lines.length && lines[i].kind === 'add') adds.push(lines[i++]);
      for (let k = 0; k < Math.max(dels.length, adds.length); k++) rows.push({ left: dels[k] ?? null, right: adds[k] ?? null });
    } else if (l.kind === 'add') {
      rows.push({ left: null, right: l });
      i++;
    } else {
      rows.push({ left: l, right: l });
      i++;
    }
  }
  return rows;
}

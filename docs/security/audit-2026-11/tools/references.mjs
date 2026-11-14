// The `file:line` references of the audit's report (ESC-418): each is read at the commit of the version audited
// (AUDITED), and, to audit another version, looked for in its code by the text of the lines it cites.
//
//   node docs/security/audit-2026-11/tools/references.mjs check
//   node docs/security/audit-2026-11/tools/references.mjs locate <commit of another version> [<out.json>]
//   node docs/security/audit-2026-11/tools/references.mjs move <commit of another version> [<out.json>]
//
// `check` fails when a reference names a file or a line that is not there. `locate` says, for each reference, where
// its lines are in the other commit: `exact` (every cited line, as it is), `partial` (some: to be read), `lost`. `move`
// does the same and writes in the report the new lines of each reference found whole in the same file; the others are
// left for a hand, then AUDITED is set to that commit.
// A reference written apart from its file (`:123` after a few words) is of the last file named before it.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..', '..', '..');
const REPORT = path.resolve(HERE, '..', 'rapport.md');

/** The commit of the version audited: the 2.2.0 (the tag v2.2.0). */
const AUDITED = 'ab351a00';

const FILE =
  /((?:src-tauri|src|crates|plugins|docs|website|scripts|\.github|store)\/[\w./-]*\w\.(?:rs|ts|svelte|json|toml|yml|md|conf|mjs|sh|lock)|deny\.toml|package-lock\.json)((?:`?,? ?(?:; )?`?:\d+(?:-\d+)?)*)/g;
const APART = /`:(\d+)(?:-(\d+))?`/g;

const git = (...args) =>
  execFileSync('git', args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 1 << 28, stdio: ['ignore', 'pipe', 'ignore'] });
const cache = new Map();
/** The lines of `file` at `commit`, null when it is not there. */
function linesOf(commit, file) {
  const key = `${commit}:${file}`;
  if (!cache.has(key)) {
    try {
      cache.set(key, git('show', key).split('\n'));
    } catch {
      cache.set(key, null);
    }
  }
  return cache.get(key);
}

/** Every reference of the report: its line, its commit, its file, its lines. */
function references() {
  const lines = fs.readFileSync(REPORT, 'utf8').replace(/\r\n/g, '\n').split('\n');
  const refs = [];
  let lastFile = null;
  const commit = AUDITED;
  lines.forEach((line, i) => {
    // A file named in a section is not the file of a line of the next one.
    if (/^#{1,2} /.test(line)) lastFile = null;
    const spans = [];
    for (const m of line.matchAll(FILE)) {
      const file = m[1];
      if (file.startsWith('docs/security/')) continue;
      if (!m[2] && !file.includes('/')) continue;
      spans.push({ from: m.index, to: m.index + m[0].length, file });
      if (!m[2]) refs.push({ line: i + 1, commit, file });
      // `at` and `len`: where its lines are written in the line of the report (`move` writes the new ones there).
      for (const r of m[2].matchAll(/:(\d+)(?:-(\d+))?/g))
        refs.push({
          line: i + 1,
          commit,
          file,
          from: Number(r[1]),
          to: Number(r[2] ?? r[1]),
          at: m.index + m[1].length + r.index + 1,
          len: r[0].length - 1,
        });
    }
    for (const m of line.matchAll(APART)) {
      if (spans.some((g) => m.index >= g.from && m.index < g.to)) continue;
      const file = spans.filter((g) => g.to <= m.index).at(-1)?.file ?? lastFile;
      refs.push({
        line: i + 1,
        commit,
        file,
        from: Number(m[1]),
        to: Number(m[2] ?? m[1]),
        apart: true,
        at: m.index + 2,
        len: m[0].length - 3,
      });
    }
    if (spans.length) lastFile = spans.at(-1).file;
  });
  return refs;
}

const shown = (r) => `${r.file ?? '?'}${r.from ? `:${r.from}${r.to !== r.from ? `-${r.to}` : ''}` : ''}`;

function check() {
  const count = {};
  const seen = new Set();
  let bad = 0;
  for (const r of references()) {
    const key = `${r.commit}:${shown(r)}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const text = r.file ? linesOf(r.commit, r.file) : null;
    let why = null;
    if (!text) why = r.file ? 'no such file' : 'no file is named before it';
    else if (r.from && (r.to > text.length || r.from > r.to)) why = `the file has ${text.length} lines`;
    else if (r.from && !text.slice(r.from - 1, r.to).some((l) => l.trim())) why = 'blank lines';
    if (why) {
      bad++;
      console.log(`line ${r.line} of the report: ${shown(r)} at ${r.commit}: ${why}`);
    } else if (r.from) count[r.commit] = (count[r.commit] ?? 0) + 1;
  }
  console.log(
    `${Object.entries(count)
      .map(([c, n]) => `${n} references at ${c}`)
      .join(', ')}: ${bad} bad`,
  );
  return bad === 0;
}

const t = (s) => (s ?? '').trim();
/** Where the lines of the reference `r` are at the commit `target`. */
function locate(r, target, renames) {
  const old = linesOf(r.commit, r.file);
  if (!old) return { quality: 'lost' };
  const block = [];
  for (let i = r.from - 1; i <= r.to - 1; i++) block.push(t(old[i]));
  const telling = block.filter((l) => l).length;
  // The most telling lines of the block, then of what surrounds it: what is looked for.
  const anchors = [];
  for (let i = r.from - 4; i <= r.to + 2; i++) {
    const l = t(old[i]);
    if (l.length >= 10) anchors.push({ at: i, text: l, inBlock: i >= r.from - 1 && i <= r.to - 1 });
  }
  anchors.sort((a, b) => Number(b.inBlock) - Number(a.inBlock) || b.text.length - a.text.length);
  const candidates = new Set();
  if (linesOf(target, r.file)) candidates.add(r.file);
  if (renames.has(r.file)) candidates.add(renames.get(r.file));
  for (const a of anchors.slice(0, 3)) {
    try {
      const found = git('grep', '-l', '-F', '-e', a.text, target, '--', `*.${r.file.split('.').pop()}`);
      for (const l of found.split('\n').filter(Boolean).slice(0, 30)) candidates.add(l.slice(l.indexOf(':') + 1));
    } catch {
      // No file holds that line.
    }
  }
  let best = null;
  for (const f of candidates) {
    const now = (linesOf(target, f) ?? []).map(t);
    for (const a of anchors.slice(0, 4)) {
      for (let j = 0; j < now.length; j++) {
        if (now[j] !== a.text) continue;
        const s = j - (a.at - (r.from - 1));
        let [inBlock, around] = [0, 0];
        for (let i = -3; i <= r.to - r.from + 3; i++) {
          const o = t(old[r.from - 1 + i]);
          if (!o || o !== now[s + i]) continue;
          if (i >= 0 && i <= r.to - r.from) inBlock++;
          else around++;
        }
        const score = inBlock * 1000 + around * 10 + (f === r.file ? 1 : 0);
        if (!best || score > best.score) best = { score, file: f, from: s + 1, to: s + 1 + (r.to - r.from), inBlock, around };
      }
    }
  }
  if (!best || best.inBlock === 0) return { quality: 'lost', was: block.find((l) => l) ?? '' };
  return {
    // A line alone with nothing around it that matches may be another line of the same text: to be read.
    quality: best.inBlock < telling ? 'partial' : r.from === r.to && best.around < 2 ? 'to read' : 'exact',
    file: best.file,
    from: best.from,
    to: best.to,
    matched: `${best.inBlock}/${telling}`,
    was: block.find((l) => l) ?? '',
  };
}

function locateAll(target, out, write) {
  const moves = [];
  const renames = new Map();
  for (const l of git('diff', '-M40%', '--name-status', '--diff-filter=R', AUDITED, target).split('\n')) {
    const [, from, to] = l.split('\t');
    if (from && to) renames.set(from, to);
  }
  const tally = {};
  const found = [];
  for (const r of references()) {
    if (r.commit !== AUDITED || !r.file) continue;
    const at = r.from
      ? locate(r, target, renames)
      : { quality: linesOf(target, r.file) ? 'exact' : renames.has(r.file) ? 'renamed' : 'lost', file: renames.get(r.file) ?? r.file };
    const moved = at.quality === 'exact' && shown({ ...r, ...at }) !== shown(r);
    const kind = moved ? 'exact, at another place' : at.quality;
    tally[kind] = (tally[kind] ?? 0) + 1;
    found.push({
      line: r.line,
      reference: shown(r),
      quality: at.quality,
      now: at.file ? shown({ ...r, ...at }) : null,
      matched: at.matched,
      was: at.was,
    });
    if (at.quality !== 'exact')
      console.log(`line ${r.line}: ${shown(r)} → ${at.quality} ${at.file ? shown({ ...r, ...at }) : ''} | ${(at.was ?? '').slice(0, 90)}`);
    // What `move` writes: the lines of a reference found whole, in the same file. The rest is for a hand.
    if (moved && at.file === r.file)
      moves.push({ line: r.line, at: r.at, len: r.len, by: shown({ ...r, ...at }).slice(r.file.length + 1) });
    else if (moved) console.log(`line ${r.line}: ${shown(r)} is now ${shown({ ...r, ...at })}: another file, to write by hand`);
  }
  if (out) fs.writeFileSync(out, `${JSON.stringify(found, null, 1)}\n`);
  console.log(tally);
  if (!write) return;
  const lines = fs.readFileSync(REPORT, 'utf8').replace(/\r\n/g, '\n').split('\n');
  // From the end of each line to its start: a place written does not shift those before it.
  for (const m of moves.sort((a, b) => b.at - a.at)) {
    const l = lines[m.line - 1];
    lines[m.line - 1] = `${l.slice(0, m.at)}${m.by}${l.slice(m.at + m.len)}`;
  }
  fs.writeFileSync(REPORT, lines.join('\n'));
  console.log(`${moves.length} references moved in the report: set AUDITED to ${target}, then check`);
}

const [mode, target, out] = process.argv.slice(2);
if (mode === 'check') process.exit(check() ? 0 : 1);
else if (mode === 'locate' && target) locateAll(target, out, false);
else if (mode === 'move' && target) locateAll(target, out, true);
else {
  console.error('usage: references.mjs check | locate <commit> [<out.json>] | move <commit> [<out.json>]');
  process.exit(2);
}

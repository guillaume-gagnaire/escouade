// Takes the people out of GitLab's answers before they are kept in the repository: their answers are public on
// gitlab.com, but they are not ours to copy. `record.mjs` passes what it recorded through `anonymize`;
// `node anonymize.mjs` passes the files of this folder through it again.
//
// - a person (an object with a `username`: author, reviewers, approvers, `user` of a job or a deployment, `merged_by`,
//   assignees…) becomes an alias, the same in every file: the people sorted by their id get alice, bob, carol…, their
//   name Alice, Bob…, their profile https://gitlab.com/<alias> and their e-mails <alias>@example.com; ids are kept;
// - every other e-mail (in a text, a log) becomes user<n>@example.com, but GitLab's `git@gitlab.com`;
// - the logins and names of these people are replaced in every text too (descriptions, notes, logs).
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ALIASES = ['alice', 'bob', 'carol', 'dave', 'erin', 'frank', 'grace', 'heidi', 'ivan', 'judy', 'mallory', 'niaj'];
const EMAIL = /[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/gi;
const KEEP = new Set(['git@gitlab.com']);
const escape = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const capital = (s) => s[0].toUpperCase() + s.slice(1);

function people(values) {
  const found = new Map();
  const walk = (v) => {
    if (Array.isArray(v)) v.forEach(walk);
    else if (v && typeof v === 'object') {
      if (typeof v.username === 'string') {
        const known = found.get(v.id ?? v.username) ?? { id: v.id, usernames: new Set(), names: new Set(), emails: new Set() };
        known.usernames.add(v.username);
        if (typeof v.name === 'string' && v.name) known.names.add(v.name);
        for (const [k, x] of Object.entries(v)) if (/email$/i.test(k) && typeof x === 'string' && x) known.emails.add(x);
        found.set(v.id ?? v.username, known);
      }
      Object.values(v).forEach(walk);
    }
  };
  values.forEach(walk);
  const list = [...found.values()].sort((a, b) => Number(a.id ?? 0) - Number(b.id ?? 0));
  list.forEach((p, i) => (p.alias = ALIASES[i] ?? `user${i + 1}`));
  return list;
}

/** The answers `values` (JSON values, or texts: a log), without their people; in the same order. */
export function anonymize(values) {
  const list = people(values.filter((v) => typeof v !== 'string'));
  const byId = new Map(list.map((p) => [p.id ?? [...p.usernames][0], p]));
  // The texts to replace, the longest first (an e-mail before the login it holds).
  const swaps = [];
  for (const p of list) {
    for (const e of p.emails) swaps.push([e, `${p.alias}@example.com`]);
    for (const n of p.names) swaps.push([n, capital(p.alias)]);
    for (const u of p.usernames) swaps.push([u, p.alias]);
  }
  swaps.sort((a, b) => b[0].length - a[0].length);
  const others = new Map();
  const text = (s) => {
    let out = s.replace(EMAIL, (e) => {
      if (KEEP.has(e.toLowerCase()) || /@example\.com$/i.test(e)) return e;
      const own = list.find((p) => [...p.emails].some((x) => x.toLowerCase() === e.toLowerCase()));
      if (own) return `${own.alias}@example.com`;
      if (!others.has(e.toLowerCase())) others.set(e.toLowerCase(), `user${others.size + 1}@example.com`);
      return others.get(e.toLowerCase());
    });
    for (const [from, to] of swaps) out = out.replace(new RegExp(escape(from), 'gi'), to);
    return out;
  };
  const walk = (v) => {
    if (typeof v === 'string') return text(v);
    if (Array.isArray(v)) return v.map(walk);
    if (v && typeof v === 'object') {
      const p = typeof v.username === 'string' ? byId.get(v.id ?? v.username) : undefined;
      const out = {};
      for (const [k, x] of Object.entries(v)) {
        if (p && k === 'username') out[k] = p.alias;
        else if (p && k === 'name') out[k] = capital(p.alias);
        else if (p && k === 'web_url') out[k] = `https://gitlab.com/${p.alias}`;
        else if (p && /email$/i.test(k) && typeof x === 'string') out[k] = x ? `${p.alias}@example.com` : x;
        else out[k] = walk(x);
      }
      return out;
    }
    return v;
  };
  return values.map(walk);
}

// `node anonymize.mjs`: the files of this folder, again.
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const here = dirname(fileURLToPath(import.meta.url));
  const files = readdirSync(here).filter((f) => f.endsWith('.json') || f === 'job-log.txt');
  const values = files.map((f) => {
    const t = readFileSync(join(here, f), 'utf8');
    return f.endsWith('.json') ? JSON.parse(t) : t;
  });
  anonymize(values).forEach((v, i) => writeFileSync(join(here, files[i]), typeof v === 'string' ? v : JSON.stringify(v, null, 2) + '\n'));
  console.log(`anonymized ${files.length} files`);
}

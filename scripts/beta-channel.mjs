#!/usr/bin/env node
// The beta channel of the in-app updater (ESC-374): beta.json, always in the latest stable release,
// holds the most recent version, stable or beta. latest.json never holds a beta, so that the
// versions without « Recevoir les versions bêta » (2.1.3 and before) never see one; an app with
// the option reads beta.json first.
//   node scripts/beta-channel.mjs pick <tag> <latest.json> [<beta.json now>]
//     → the beta.json to put in place: the newer of the tag's latest.json and the beta.json of the
//       latest release now (absent or empty: the tag's). A beta's goes into the latest release, in
//       place of the one there; a stable's into its own release, before it becomes the latest.
//   node scripts/beta-channel.mjs check <tag> <beta.json>
//     → exit 0 when beta.json holds the tag's version or a newer one, else a workflow error.

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/;

/** The parts of a version (`v` and build metadata left out); null for one that is not semver. */
export function parse(version) {
  const m = SEMVER.exec(String(version ?? '').replace(/^v/, ''));
  if (!m) return null;
  return { core: [Number(m[1]), Number(m[2]), Number(m[3])], pre: m[4] ? m[4].split('.') : [] };
}

/** Semver's order: < 0 when `a` comes before `b`, 0 when equal, > 0 after. `beta.10` comes after `beta.9`. */
export function compare(a, b) {
  const x = parse(a);
  const y = parse(b);
  if (!x || !y) throw new Error(`not a version: ${!x ? a : b}`);
  for (let i = 0; i < 3; i++) if (x.core[i] !== y.core[i]) return x.core[i] - y.core[i];
  // A pre-release comes before its version.
  if (!x.pre.length || !y.pre.length) return y.pre.length - x.pre.length;
  for (let i = 0; i < Math.max(x.pre.length, y.pre.length); i++) {
    const [p, q] = [x.pre[i], y.pre[i]];
    if (p === undefined) return -1;
    if (q === undefined) return 1;
    const [pn, qn] = [/^\d+$/.test(p), /^\d+$/.test(q)];
    if (pn && qn) {
      if (Number(p) !== Number(q)) return Number(p) - Number(q);
    } else if (pn !== qn) {
      // Numbers come before words.
      return pn ? -1 : 1;
    } else if (p !== q) {
      return p < q ? -1 : 1;
    }
  }
  return 0;
}

/**
 * The beta.json to put in place for `tag`: its own latest.json (`latest`), unless the beta.json of the latest release
 * now (`current`, null when there is none) holds a newer version (a beta of the next version, while a stable one goes
 * out). The same version again (a run started again) takes the tag's, its notes fresh.
 */
export function pick(tag, latest, current) {
  const version = String(tag).replace(/^v/, '');
  if (latest?.version !== version) {
    throw new Error(`latest.json says version ${latest?.version ?? '(none)'}, but the tag ${tag} is version ${version}`);
  }
  if (current?.version && parse(current.version) && compare(current.version, version) > 0) return current;
  return latest;
}

/** What is wrong with `beta` (the parsed beta.json the updater reads) once `tag` is out; empty when it is fine. */
export function problems(tag, beta) {
  const version = String(tag).replace(/^v/, '');
  if (!beta?.version || !parse(beta.version)) return [`beta.json has no version`];
  if (compare(beta.version, version) < 0) return [`beta.json says version ${beta.version}, older than ${version}`];
  if (!beta.platforms || !Object.keys(beta.platforms).length) return [`beta.json names no platform`];
  return [];
}

/** The parsed JSON of `file`; null when it is absent or empty (no beta.json in the latest release yet). */
function readJson(file) {
  if (!file || !fs.existsSync(file)) return null;
  const text = fs.readFileSync(file, 'utf8').trim();
  return text ? JSON.parse(text) : null;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [mode, tag, ...files] = process.argv.slice(2);
  const fail = (message) => {
    console.log(`::error::${String(message).replace(/\s*\r?\n\s*/g, ' ')}`);
    process.exitCode = 1;
  };
  try {
    if (mode === 'pick' && tag && files[0]) {
      process.stdout.write(JSON.stringify(pick(tag, readJson(files[0]), readJson(files[1])), null, 2) + '\n');
    } else if (mode === 'check' && tag && files[0]) {
      const beta = readJson(files[0]);
      const found = problems(tag, beta);
      for (const problem of found) fail(problem);
      if (!found.length) console.log(`beta.json says version ${beta.version} once ${tag} is out`);
    } else {
      fail('usage: node scripts/beta-channel.mjs pick <tag> <latest.json> [<beta.json now>] | check <tag> <beta.json>');
    }
  } catch (error) {
    fail(error.message);
  }
}

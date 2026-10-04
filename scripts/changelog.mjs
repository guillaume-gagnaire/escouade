#!/usr/bin/env node
// Prints the release notes of a version: its section of CHANGELOG.md, without its heading.
//   node scripts/changelog.mjs v1.3.0 (or 1.3.0) → the section of 1.3.0, or a fallback sentence

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const FALLBACK = 'Voir les commits depuis la version précédente.';

/** The body of `## [<version>] …` in `text`, up to the next `## ` heading; FALLBACK without one. */
export function releaseNotes(text, tag) {
  const version = String(tag).replace(/^v/, '');
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
  if (start < 0) return FALLBACK;
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((l) => l.startsWith('## '));
  const body = (end < 0 ? rest : rest.slice(0, end)).join('\n').trim();
  return body || FALLBACK;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const file = path.join(root, 'CHANGELOG.md');
  const text = fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '';
  process.stdout.write(releaseNotes(text, process.argv[2] ?? '') + '\n');
}

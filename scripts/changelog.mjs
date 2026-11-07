#!/usr/bin/env node
// Prints the release notes of a version: its section of the changelog, without its heading.
//   node scripts/changelog.mjs v1.3.0 (or 1.3.0) → the section of 1.3.0 in CHANGELOG.md (French), or a fallback sentence
//   node scripts/changelog.mjs v1.3.0 --lang en  → the same from CHANGELOG.en.md (English: the GitHub release's notes)
//   node scripts/changelog.mjs v2.2.0-beta.1    → its own section, else « ## [Non publié] » (« ## [Unreleased] » in English)

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** The changelog of each language: French, the in-app update's and the French Discord's; English, GitHub's and the English Discord's. */
export const CHANGELOGS = { fr: 'CHANGELOG.md', en: 'CHANGELOG.en.md' };

export const FALLBACKS = {
  fr: 'Voir les commits depuis la version précédente.',
  en: 'See the commits since the previous version.',
};
export const FALLBACK = FALLBACKS.fr;

/** The headings of what is not out yet, in each changelog: « [Non publié] », « [Unreleased] ». */
export const UNRELEASED = ['Non publié', 'Unreleased'];

/** Whether `version` is a pre-release (`2.2.0-beta.1`): a suffix after a dash, as semver has it. */
export function isPrerelease(version) {
  return /^\d+\.\d+\.\d+-/.test(String(version).replace(/^v/, ''));
}

/**
 * The body of `## [<version>] …` in `text`, up to the next `## ` heading; `fallback` without one. A pre-release
 * (`v2.2.0-beta.1`) without a section of its own takes that of what is not out yet: a beta goes out without the
 * section being renamed.
 */
export function releaseNotes(text, tag, fallback = FALLBACK) {
  const version = String(tag).replace(/^v/, '');
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  let start = lines.findIndex((l) => l.startsWith(`## [${version}]`));
  if (start < 0 && isPrerelease(version)) {
    start = lines.findIndex((l) => UNRELEASED.some((u) => l.startsWith(`## [${u}]`)));
  }
  if (start < 0) return fallback;
  const rest = lines.slice(start + 1);
  const end = rest.findIndex((l) => l.startsWith('## '));
  const body = (end < 0 ? rest : rest.slice(0, end)).join('\n').trim();
  return body || fallback;
}

/** The language a command line asks for (`--lang en`), French by default; any other is refused. */
export function langOf(args) {
  const at = args.indexOf('--lang');
  const lang = at < 0 ? 'fr' : args[at + 1];
  if (!Object.hasOwn(CHANGELOGS, lang)) throw new Error(`Langue inconnue : ${lang} (fr ou en).`);
  return lang;
}

/** The text of the changelog of `lang` in the repository at `root`; empty when it is not there. */
export function changelogText(root, lang) {
  const file = path.join(root, CHANGELOGS[lang]);
  return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : '';
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const args = process.argv.slice(2);
  const lang = langOf(args);
  const tag = args.find((a, i) => !a.startsWith('--') && args[i - 1] !== '--lang') ?? '';
  process.stdout.write(releaseNotes(changelogText(root, lang), tag, FALLBACKS[lang]) + '\n');
}

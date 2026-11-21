#!/usr/bin/env node
// What a beta says before its notes, wherever they are read (ESC-586): the Release's notes (in English) and the two
// channels of the Discord (scripts/discord.mjs). A beta is never announced as the stable version: it says it is one,
// who gets it, and how to go back to the stable version before it.
//   node scripts/beta-notes.mjs v2.3.0-beta.1 [--lang en]   → the notice (nothing for a stable version)

import path from 'node:path';
import { fileURLToPath } from 'node:url';

const VERSION = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;
const RELEASES = 'https://github.com/guillaume-gagnaire/escouade/releases';

/** Whether `tag` is a pre-release (`v2.3.0-beta.1`), as the Release marks it. */
export function isBeta(tag) {
  const m = String(tag).match(VERSION);
  return Boolean(m && m[4]);
}

/**
 * The stable line a beta goes back to: `2.2` for a beta of 2.3.0, `2.3` for one of 2.3.4 (2.3.3 is out); null when it
 * cannot be told (a beta of 3.0.0), then the notice says « the stable version ».
 */
export function stableBefore(tag) {
  const m = String(tag).match(VERSION);
  if (!m) return null;
  const [major, minor, patch] = m.slice(1, 4).map(Number);
  if (patch > 0) return `${major}.${minor}`;
  if (minor > 0) return `${major}.${minor - 1}`;
  return null;
}

const TEXT = {
  fr: {
    word: 'bêta',
    head: (v) =>
      `> **Version bêta.** Escouade ${v} est une version d'essai, pas la version stable : elle peut avoir des défauts que la version stable n'aura plus. Elle n'arrive qu'à qui a coché « Recevoir les versions bêta » (Réglages › Général).`,
    back: (s) =>
      `> **Revenir à ${s ? `la ${s}` : 'la version stable'}** : décoche « Recevoir les versions bêta », puis réinstalle ${s ? `la dernière ${s}` : 'la dernière version stable'} depuis [la page des versions](<${RELEASES}/latest>). Tes projets, tes tickets et tes réglages restent. Décocher seul te garde sur la bêta jusqu'à la version stable qui la suit.`,
  },
  en: {
    word: 'beta',
    head: (v) =>
      `> **Beta version.** Escouade ${v} is a trial version, not the stable one: it may have flaws the stable version will not. It only reaches whoever ticked “Receive beta versions” (Settings › General).`,
    back: (s) =>
      `> **Going back to ${s ?? 'the stable version'}**: untick “Receive beta versions”, then install ${s ? `the latest ${s}` : 'the latest stable version'} again from [the releases page](<${RELEASES}/latest>). Your projects, tickets and settings stay. Unticking alone keeps you on the beta until the stable version that follows it.`,
  },
};

/** The word for a beta in `lang` (« bêta », “beta”). */
export function betaWord(lang = 'fr') {
  return (TEXT[lang] ?? TEXT.fr).word;
}

/** What a beta says before its notes, in `lang` (fr or en); empty for a stable version. */
export function betaNotice(tag, lang = 'fr') {
  if (!isBeta(tag)) return '';
  const t = TEXT[lang] ?? TEXT.fr;
  const version = String(tag).replace(/^v/, '');
  return `${t.head(version)}\n>\n${t.back(stableBefore(tag))}`;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  const at = args.indexOf('--lang');
  const lang = at >= 0 ? args[at + 1] : 'fr';
  const tag = args.find((a, i) => !a.startsWith('--') && args[i - 1] !== '--lang');
  if (!tag || !VERSION.test(tag)) {
    console.error(`beta-notes.mjs: « ${tag ?? ''} » n'est pas une version (v2.3.0-beta.1).`);
    process.exit(1);
  }
  const notice = betaNotice(tag, lang);
  if (notice) process.stdout.write(notice + '\n');
}

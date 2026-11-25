#!/usr/bin/env node
// Takes a snapshot of PiperHub's catalog of Piper voices (ESC-97), in the languages of the interface, with what the
// rule of the voices the app offers needs: the licence the model's card on Hugging Face gives (it prevails over
// PiperHub's field, sometimes wrong), its dataset's (the MODEL_CARD of the official voices, the card of a dataset's
// repo), whether it is a character's, and its files' sizes and SHA-256.
//   node scripts/piper-voices.mjs → scripts/piper-voices/snapshot.json
// Then `cargo test -p escouade-voice-local -- --ignored write_the_voice_catalog` (in src-tauri) applies the rule (`speech::catalog::admit`) and
// writes the list the app ships (crates/escouade-voice-local/src/speech/voices.json), and
// `node scripts/piper-voices-check.mjs` checks every file of it at its revision.
//
// The files are read at the commit `REVISIONS` pins for their repository (ESC-664), never `main`: rhasspy/piper-voices
// retrains its voices in place (from 2026-11-09), and the app downloads at the revision the catalog says. A new
// snapshot changes a revision on purpose, here; a repository left out is refused, its commit on `main` told.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HUB = 'https://raw.githubusercontent.com/fox3000foxy/piper-bank/main';
const HF = 'https://huggingface.co';
const LANGS = new Set(['fr', 'en']);

/** The commit each repository is read at (ESC-664): the last of rhasspy/piper-voices before its retrainings, where
 * every voice of the catalog has the sizes and SHA-256 of 2026-10-16. */
export const REVISIONS = {
  'rhasspy/piper-voices': 'e7229f7d33a10e145e109b886d661984201ac3c4',
  'nardocolin/nardocolin-pipertts': 'd79eb4895749230ff0c4f3e1b22b91bd69b5aed3',
  'DataCraftsmanAustralia/piper-en_AU-librivox-medium': '7f35faf19fe1789ece4b14233448f7c04e35a84f',
  'fox3000foxy/piper-checkpoints-fuzeiii': 'b7e2dea383761d52e0ca66d9fa89b2a9c890a1e1',
};

async function get(url, as = 'json') {
  for (let attempt = 1; ; attempt++) {
    const r = await fetch(url, { headers: { 'User-Agent': 'escouade-piper-voices' } });
    if (r.ok) return as === 'json' ? r.json() : as === 'text' ? r.text() : Buffer.from(await r.arrayBuffer());
    if (r.status === 404 || r.status === 401) return null;
    if (attempt >= 4) throw new Error(`${url}: ${r.status}`);
    await new Promise((res) => setTimeout(res, 1000 * attempt));
  }
}

/** The licence a card on Hugging Face declares (its `license` field, else its `license:` tag), or null. */
function cardLicense(info) {
  const l = info?.cardData?.license ?? info?.tags?.find((t) => t.startsWith('license:'))?.slice('license:'.length);
  if (Array.isArray(l)) return l.join(', ');
  return l && l !== 'other' ? l : (info?.cardData?.license_name ?? (l || null));
}

/** What a MODEL_CARD of rhasspy/piper-voices says of its dataset, and the voice it was finetuned from. */
export function readModelCard(text) {
  if (!text) return null;
  const dataset = text.split(/^## /m).find((s) => s.startsWith('Dataset')) ?? '';
  const url = dataset.match(/^\* URL: *(.+)$/m)?.[1]?.trim() ?? null;
  const license = dataset.match(/^\* License: *(.+)$/m)?.[1]?.trim() ?? null;
  const base = text.match(/Finetuned from ([^\n]+?)\.?$/m)?.[1]?.trim() ?? null;
  return { datasetUrl: url, datasetLicense: license, base };
}

async function files(repo, rev, paths) {
  const r = await fetch(`${HF}/api/models/${repo}/paths-info/${rev}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'User-Agent': 'escouade-piper-voices' },
    body: JSON.stringify({ paths, expand: false }),
  });
  if (!r.ok) return {};
  const list = await r.json();
  return Object.fromEntries(list.map((f) => [f.path, f]));
}

async function main() {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const models = await get(`${HUB}/src/data/models.json`);
  const datasets = await get(`${HUB}/src/data/datasets.json`);
  const datasetLicense = new Map(datasets.map((d) => [d.repo, d.license ?? null]));
  const repoCards = new Map();
  const datasetCards = new Map();
  const out = [];
  for (const m of models.filter((x) => LANGS.has(x.lang))) {
    if (!repoCards.has(m.repo)) repoCards.set(m.repo, cardLicense(await get(`${HF}/api/models/${m.repo}`)));
    const dsRepo = m.dataset?.repo ?? null;
    if (dsRepo && !datasetCards.has(dsRepo)) datasetCards.set(dsRepo, cardLicense(await get(`${HF}/api/datasets/${dsRepo}`)));
    const card =
      m.repo === 'rhasspy/piper-voices'
        ? readModelCard(await get(`${HF}/${m.repo}/raw/${rev ?? 'main'}/${path.posix.dirname(m.onnx)}/MODEL_CARD`, 'text'))
        : null;
    const rev = REVISIONS[m.repo];
    if (!rev) {
      // A voice of a repository not pinned: told, with the commit of its `main`, and kept out (no files, not admitted).
      const head = await get(`${HF}/api/models/${m.repo}/revision/main`);
      process.stderr.write(`${m.slug}: ${m.repo} has no revision in REVISIONS (its main: ${head?.sha ?? '?'})
`);
    }
    const info = rev ? await files(m.repo, rev, [m.onnx, m.json]) : {};
    const onnx = info[m.onnx];
    let json = null;
    const jsonBytes = info[m.json] ? await get(`${HF}/${m.repo}/resolve/${rev}/${m.json}`, 'bytes') : null;
    if (jsonBytes) json = { size: jsonBytes.length, sha256: crypto.createHash('sha256').update(jsonBytes).digest('hex') };
    out.push({
      slug: m.slug,
      name: m.name,
      lang: m.lang,
      quality: m.quality,
      repo: m.repo,
      revision: rev ?? null,
      onnx: m.onnx,
      json: m.json,
      speakers: m.voices,
      speakerNames: m.voiceNames,
      sampleRate: m.sampleRate,
      phonemeType: m.phonemeType ?? null,
      espeakVoice: m.configs?.espeak?.voice ?? null,
      usable: m.usable !== false,
      hub: {
        license: m.license ?? null,
        universe: m.avatar?.universe ?? null,
        certified: !!m.avatar?.certified,
        dataset: m.dataset ? { repo: dsRepo, source: m.dataset.source ?? null, license: datasetLicense.get(dsRepo) ?? null } : null,
      },
      hf: { license: repoCards.get(m.repo) ?? null, datasetLicense: dsRepo ? (datasetCards.get(dsRepo) ?? null) : null },
      card,
      files: {
        onnx: onnx ? { size: onnx.lfs?.size ?? onnx.size, sha256: onnx.lfs?.oid ?? null } : null,
        json,
      },
    });
    process.stderr.write(`${out.length} ${m.lang} ${m.slug}\n`);
  }
  const file = path.join(root, 'scripts', 'piper-voices', 'snapshot.json');
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify({ taken: new Date().toISOString().slice(0, 10), models: out }, null, 1) + '\n');
  process.stderr.write(`${out.length} voices → ${path.relative(root, file)}\n`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}

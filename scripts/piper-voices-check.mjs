#!/usr/bin/env node
// Checks every voice of the catalog the app ships (crates/escouade-voice-local/src/speech/voices.json) against the
// revision it is pinned to (ESC-664): its model's size and SHA-256 by the headers of Hugging Face (`X-Linked-Size`,
// `X-Linked-ETag`, the LFS oid), without downloading it; its settings (a small file, not in LFS) downloaded and hashed.
//   node scripts/piper-voices-check.mjs                 → the revisions of voices.json
//   node scripts/piper-voices-check.mjs <repo>=<commit> → that commit for that repository instead (to try one)
// Exits 1 when a file differs, and says which.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HF = 'https://huggingface.co';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const voices = JSON.parse(fs.readFileSync(path.join(root, 'crates/escouade-voice-local/src/speech/voices.json'), 'utf8'));
const override = Object.fromEntries(process.argv.slice(2).map((a) => a.split('=')));

async function head(url) {
  for (let attempt = 1; ; attempt++) {
    const r = await fetch(url, { method: 'HEAD', redirect: 'manual', headers: { 'User-Agent': 'escouade-piper-voices' } });
    if (r.status < 500 && r.status !== 429) return r;
    if (attempt >= 4) throw new Error(`${url}: ${r.status}`);
    await new Promise((res) => setTimeout(res, 1000 * attempt));
  }
}

async function bytes(url) {
  const r = await fetch(url, { headers: { 'User-Agent': 'escouade-piper-voices' } });
  if (!r.ok) throw new Error(`${url}: ${r.status}`);
  return Buffer.from(await r.arrayBuffer());
}

let bad = 0;
for (const v of voices) {
  const rev = override[v.repo] ?? v.revision;
  const url = (file) => `${HF}/${v.repo}/resolve/${rev}/${file}`;
  const problems = [];
  if (!/^[0-9a-f]{40}$/.test(rev ?? '')) problems.push(`revision ${rev} is not a commit`);
  else {
    const h = await head(url(v.onnx));
    const size = Number(h.headers.get('x-linked-size'));
    const sha = (h.headers.get('x-linked-etag') ?? '').replaceAll('"', '');
    if (size !== v.onnxSize) problems.push(`onnx: ${size} bytes, ${v.onnxSize} expected`);
    if (sha !== v.onnxSha256) problems.push(`onnx: sha256 ${sha || '?'}, ${v.onnxSha256} expected`);
    const json = await bytes(url(v.json));
    const jsonSha = crypto.createHash('sha256').update(json).digest('hex');
    if (json.length !== v.jsonSize) problems.push(`json: ${json.length} bytes, ${v.jsonSize} expected`);
    if (jsonSha !== v.jsonSha256) problems.push(`json: sha256 ${jsonSha}, ${v.jsonSha256} expected`);
  }
  if (problems.length) bad++;
  console.log(
    `${problems.length ? 'DIFF' : 'ok  '} ${v.id} @ ${v.repo}/${(rev ?? '').slice(0, 12)}${problems.length ? '\n     ' + problems.join('\n     ') : ''}`,
  );
}
console.log(`${voices.length - bad}/${voices.length} voices match their pinned revision`);
process.exit(bad ? 1 : 0);

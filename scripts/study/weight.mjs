#!/usr/bin/env node
// ESC-444 (study): what the VS Code view adds to the frontend. Compares two builds of it (`vite build`): main's and the
// study's. Raw, gzip and brotli (the app embeds its frontend compressed with brotli: the brotli column is about what the
// binary, then the installer, grow by). Prints a Markdown table.
//
//   node scripts/study/weight.mjs <dist of main> <dist of the branch>

import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

function files(dir) {
  const out = [];
  (function walk(d) {
    for (const e of fs.readdirSync(d, { withFileTypes: true })) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p);
      else out.push(p);
    }
  })(dir);
  return out;
}

function weigh(dir) {
  let raw = 0;
  let gz = 0;
  let br = 0;
  const all = files(dir);
  for (const f of all) {
    const b = fs.readFileSync(f);
    raw += b.length;
    gz += zlib.gzipSync(b, { level: 9 }).length;
    br += zlib.brotliCompressSync(b, { params: { [zlib.constants.BROTLI_PARAM_QUALITY]: 11 } }).length;
  }
  return { files: all.length, raw, gz, br };
}

const mb = (n) => `${(n / 2 ** 20).toFixed(1)} Mo`;
const [a, b] = process.argv.slice(2).map((d) => weigh(path.resolve(d)));
console.log('| Frontend | Fichiers | Brut | gzip | brotli |');
console.log('|---|---|---|---|---|');
console.log(`| main | ${a.files} | ${mb(a.raw)} | ${mb(a.gz)} | ${mb(a.br)} |`);
console.log(`| étude (vue VS Code) | ${b.files} | ${mb(b.raw)} | ${mb(b.gz)} | ${mb(b.br)} |`);
console.log(`| ajouté | +${b.files - a.files} | +${mb(b.raw - a.raw)} | +${mb(b.gz - a.gz)} | +${mb(b.br - a.br)} |`);

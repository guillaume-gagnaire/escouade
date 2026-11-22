#!/usr/bin/env node
// The SHA-256 of a release's installers and packages (SEC-16 of the security audit, ESC-594), in the form `sha256sum`
// writes and reads back (`<hex>  <name>`, one line per file, by name):
//   node scripts/release-sums.mjs <folder>  > SHA256SUMS.txt
// Every file of the folder, the release workflow having downloaded there the installers only (no .json, .sig, .txt).
// Exits 1 on an empty folder: a release with nothing to sum is a broken one.

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/** The SHA-256 of a file, in lower-case hex, read by chunks (an installer is ~100 MB). */
export async function sha256(file) {
  const hash = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}

/** The lines of SHA256SUMS.txt for the files of `dir`, sorted by name; a sub-folder is left out. */
export async function sums(dir) {
  const names = fs
    .readdirSync(dir, { withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => e.name)
    .sort();
  const lines = [];
  for (const name of names) lines.push(`${await sha256(path.join(dir, name))}  ${name}`);
  return lines;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const dir = process.argv[2];
  if (!dir) {
    console.error('usage: node scripts/release-sums.mjs <folder>');
    process.exit(2);
  }
  const lines = await sums(dir);
  if (lines.length === 0) {
    console.error(`::error::${dir} holds no file to sum`);
    process.exit(1);
  }
  process.stdout.write(`${lines.join('\n')}\n`);
}

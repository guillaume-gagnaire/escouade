#!/usr/bin/env node
// Keeps the app version identical in package.json, tauri.conf.json, Cargo.toml and the crate of `escouade-plugin`.
//   node scripts/version.mjs 1.2.0 (or v1.2.0, a tag) → sets the version everywhere
//   node scripts/version.mjs                         → prints the version of each manifest

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;
const CARGO_VERSION = /^(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")/;

const files = (root) => ({
  package: path.join(root, 'package.json'),
  tauri: path.join(root, 'src-tauri', 'tauri.conf.json'),
  cargo: path.join(root, 'src-tauri', 'Cargo.toml'),
  // The program the store's image builds says the same version as the app's (ESC-328).
  cli: path.join(root, 'crates', 'escouade-plugin-cli', 'Cargo.toml'),
});

export function readVersions(root) {
  const f = files(root);
  return {
    package: JSON.parse(fs.readFileSync(f.package, 'utf8')).version,
    tauri: JSON.parse(fs.readFileSync(f.tauri, 'utf8')).version,
    cargo: fs.readFileSync(f.cargo, 'utf8').match(CARGO_VERSION)?.[2],
    ...(fs.existsSync(f.cli) ? { cli: fs.readFileSync(f.cli, 'utf8').match(CARGO_VERSION)?.[2] } : {}),
  };
}

export function setVersion(root, input) {
  const version = String(input).replace(/^v/, '');
  if (!SEMVER.test(version)) throw new Error(`"${input}" is not a semver version (e.g. 1.2.0)`);
  const f = files(root);
  for (const file of [f.package, f.tauri]) {
    const json = JSON.parse(fs.readFileSync(file, 'utf8'));
    json.version = version;
    fs.writeFileSync(file, JSON.stringify(json, null, 2) + '\n');
  }
  const cargo = fs.readFileSync(f.cargo, 'utf8');
  fs.writeFileSync(f.cargo, cargo.replace(CARGO_VERSION, `$1${version}$3`));
  const cli = fs.existsSync(f.cli) ? fs.readFileSync(f.cli, 'utf8') : null;
  if (cli) fs.writeFileSync(f.cli, cli.replace(CARGO_VERSION, `$1${version}$3`));
  // The lockfiles record the app's own version too: keep them in step (dependencies untouched).
  const npmLock = path.join(root, 'package-lock.json');
  if (fs.existsSync(npmLock)) {
    const lock = JSON.parse(fs.readFileSync(npmLock, 'utf8'));
    lock.version = version;
    if (lock.packages?.['']) lock.packages[''].version = version;
    fs.writeFileSync(npmLock, JSON.stringify(lock, null, 2) + '\n');
  }
  const cargoLock = path.join(root, 'src-tauri', 'Cargo.lock');
  const names = [cargo, cli].map((t) => t?.match(/^\[package\][\s\S]*?\nname\s*=\s*"([^"]+)"/)?.[1]).filter(Boolean);
  if (names.length && fs.existsSync(cargoLock)) {
    let lock = fs.readFileSync(cargoLock, 'utf8');
    for (const name of names) {
      const entry = new RegExp(`(\nname = "${name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}"\nversion = ")[^"]+(")`);
      lock = lock.replace(entry, `$1${version}$2`);
    }
    fs.writeFileSync(cargoLock, lock);
  }
  return version;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const [first] = process.argv.slice(2);
  if (first) {
    console.log(`version set to ${setVersion(root, first)}`);
  } else {
    console.log(JSON.stringify(readVersions(root)));
  }
}

#!/usr/bin/env node
// The tags of a server's image for a release's tag (ESC-515, ESC-586), which the Release's job « server-image » puts on
// the index of the two architectures once the release is published:
//   node scripts/image-tags.mjs ghcr.io/o/r/escouade-team-server v2.3.0         → …:2.3.0 and …:latest
//   node scripts/image-tags.mjs ghcr.io/o/r/escouade-team-server v2.3.0-beta.1  → …:2.3.0-beta.1 and …:beta
// One per line. `latest` is a stable version's alone: a beta (any pre-release) never takes it, so a compose that pulls
// `latest` never gets one. A tag that is not a version is refused.

import path from 'node:path';
import { fileURLToPath } from 'node:url';

const VERSION = /^v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/;

/** The image's tags for `tag`: its version, then `latest` (stable) or `beta` (a pre-release). */
export function imageTags(image, tag) {
  if (!image || /[\s:@]$/.test(image) || /\s/.test(image)) throw new Error(`"${image}" is not an image's name`);
  const m = String(tag).match(VERSION);
  if (!m) throw new Error(`"${tag}" is not a version's tag (v1.2.3 or v1.2.3-beta.1)`);
  const version = m[1];
  const channel = version.includes('-') ? 'beta' : 'latest';
  return [`${image}:${version}`, `${image}:${channel}`];
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [image, tag] = process.argv.slice(2);
  try {
    process.stdout.write(imageTags(image, tag).join('\n') + '\n');
  } catch (e) {
    console.error(`image-tags.mjs: ${e.message}`);
    process.exit(1);
  }
}

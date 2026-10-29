// The plugins of the store's tests, valid and booby-trapped (docs/STORE.md §3.3), as the archives GitHub makes of a
// commit: a gzip of a tar, a global pax header with the commit, everything under `owner-repo-sha/`. Shared by the
// unit tests (tests/unit/checks.test.ts) and the fake GitHub of the end-to-end tests (tests/fake/github.mjs), which
// serves them as repositories with their tags, releases and attestations. Nothing here is ever run: they are bytes.
import { createHash, randomBytes } from 'node:crypto';
import { crc32, deflateSync, gzipSync } from 'node:zlib';

const BLOCK = 512;

function octal(n, len) {
  return n.toString(8).padStart(len - 1, '0') + '\0';
}

/** One tar header (ustar), its checksum computed. */
function header(name, { size = 0, type = '0', link = '' } = {}) {
  const h = Buffer.alloc(BLOCK);
  const put = (s, at, len) => Buffer.from(s, 'utf8').copy(h, at, 0, len);
  put(name, 0, 100);
  put(octal(type === '5' ? 0o755 : 0o644, 8), 100, 8);
  put(octal(0, 8), 108, 8);
  put(octal(0, 8), 116, 8);
  put(octal(size, 12), 124, 12);
  put(octal(1761700000, 12), 136, 12);
  h.fill(32, 148, 156);
  put(type, 156, 1);
  put(link, 157, 100);
  put('ustar\0', 257, 6);
  put('00', 263, 2);
  let sum = 0;
  for (const b of h) sum += b;
  put(octal(sum, 7) + ' ', 148, 8);
  return h;
}

const pad = (b) => Buffer.concat([b, Buffer.alloc((BLOCK - (b.length % BLOCK)) % BLOCK)]);

function paxRecord(key, value) {
  const body = ` ${key}=${value}\n`;
  let len = body.length + 1;
  while (String(len).length + body.length !== len) len = String(len).length + body.length;
  return `${len}${body}`;
}

/**
 * A tarball as GitHub makes it. `files`: path → text or bytes, or `{ symlink }`, `{ hardlink }`, `{ fifo: true }`;
 * `raw`: entries named as is (outside the top folder: `/etc/passwd`, `../evil`).
 */
export function tarball(files, { top = 'owner-repo-0000000', commit = '0'.repeat(40), raw = {} } = {}) {
  const parts = [];
  const global = Buffer.from(paxRecord('comment', commit));
  parts.push(header('pax_global_header', { size: global.length, type: 'g' }), pad(global));
  parts.push(header(`${top}/`, { type: '5' }));
  const entry = (name, v) => {
    if (v && typeof v === 'object' && !Buffer.isBuffer(v)) {
      if (v.symlink) return parts.push(header(name, { type: '2', link: v.symlink }));
      if (v.hardlink) return parts.push(header(name, { type: '1', link: v.hardlink }));
      if (v.fifo) return parts.push(header(name, { type: '6' }));
    }
    const data = Buffer.isBuffer(v) ? v : Buffer.from(String(v), 'utf8');
    if (Buffer.byteLength(name) > 100) {
      const long = Buffer.from(paxRecord('path', name));
      parts.push(header('PaxHeader/long', { size: long.length, type: 'x' }), pad(long));
      name = name.slice(-99);
    }
    parts.push(header(name, { size: data.length }), pad(data));
  };
  for (const [p, v] of Object.entries(files)) entry(`${top}/${p}`, v);
  for (const [p, v] of Object.entries(raw)) entry(p, v);
  parts.push(Buffer.alloc(BLOCK * 2));
  return gzipSync(Buffer.concat(parts));
}

/** A PNG of `width` × `height`, of one color, or of noise (which does not compress); with a text chunk of metadata. */
export function png(width, height, { noise = false, text = 'Author\0Somebody at 48.85,2.35' } = {}) {
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(td) >>> 0);
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 2;
  const row = width * 3 + 1;
  const pixels = noise ? randomBytes(row * height) : Buffer.alloc(row * height);
  for (let y = 0; y < height; y++) {
    pixels[y * row] = 0;
    if (!noise) for (let x = 0; x < width * 3; x++) pixels[y * row + 1 + x] = x % 3 === 2 ? 0xc0 : 0x30;
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('tEXt', Buffer.from(text, 'latin1')),
    chunk('IDAT', deflateSync(pixels)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

export const MIT = `MIT License

Copyright (c) 2026 The authors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction...
`;

/** A manifest of a plugin of `repo` (`owner/name`), with what `over` changes. */
export function manifest(repo, over = {}) {
  const name = repo.split('/')[1];
  return {
    $schema: 'https://escouade.dev/schemas/plugin-1.json',
    id: `io.github.${repo.split('/')[0]}.${name}`.toLowerCase().replace(/[^a-z0-9.-]/g, '-'),
    name: { en: 'Ntfy', fr: 'Ntfy' },
    version: '0.3.0',
    description: { en: 'Agents waiting, on your phone.', fr: 'Les agents qui attendent, sur ton téléphone.' },
    author: { name: 'Sam Roux' },
    license: 'MIT',
    repository: `https://github.com/${repo}`,
    escouade: { api: '1', min: '2.1.0' },
    permissions: { events: ['agent.waiting'], network: ['ntfy.sh'] },
    contributes: { webhooks: [{ event: 'agent.waiting', url: 'https://ntfy.sh/{topic}' }] },
    store: { categories: ['notifications'], keywords: ['phone'], screenshots: ['store/1.png'] },
    ...over,
  };
}

/** The files of a plugin of `repo`: its manifest, README, LICENSE and a screenshot, with what `over` changes. */
export function pluginFiles(repo, { manifest: m = {}, files = {} } = {}) {
  const base = {
    'escouade-plugin.json': JSON.stringify(manifest(repo, m), null, 2),
    'README.md': '# Ntfy\n\nThe agents waiting, on your phone.\n',
    LICENSE: MIT,
    'store/1.png': png(320, 200),
  };
  const out = { ...base, ...files };
  for (const [k, v] of Object.entries(out)) if (v === null) delete out[k];
  return out;
}

export const sha256 = (b) => createHash('sha256').update(b).digest('hex');

/**
 * An attestation of provenance as GitHub serves it (`GET /repos/{o}/{r}/attestations/sha256:<digest>`): a Sigstore
 * bundle whose DSSE payload is an in-toto statement of SLSA provenance. Not signed: the store reads the statement,
 * the review checks the signature on the user's machine (docs/STORE.md §4.2).
 */
export function attestation(digest, { repo, commit, workflow = '.github/workflows/release.yml' }) {
  const statement = {
    _type: 'https://in-toto.io/Statement/v1',
    subject: [{ name: 'asset', digest: { sha256: digest } }],
    predicateType: 'https://slsa.dev/provenance/v1',
    predicate: {
      buildDefinition: {
        buildType: 'https://actions.github.io/buildtypes/workflow/v1',
        externalParameters: { workflow: { ref: 'refs/tags/v0.3.0', repository: `https://github.com/${repo}`, path: workflow } },
        resolvedDependencies: [{ uri: `git+https://github.com/${repo}@refs/tags/v0.3.0`, digest: { gitCommit: commit } }],
      },
    },
  };
  return {
    bundle: {
      mediaType: 'application/vnd.dev.sigstore.bundle.v0.3+json',
      dsseEnvelope: {
        payloadType: 'application/vnd.in-toto+json',
        payload: Buffer.from(JSON.stringify(statement)).toString('base64'),
        signatures: [{ sig: 'MEUCIQ-fake' }],
      },
    },
  };
}

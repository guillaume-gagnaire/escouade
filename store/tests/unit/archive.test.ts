import { gzipSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { imageInfo, reencode } from '../../server/utils/images';
import { judgeLicense, spdxParts } from '../../server/utils/licenses';
import { readArchive } from '../../server/utils/tar';
import { canonical, compareVersions, tagVersion, versionFile } from '../../server/utils/versions';
import { png, tarball } from '../fake/plugins.mjs';

// The archive's reader, the images, the licenses and the versions: what the checks stand on (docs/STORE.md §3.3,
// §3.4). Nothing is written to disk and nothing runs: an archive is bytes in memory.

describe('the archive reader', () => {
  it('reads GitHub’s tarball: the top folder taken off, the commit of the global header', () => {
    const read = readArchive(tarball({ 'a.txt': 'hello', 'dir/b.txt': 'b' }, { commit: 'c'.repeat(40) }));
    expect(read.problems).toEqual([]);
    expect(read.comment).toBe('c'.repeat(40));
    expect([...read.files.keys()].sort()).toEqual(['a.txt', 'dir/b.txt']);
    expect(read.files.get('a.txt')!.toString()).toBe('hello');
  });

  it('reads a long name from its pax header', () => {
    const long = `${'d/'.repeat(60)}file.txt`;
    const read = readArchive(tarball({ [long]: 'x' }));
    expect(read.problems).toEqual([]);
    expect(read.files.get(long)!.toString()).toBe('x');
  });

  it('refuses links, special files, absolute paths and climbing paths, and says which', () => {
    const read = readArchive(
      tarball(
        { 'link-out': { symlink: '/etc/passwd' }, hard: { hardlink: 'a' }, pipe: { fifo: true }, ok: 'ok' },
        { raw: { '/etc/evil': 'x', 'top/../../evil': 'x', 'C:/evil': 'x' } },
      ),
    );
    expect(read.problems).toEqual([
      { code: 'symlink', path: 'link-out' },
      { code: 'hardlink', path: 'hard' },
      { code: 'special', path: 'pipe' },
      { code: 'absolute', path: '/etc/evil' },
      { code: 'parent', path: 'top/../../evil' },
      { code: 'absolute', path: 'C:/evil' },
    ]);
    expect([...read.files.keys()]).toEqual(['ok']);
  });

  it('stops a gzip bomb before it fills the memory', () => {
    const bomb = tarball({ zeros: Buffer.alloc(8 * 1024 * 1024) });
    expect(bomb.length).toBeLessThan(100_000);
    const read = readArchive(bomb, { archive: 50 * 1024 * 1024, unpacked: 1024 * 1024, entries: 100 });
    expect(read.problems).toEqual([{ code: 'bomb' }]);
    expect(read.files.size).toBe(0);
  });

  it('refuses an archive past its size, too many entries, and bytes that are no archive', () => {
    expect(readArchive(Buffer.alloc(11), { archive: 10, unpacked: 100, entries: 10 }).problems).toEqual([{ code: 'too-big', size: 11 }]);
    const many = Object.fromEntries(Array.from({ length: 12 }, (_, i) => [`f${i}`, '']));
    expect(readArchive(tarball(many), { archive: 1e6, unpacked: 1e6, entries: 10 }).problems).toEqual([{ code: 'too-many' }]);
    expect(readArchive(Buffer.from('not a gzip')).problems).toEqual([{ code: 'corrupt' }]);
    expect(readArchive(gzipSync(Buffer.alloc(512, 7))).problems).toEqual([{ code: 'corrupt' }]);
  });
});

describe('the screenshots', () => {
  it('tells PNG, JPEG and WebP by their bytes, with their size', async () => {
    const p = png(320, 200);
    expect(imageInfo(p)).toEqual({ format: 'png', width: 320, height: 200 });
    const jpeg = await reencode(p, 'jpeg');
    expect(imageInfo(jpeg)).toEqual({ format: 'jpeg', width: 320, height: 200 });
    const webp = await reencode(p, 'webp');
    expect(imageInfo(webp)).toEqual({ format: 'webp', width: 320, height: 200 });
    expect(imageInfo(Buffer.from('GIF89a\x01\x00\x01\x00'))).toBeNull();
    expect(imageInfo(Buffer.from('<svg xmlns="http://www.w3.org/2000/svg"/>'))).toBeNull();
  });

  it('re-encodes a PNG without its metadata', async () => {
    const p = png(64, 32, { text: 'Comment\0secret place' });
    expect(p.includes(Buffer.from('secret place'))).toBe(true);
    const out = await reencode(p, 'png');
    expect(imageInfo(out)).toEqual({ format: 'png', width: 64, height: 32 });
    expect(out.includes(Buffer.from('secret place'))).toBe(false);
  });

  it('refuses to decode an image whose header lies about its pixels', async () => {
    const huge = png(1, 1);
    huge.writeUInt32BE(100_000, 16);
    huge.writeUInt32BE(100_000, 20);
    await expect(reencode(huge, 'png')).rejects.toThrow();
  });
});

describe('the licenses', () => {
  it('takes a free license the file agrees with, GPL suffixes aside', () => {
    expect(judgeLicense('MIT', 'MIT', true)).toEqual({ ok: true });
    expect(judgeLicense('GPL-3.0-or-later', 'GPL-3.0', true)).toEqual({ ok: true });
    expect(judgeLicense('MIT OR Apache-2.0', 'Apache-2.0', true)).toEqual({ ok: true });
  });

  it('says why it refuses one', () => {
    expect(judgeLicense(undefined, 'MIT', true)).toMatchObject({ code: 'missing' });
    expect(judgeLicense('MIT AND BSD-3-Clause', 'MIT', true)).toMatchObject({ code: 'expression' });
    expect(judgeLicense('BUSL-1.1', 'BUSL-1.1', true)).toMatchObject({ code: 'not-free', manifest: 'BUSL-1.1' });
    expect(judgeLicense('MIT', null, false)).toMatchObject({ code: 'no-file' });
    expect(judgeLicense('MIT', 'NOASSERTION', true)).toMatchObject({ code: 'undetected' });
    expect(judgeLicense('MIT', 'Apache-2.0', true)).toEqual({ ok: false, code: 'different', manifest: 'MIT', detected: 'Apache-2.0' });
    expect(spdxParts('(MIT OR ISC)')).toEqual(['MIT', 'ISC']);
  });
});

describe('the versions', () => {
  it('reads a tag’s version and compares versions as semver does', () => {
    expect(tagVersion('v1.2.0')).toBe('1.2.0');
    expect(tagVersion('1.2.0')).toBe('1.2.0');
    expect(tagVersion('version-1')).toBe('version-1');
    expect(compareVersions('1.10.0', '1.9.0')).toBe(1);
    expect(compareVersions('1.0.0-rc.1', '1.0.0')).toBe(-1);
    expect(compareVersions('1.0.0-alpha', '1.0.0-alpha.1')).toBe(-1);
    expect(compareVersions('1.0.0-alpha.2', '1.0.0-alpha.10')).toBe(-1);
    expect(compareVersions('1.0.0-beta', '1.0.0-alpha.9')).toBe(1);
    expect(compareVersions('1.0.0+build', '1.0.0')).toBe(0);
  });

  it('writes version.json canonical: the same value, the same bytes, the same fingerprint', () => {
    expect(canonical({ b: 1, a: { d: [1, { z: 1, y: 2 }], c: 'x' }, u: undefined })).toBe('{"a":{"c":"x","d":[1,{"y":2,"z":1}]},"b":1}');
    const v = {
      id: 'io.github.sroux.ntfy',
      version: '0.3.0',
      repository: 'https://github.com/sroux/escouade-ntfy',
      tag: 'v0.3.0',
      commit: 'a'.repeat(40),
      archive: { sha256: 'b'.repeat(64), size: 10 },
      binaries: {},
      manifest_sha256: 'c'.repeat(64),
      escouade: { api: '1', min: '2.1.0' },
    };
    const one = versionFile(v);
    const two = versionFile({ ...v, escouade: { min: '2.1.0', api: '1' } });
    expect(one).toEqual(two);
    expect(one.json).not.toMatch(/\s/);
    expect(one.json.startsWith('{"archive":{"sha256":')).toBe(true);
    expect(one.sha256).toMatch(/^[0-9a-f]{64}$/);
    expect(versionFile({ ...v, commit: 'd'.repeat(40) }).sha256).not.toBe(one.sha256);
  });
});

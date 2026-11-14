// What a macOS package of the app holds of a signature and of a notarization, read on any system without running
// anything: the update archive (`.app.tar.gz`) and, when given, the disk image (`.dmg`).
//
//   node inspect-macos-package.mjs <Escouade.app.tar.gz> [<Escouade.dmg>]
//
// It tells what is THERE, never that it is valid: the validity of a signature and of a notarization ticket is Apple's to
// tell, on a Mac (`codesign --verify --deep --strict`, `spctl -a -vv`, `xcrun stapler validate`). It prints no name of
// a signer: only whether a Developer ID signature, a hardened runtime and a stapled ticket are present.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const [archive, image] = process.argv.slice(2);
if (!archive) {
  console.error('usage: node inspect-macos-package.mjs <app.tar.gz> [<dmg>]');
  process.exit(2);
}

/** The entries of a tar: `{ name, type, data }` (the data of the plain files only). */
function entries(tar) {
  const out = [];
  let longName = null;
  for (let at = 0; at + 512 <= tar.length;) {
    const head = tar.subarray(at, at + 512);
    if (head.every((b) => b === 0)) break;
    const text = (from, to) => head.subarray(from, to).toString('utf8').replace(/\0.*$/, '');
    const size = parseInt(text(124, 136).trim() || '0', 8);
    const type = String.fromCharCode(head[156] || 48);
    const data = tar.subarray(at + 512, at + 512 + size);
    at += 512 + Math.ceil(size / 512) * 512;
    // A long name comes first, in an entry of its own (GNU `L`, pax `x`).
    if (type === 'L') {
      longName = data.toString('utf8').replace(/\0.*$/, '');
      continue;
    }
    if (type === 'x') {
      const m = /\d+ path=([^\n]+)\n/.exec(data.toString('utf8'));
      if (m) longName = m[1];
      continue;
    }
    const prefix = text(345, 500);
    const name = longName ?? (prefix ? `${prefix}/${text(0, 100)}` : text(0, 100));
    longName = null;
    out.push({ name, type, data: type === '0' ? data : null });
  }
  return out;
}

/** The code directories of a Mach-O (one per architecture): whether each asks for the hardened runtime and names a team. */
function codeDirectories(bytes) {
  const found = [];
  const magic = Buffer.from([0xfa, 0xde, 0x0c, 0x02]);
  for (let at = bytes.indexOf(magic); at >= 0; at = bytes.indexOf(magic, at + 4)) {
    const length = bytes.readUInt32BE(at + 4);
    const version = bytes.readUInt32BE(at + 8);
    if (length < 44 || length > bytes.length - at || version < 0x20000 || version > 0x20fff) continue;
    const flags = bytes.readUInt32BE(at + 12);
    const team = version >= 0x20200 ? bytes.readUInt32BE(at + 48) : 0;
    found.push({ hardenedRuntime: (flags & 0x10000) !== 0, adhoc: (flags & 0x2) !== 0, namesATeam: team > 0 });
  }
  return found;
}

const has = (bytes, text) => bytes.includes(Buffer.from(text, 'latin1'));
const yes = (b) => (b ? 'present' : 'ABSENT');

const tar = zlib.gunzipSync(fs.readFileSync(archive));
const all = entries(tar);
const app = all.map((e) => e.name.replace(/^\.\//, '')).find((n) => /^[^/]+\.app\/?$/.test(n)) ?? '';
const root = app.replace(/\/$/, '');
const at = (rest) => all.find((e) => e.name.replace(/^\.\//, '') === `${root}/${rest}`);
const sealed = at('Contents/_CodeSignature/CodeResources');
const ticket = at('Contents/CodeResources');
const plist = at('Contents/Info.plist')?.data?.toString('utf8') ?? '';
const executable = /<key>CFBundleExecutable<\/key>\s*<string>([^<]+)<\/string>/.exec(plist)?.[1];
const main = executable ? at(`Contents/MacOS/${executable}`) : undefined;
const version = /<key>CFBundleShortVersionString<\/key>\s*<string>([^<]+)<\/string>/.exec(plist)?.[1];

console.log(`archive: ${path.basename(archive)} (${all.length} entries, bundle ${root || '?'}, version ${version ?? '?'})`);
console.log(`seal of the bundle (_CodeSignature/CodeResources): ${yes(sealed)}`);
console.log(`stapled notarization ticket (Contents/CodeResources): ${yes(ticket)}${ticket?.data ? `, ${ticket.data.length} bytes` : ''}`);
if (main?.data) {
  const dirs = codeDirectories(main.data);
  console.log(`main program: ${dirs.length} signed architecture(s)`);
  console.log(`  hardened runtime: ${dirs.length && dirs.every((d) => d.hardenedRuntime) ? 'asked by each' : 'NOT by each'}`);
  console.log(`  ad hoc signature (no identity): ${dirs.some((d) => d.adhoc) ? 'YES' : 'no'}`);
  console.log(`  a team identifier: ${yes(dirs.length > 0 && dirs.every((d) => d.namesATeam))}`);
  console.log(`  a certificate « Developer ID Application »: ${yes(has(main.data, 'Developer ID Application'))}`);
  console.log(`  its authority « Developer ID Certification Authority »: ${yes(has(main.data, 'Developer ID Certification Authority'))}`);
  console.log(
    `  a signed timestamp (« Apple Timestamp Certification Authority »): ${yes(has(main.data, 'Apple Timestamp Certification Authority'))}`,
  );
} else {
  console.log('main program: NOT FOUND');
}
if (image) {
  const dmg = fs.readFileSync(image);
  // A disk image ends with its « koly » block; a signed one holds its signature before it.
  const koly = dmg.subarray(dmg.length - 512, dmg.length - 508).toString('latin1') === 'koly';
  console.log(`\ndisk image: ${path.basename(image)} (${dmg.length} bytes, ${koly ? 'a disk image' : 'NOT a disk image by its end'})`);
  console.log(`  a certificate « Developer ID Application »: ${yes(has(dmg, 'Developer ID Application'))}`);
}
console.log('\nPresent is not valid: check on a Mac with codesign --verify --deep --strict, spctl -a -vv and xcrun stapler validate.');

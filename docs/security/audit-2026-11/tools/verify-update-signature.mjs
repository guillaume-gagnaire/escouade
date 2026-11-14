// Checks the update signature of an installer as the app's updater does: a minisign signature (as Tauri writes it)
// against the public key the app embeds (`plugins.updater.pubkey` of a tauri.conf.json).
//
//   node verify-update-signature.mjs <tauri.conf.json> <installer> [<installer>.sig]
//   node verify-update-signature.mjs <tauri.conf.json> --manifest <latest.json> <folder of the release's files>
//
// The first form reads the signature in the file beside the installer. The second reads what the updater itself reads:
// the manifest of a release, each platform with its signature and the address of its file, looked for by its name in
// the folder. Prints what it checked; exits 1 when a check fails. Reads public material only.
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const conf = args[0];
const usage = () => {
  console.error('usage: node verify-update-signature.mjs <tauri.conf.json> <installer> [<installer>.sig]');
  console.error('       node verify-update-signature.mjs <tauri.conf.json> --manifest <latest.json> <folder>');
  process.exit(2);
};
if (!conf || args.length < 2) usage();

/** The lines of a minisign text that Tauri wraps in base64. */
const unwrap = (text) => Buffer.from(text.trim(), 'base64').toString('utf8').split('\n');
const ed25519 = (raw) =>
  crypto.createPublicKey({
    key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), raw]),
    format: 'der',
    type: 'spki',
  });

const pubkey = JSON.parse(fs.readFileSync(conf, 'utf8')).plugins.updater.pubkey;
const key = Buffer.from(unwrap(pubkey)[1], 'base64');
const [keyAlg, keyId, publicKey] = [key.subarray(0, 2).toString(), key.subarray(2, 10), key.subarray(10)];

/** The checks of `file` against the signature `wrapped` (the text of a `.sig`, or a manifest's `signature`). */
function check(file, wrapped) {
  const lines = unwrap(wrapped);
  const sig = Buffer.from(lines[1], 'base64');
  const [alg, signer, signature] = [sig.subarray(0, 2).toString(), sig.subarray(2, 10), sig.subarray(10)];
  const trusted = lines[2].replace(/^trusted comment: /, '');
  const global = Buffer.from(lines[3], 'base64');
  const bytes = fs.readFileSync(file);
  // "ED": the signature is of the BLAKE2b-512 of the file; "Ed": of the file itself.
  const message = alg === 'ED' ? crypto.createHash('blake2b512').update(bytes).digest() : bytes;
  const checks = {
    'the key is a minisign Ed25519 key': keyAlg === 'Ed' && publicKey.length === 32,
    'the signature names the key the app embeds': signer.equals(keyId),
    'the signature of the file holds': crypto.verify(null, message, ed25519(publicKey), signature),
    'the signature of the trusted comment holds': crypto.verify(
      null,
      Buffer.concat([signature, Buffer.from(trusted)]),
      ed25519(publicKey),
      global,
    ),
  };
  console.log(`file: ${path.basename(file)} (${bytes.length} bytes, sha256 ${crypto.createHash('sha256').update(bytes).digest('hex')})`);
  console.log(`algorithm: ${alg === 'ED' ? 'Ed25519 over BLAKE2b-512 (prehashed)' : 'Ed25519'}`);
  console.log(`trusted comment: ${trusted.replace(/\t/g, ' ')}`);
  for (const [what, ok] of Object.entries(checks)) console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
  return Object.values(checks).every(Boolean);
}

let all = true;
if (args[1] === '--manifest') {
  const [manifestFile, folder] = args.slice(2);
  if (!manifestFile || !folder) usage();
  const manifest = JSON.parse(fs.readFileSync(manifestFile, 'utf8'));
  console.log(`manifest: version ${manifest.version}, ${Object.keys(manifest.platforms ?? {}).length} platforms`);
  // Several platforms share a file: each file once, with the platforms that name it.
  const files = new Map();
  for (const [platform, entry] of Object.entries(manifest.platforms ?? {})) {
    const name = decodeURIComponent(new URL(entry.url).pathname.split('/').pop());
    const at = files.get(name) ?? { signature: entry.signature, platforms: [] };
    if (at.signature !== entry.signature) {
      console.log(`FAIL ${name}: two platforms give it two signatures`);
      all = false;
    }
    at.platforms.push(platform);
    files.set(name, at);
  }
  for (const [name, { signature, platforms }] of files) {
    console.log(`\n${platforms.join(', ')}`);
    const file = path.join(folder, name);
    if (!fs.existsSync(file)) {
      console.log(`FAIL ${name} is not in the folder`);
      all = false;
      continue;
    }
    all = check(file, signature) && all;
    // The `.sig` published beside the file says the same as the manifest.
    if (fs.existsSync(`${file}.sig`)) {
      const same = fs.readFileSync(`${file}.sig`, 'utf8').trim() === signature.trim();
      console.log(`${same ? 'ok  ' : 'FAIL'} the .sig beside the file is the manifest's signature`);
      all = same && all;
    }
  }
} else {
  const [file, sigFile = `${file}.sig`] = args.slice(1);
  all = check(file, fs.readFileSync(sigFile, 'utf8'));
}
process.exit(all ? 0 : 1);

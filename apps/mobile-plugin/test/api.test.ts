// MO-3 and MO-4: the plugin's three sides agree. The TypeScript API (src/definitions.ts) is what the Svelte interface sees; the
// Kotlin and Swift plugins answer it; the crate's UniFFI layer (crates/escouade-mobile/src/ffi.rs) says the states and
// the errors. A method, a field or an error code added on one side alone fails here, before a phone sees it.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createPrivateKey, createPublicKey, diffieHellman } from 'node:crypto';
import { describe, expect, it } from 'vitest';
import { derive } from '../src/derive';

const read = (path: string) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8');
const ts = read('../src/definitions.ts');
const kotlin = read('../android/src/main/java/dev/escouade/mobile/plugin/EscouadeMobilePlugin.kt');
const swift = read('../ios/Sources/EscouadeMobilePlugin/EscouadeMobilePlugin.swift');
const rust = read('../../../crates/escouade-mobile/src/ffi.rs');

const all = (text: string, re: RegExp) => [...text.matchAll(re)].map((m) => m[1]);
const sorted = (xs: string[]) => [...new Set(xs)].sort();

/** The body of `interface name … { … }` (its own lines, not those of what it extends). */
function body(name: string): string {
  const m = ts.match(new RegExp(`export interface ${name}\\b[^{]*\\{([\\s\\S]*?)\\n\\}`));
  if (!m) throw new Error(`no interface ${name}`);
  return m[1];
}
const fields = (name: string) => all(body(name), /^ {2}(\w+)\??:/gm);
const union = (name: string) => {
  const m = ts.match(new RegExp(`export type ${name} =([^;]+);`));
  if (!m) throw new Error(`no type ${name}`);
  return all(m[1], /'(\w+)'/g);
};
/** The variants of a Rust enum of ffi.rs. */
const variants = (name: string) => {
  const m = rust.match(new RegExp(`pub enum ${name} \\{([\\s\\S]*?)\\n\\}`));
  if (!m) throw new Error(`no enum ${name}`);
  return all(m[1], /^ {4}(\w+)[,(]/gm);
};

describe('the Capacitor plugin', () => {
  const methods = sorted(all(body('EscouadeMobilePlugin'), /^ {2}(\w+)\(/gm));

  it('has the same methods in TypeScript, Kotlin and Swift', () => {
    expect(methods).toEqual([
      'dictate',
      'finishPairing',
      'forgetSpace',
      'lockState',
      'openFile',
      'openNotification',
      'pairingKey',
      'readPairing',
      'startTunnel',
      'stopDictation',
      'stopTunnel',
      'tunnelStatus',
      'unlock',
      'version',
    ]);
    expect(sorted(all(kotlin, /@PluginMethod\s+fun (\w+)\(/g))).toEqual(methods);
    expect(sorted(all(swift, /CAPPluginMethod\(name: "(\w+)"/g))).toEqual(methods);
    expect(sorted(all(swift, /@objc func (\w+)\(_ call: CAPPluginCall\)/g))).toEqual(methods);
    expect(kotlin).toContain('@CapacitorPlugin(name = "EscouadeMobile")');
    expect(swift).toContain('public let jsName = "EscouadeMobile"');
    expect(read('../src/index.ts')).toContain("registerPlugin<EscouadeMobilePlugin>('EscouadeMobile', {");
  });

  it('reads only the options the API names', () => {
    const options = new Set(
      [
        'SpaceOptions',
        'UnlockOptions',
        'ReadPairingOptions',
        'FinishPairingOptions',
        'OpenNotificationOptions',
        'DictateOptions',
        'OpenFileOptions',
      ].flatMap(fields),
    );
    const kotlinReads = all(kotlin, /call\.get(?:String|Int)\("(\w+)"\)/g);
    const swiftReads = all(swift, /call\.get(?:String|Int)\("(\w+)"\)/g);
    expect(kotlinReads.length).toBeGreaterThan(0);
    for (const key of [...kotlinReads, ...swiftReads]) expect(options).toContain(key);
    // Every option is read on both sides.
    for (const key of options) {
      expect(kotlinReads).toContain(key);
      expect(swiftReads).toContain(key);
    }
  });

  it('answers only the fields the API names', () => {
    // `token` is the browser's alone (no proxy writes it there): never a native side's answer.
    const answers = new Set(
      [...['TunnelStatus', 'OpenedNotification', 'LockState', 'PairingFound', 'PairingKey', 'Dictated'].flatMap(fields), 'version'].filter(
        (f) => f !== 'token',
      ),
    );
    const kotlinPuts = all(kotlin, /\.put\("(\w+)"/g);
    // A dictionary's literal keys, and the optional ones set after (`o["target"] = t`).
    const swiftKeys = [...all(swift, /"(\w+)": /g), ...all(swift, /o\["(\w+)"\] = /g)];
    for (const key of [...kotlinPuts, ...swiftKeys]) expect(answers).toContain(key);
    for (const key of answers) {
      expect(kotlinPuts).toContain(key);
      expect(swiftKeys).toContain(key);
    }
  });

  it('has the states and the error codes of the crate', () => {
    expect(union('TunnelState')).toEqual(variants('MobileTunnelState').map((v) => v.toLowerCase()));
    const codes = union('EscouadeMobileErrorCode');
    expect(codes).toEqual([...variants('MobileError'), 'NoTunnel', 'NoSpace', 'NoLock', 'Cancelled', 'BadPairing', 'NoDictation', 'NoApp']);
    // The plugin's own codes are among them.
    for (const code of [...all(kotlin, /call\.reject\("[^"]*", "(\w+)"\)/g), ...all(swift, /call\.reject\("[^"]*", "(\w+)"\)/g)])
      expect(codes).toContain(code);
  });

  it('never takes a key, a token or a config from the JavaScript (SR1-15)', () => {
    const passed = ['SpaceOptions', 'UnlockOptions', 'ReadPairingOptions', 'FinishPairingOptions', 'OpenNotificationOptions'].flatMap(
      fields,
    );
    for (const secret of ['privateKey', 'token', 'pairingKey', 'peerPublicKey', 'deviceToken']) expect(passed).not.toContain(secret);
    // What startTunnel takes: a space's id alone.
    expect(body('EscouadeMobilePlugin')).toContain('startTunnel(options: SpaceOptions)');
    for (const native of [kotlin, swift]) {
      expect(native).toMatch(/deviceToken/);
      expect(native).not.toMatch(/call\.getString\("(privateKey|token|pairingKey|deviceToken)"\)/);
    }
    // The Keystore (Android) and the Keychain, this device alone, after the first unlock (iOS).
    expect(kotlin).toContain('"AndroidKeyStore"');
    expect(swift).toContain('kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly');
  });
});

describe('the pairing in a browser', () => {
  it('derives the token and the key as the Rust does (the vector of crates/escouade-mobile)', async () => {
    const pkcs8 = (k: number) =>
      createPrivateKey({
        key: Buffer.concat([Buffer.from('302e020100300506032b656e04220420', 'hex'), Buffer.alloc(32, k)]),
        format: 'der',
        type: 'pkcs8',
      });
    const shared = diffieHellman({ privateKey: pkcs8(1), publicKey: createPublicKey(pkcs8(2)) });
    const { token, key } = await derive(shared.buffer.slice(shared.byteOffset, shared.byteOffset + shared.byteLength) as ArrayBuffer);
    expect(token).toBe('mX1HZhfIWYIZzkH1vuI6Ig8Ga_W-jTfxGzbHZeqlB5o');
    expect(key).toBe('qZxFr/xX1SpwFdIjtQhNHT2R4TiuZur629pvqH4Aoq8=');
  });
});

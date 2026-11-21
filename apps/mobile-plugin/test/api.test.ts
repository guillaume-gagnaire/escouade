// MO-3: the plugin's three sides agree. The TypeScript API (src/definitions.ts) is what the Svelte interface sees; the
// Kotlin and Swift plugins answer it; the crate's UniFFI layer (crates/escouade-mobile/src/ffi.rs) says the states and
// the errors. A method, a field or an error code added on one side alone fails here, before a phone sees it.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

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
    expect(methods).toEqual(['openNotification', 'startTunnel', 'stopTunnel', 'tunnelStatus', 'version']);
    expect(sorted(all(kotlin, /@PluginMethod\s+fun (\w+)\(/g))).toEqual(methods);
    expect(sorted(all(swift, /CAPPluginMethod\(name: "(\w+)"/g))).toEqual(methods);
    expect(sorted(all(swift, /@objc func (\w+)\(_ call: CAPPluginCall\)/g))).toEqual(methods);
    expect(kotlin).toContain('@CapacitorPlugin(name = "EscouadeMobile")');
    expect(swift).toContain('public let jsName = "EscouadeMobile"');
    expect(read('../src/index.ts')).toContain("registerPlugin<EscouadeMobilePlugin>('EscouadeMobile')");
  });

  it('reads only the options the API names', () => {
    const options = new Set([...fields('TunnelConfig'), ...fields('SpaceOptions'), ...fields('OpenNotificationOptions')]);
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
    const answers = new Set([...fields('TunnelStatus'), ...fields('OpenedNotification'), 'version']);
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
    expect(codes).toEqual([...variants('MobileError'), 'NoTunnel']);
    // The plugin's own codes are among them.
    for (const code of [...all(kotlin, /call\.reject\("[^"]*", "(\w+)"\)/g), ...all(swift, /call\.reject\("[^"]*", "(\w+)"\)/g)])
      expect(codes).toContain(code);
  });
});

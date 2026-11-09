// @vitest-environment node
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { compare, pick, problems } from './beta-channel.mjs';

const SCRIPT = path.join(__dirname, 'beta-channel.mjs');

/** A latest.json of `version`, as the release workflow writes it. */
const manifest = (version: string) => ({
  version,
  notes: `- Notes de ${version}`,
  platforms: { 'windows-x86_64': { signature: 's', url: `https://x/releases/download/v${version}/setup.exe` } },
});

describe('the order of the versions', () => {
  it('follows semver: a beta before its version, beta.10 after beta.9', () => {
    expect(compare('2.2.0-beta.1', '2.2.0')).toBeLessThan(0);
    expect(compare('2.2.0', '2.2.0-beta.3')).toBeGreaterThan(0);
    expect(compare('2.2.0-beta.10', '2.2.0-beta.9')).toBeGreaterThan(0);
    expect(compare('2.2.0-beta.1', '2.1.5')).toBeGreaterThan(0);
    expect(compare('v2.1.4', '2.1.4')).toBe(0);
    expect(compare('2.2.0-beta', '2.2.0-beta.1')).toBeLessThan(0);
    expect(compare('2.2.0-alpha.2', '2.2.0-beta.1')).toBeLessThan(0);
    expect(() => compare('2.2', '2.2.0')).toThrow('2.2');
  });
});

describe('the beta.json put in place', () => {
  it('a beta, then a stable version: each is the newest in its turn', () => {
    // The first beta, while beta.json holds the stable 2.1.4.
    const beta = pick('v2.2.0-beta.1', manifest('2.2.0-beta.1'), manifest('2.1.4'));
    expect(beta.version).toBe('2.2.0-beta.1');
    expect(beta.notes).toBe('- Notes de 2.2.0-beta.1');
    // Its stable version: newer than the beta.
    expect(pick('v2.2.0', manifest('2.2.0'), beta).version).toBe('2.2.0');
  });

  it('a stable version that goes out while a newer beta exists keeps the beta', () => {
    const current = manifest('2.2.0-beta.2');
    expect(pick('v2.1.5', manifest('2.1.5'), current)).toEqual(current);
  });

  it('the first stable version without a beta.json before it takes its own', () => {
    expect(pick('v2.1.4', manifest('2.1.4'), null).version).toBe('2.1.4');
  });

  it('the same version again takes the fresh one, and a tag that is not its latest.json’s is refused', () => {
    const fresh = { ...manifest('2.1.4'), notes: '- Nouvelles notes' };
    expect(pick('v2.1.4', fresh, manifest('2.1.4')).notes).toBe('- Nouvelles notes');
    expect(() => pick('v2.1.4', manifest('2.1.3'), null)).toThrow('2.1.3');
  });
});

describe('the check once published', () => {
  it('wants the tag’s version or a newer one, with its platforms', () => {
    expect(problems('v2.1.5', manifest('2.1.5'))).toEqual([]);
    expect(problems('v2.1.5', manifest('2.2.0-beta.2'))).toEqual([]);
    expect(problems('v2.2.0-beta.1', manifest('2.1.4'))[0]).toContain('older');
    expect(problems('v2.1.5', null)[0]).toContain('no version');
    expect(problems('v2.1.5', { version: '2.1.5', platforms: {} })[0]).toContain('no platform');
  });
});

describe('the command line', () => {
  let dir = '';
  afterEach(() => fs.rmSync(dir, { recursive: true, force: true }));

  const run = (...args: string[]) => {
    try {
      return { code: 0, out: execFileSync(process.execPath, [SCRIPT, ...args], { encoding: 'utf8' }) };
    } catch (e) {
      const err = e as { status: number; stdout: string };
      return { code: err.status, out: err.stdout };
    }
  };

  it('picks from files, a beta.json now that is absent or empty counting for none, and checks the result', () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'beta-channel-'));
    const file = (name: string, body: unknown) => {
      const p = path.join(dir, name);
      fs.writeFileSync(p, typeof body === 'string' ? body : JSON.stringify(body));
      return p;
    };
    const latest = file('latest.json', manifest('2.1.4'));
    expect(JSON.parse(run('pick', 'v2.1.4', latest, path.join(dir, 'none.json')).out).version).toBe('2.1.4');
    expect(JSON.parse(run('pick', 'v2.1.4', latest, file('empty.json', '')).out).version).toBe('2.1.4');
    const newer = file('beta-now.json', manifest('2.2.0-beta.1'));
    expect(JSON.parse(run('pick', 'v2.1.4', latest, newer).out).version).toBe('2.2.0-beta.1');

    expect(run('check', 'v2.1.4', newer)).toEqual({ code: 0, out: 'beta.json says version 2.2.0-beta.1 once v2.1.4 is out\n' });
    const old = run('check', 'v2.2.0-beta.2', newer);
    expect(old.code).toBe(1);
    expect(old.out).toContain('::error::beta.json says version 2.2.0-beta.1, older than 2.2.0-beta.2');
    expect(run('pick').out).toContain('::error::usage');
    // Seven starts of node: more than Vitest's 5 s on a Windows machine that builds meanwhile.
  }, 60_000);
});

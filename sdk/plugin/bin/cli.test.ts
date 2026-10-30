// @vitest-environment node
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { gunzipSync } from 'node:zlib';
import { describe, expect, it } from 'vitest';
import { nativeBin } from './native.mjs';

// The escouade-plugin of the SDK (ESC-181, PLG-9): validate and pack in Node, test and dev handed to the app's own.

const CLI = path.join(__dirname, 'escouade-plugin.mjs');
// PLG-10's examples (ESC-182).
const EXAMPLES = path.join(__dirname, '..', '..', '..', 'plugin-examples');

function cli(args: string[], env: Record<string, string> = {}) {
  const r = spawnSync(process.execPath, [CLI, ...args, '--lang', 'en'], { encoding: 'utf8', env: { ...process.env, ...env } });
  return { code: r.status, out: r.stdout, err: r.stderr };
}

function tmp(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'sdk-cli-'));
}

/** The entries of a .tar.gz: name, mode, bytes. */
function entries(file: string) {
  const tar = gunzipSync(fs.readFileSync(file));
  const out: Array<{ name: string; mode: number; text: string }> = [];
  for (let at = 0; at + 512 <= tar.length;) {
    const h = tar.subarray(at, at + 512);
    if (h.every((b) => b === 0)) break;
    const str = (from: number, len: number) =>
      h
        .subarray(from, from + len)
        .toString('utf8')
        .replace(/\0.*$/s, '');
    const size = parseInt(str(124, 12), 8);
    const prefix = str(345, 155);
    out.push({
      name: (prefix ? prefix + '/' : '') + str(0, 100),
      mode: parseInt(str(100, 8), 8),
      text: tar.subarray(at + 512, at + 512 + size).toString('utf8'),
    });
    at += 512 + Math.ceil(size / 512) * 512;
  }
  return out;
}

describe('escouade-plugin of the SDK', () => {
  it('validates a manifest, and says each field that is wrong', () => {
    expect(cli(['validate', path.join(EXAMPLES, 'slack')])).toMatchObject({ code: 0, out: expect.stringContaining('the manifest holds') });
    const dir = tmp();
    fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify({ id: 'Nope', version: 'x', run: { command: 'gone' } }));
    const bad = cli(['validate', dir]);
    expect(bad.code).toBe(1);
    expect(bad.err).toMatch(/id/);
    expect(bad.err).toMatch(/version/);
  });

  it('refuses a run program missing from the folder', () => {
    const dir = tmp();
    fs.copyFileSync(path.join(EXAMPLES, 'slack', 'escouade-plugin.json'), path.join(dir, 'escouade-plugin.json'));
    const r = cli(['validate', dir]);
    expect(r.code).toBe(1);
    expect(r.err).toContain('slack is not in the folder');
  });

  it('packs the same bytes twice, programs executable, without node_modules, .git or packs', () => {
    const dir = tmp();
    // What the Slack example runs, without its sources (and whatever npm left in it here).
    for (const f of ['escouade-plugin.json', 'channels.json', 'slack', 'slack.cmd', 'dist/plugin.mjs']) {
      fs.cpSync(path.join(EXAMPLES, 'slack', f), path.join(dir, f));
    }
    fs.mkdirSync(path.join(dir, 'node_modules', 'x'), { recursive: true });
    fs.writeFileSync(path.join(dir, 'node_modules', 'x', 'i.js'), '');
    fs.mkdirSync(path.join(dir, '.git'));
    fs.writeFileSync(path.join(dir, '.git', 'HEAD'), '');
    fs.mkdirSync(path.join(dir, 'docs', 'a'.repeat(60), 'b'.repeat(60)), { recursive: true });
    fs.writeFileSync(path.join(dir, 'docs', 'a'.repeat(60), 'b'.repeat(60), 'long.md'), '# long');
    const first = cli(['pack', dir, '--target', 'linux-x64']);
    expect(first.code, first.err).toBe(0);
    const file = path.join(dir, 'packs', 'dev.exemple.slack-1.0.0-linux-x64.tar.gz');
    const bytes = fs.readFileSync(file);
    const sha = createHash('sha256').update(bytes).digest('hex');
    expect(first.out).toContain(`sha256 ${sha}`);
    expect(fs.readFileSync(`${file}.sha256`, 'utf8')).toBe(`${sha}  dev.exemple.slack-1.0.0-linux-x64.tar.gz\n`);
    expect(cli(['pack', dir, '--target', 'linux-x64']).code).toBe(0);
    expect(fs.readFileSync(file).equals(bytes)).toBe(true);
    const list = entries(file);
    expect(list.map((e) => e.name)).toEqual([
      'channels.json',
      'dist/plugin.mjs',
      `docs/${'a'.repeat(60)}/${'b'.repeat(60)}/long.md`,
      'escouade-plugin.json',
      'slack',
      'slack.cmd',
    ]);
    expect(list.find((e) => e.name === 'slack')?.mode).toBe(0o755);
    expect(list.find((e) => e.name === 'escouade-plugin.json')?.mode).toBe(0o644);
    expect(list.find((e) => e.name.endsWith('long.md'))?.text).toBe('# long');
  });

  it('refuses a target that is not a system and an architecture', () => {
    expect(cli(['pack', path.join(EXAMPLES, 'slack'), '--target', 'amiga', '--out', tmp()]).code).toBe(2);
  });

  it('hands test and dev to the app’s escouade-plugin, or says where to get it', () => {
    const none = cli(['test', path.join(EXAMPLES, 'slack')], { ESCOUADE_PLUGIN_BIN: '', PATH: '', Path: '', LOCALAPPDATA: tmp() });
    expect(none.code).toBe(2);
    expect(none.err).toContain('ESCOUADE_PLUGIN_BIN');
    // A stand-in: node itself, which runs the script named test in its folder, and says what it was given.
    const dir = tmp();
    fs.writeFileSync(path.join(dir, 'test'), 'console.log(JSON.stringify(process.argv.slice(2)))');
    const folder = path.join(EXAMPLES, 'slack');
    const r = spawnSync(process.execPath, [CLI, 'test', folder, '--json'], {
      cwd: dir,
      encoding: 'utf8',
      env: { ...process.env, ESCOUADE_PLUGIN_BIN: process.execPath, LANG: 'fr_FR.UTF-8' },
    });
    expect(r.status, r.stderr).toBe(0);
    expect(JSON.parse(r.stdout)).toEqual([folder, '--json', '--lang', 'fr']);
    expect(nativeBin({ ESCOUADE_PLUGIN_BIN: 'x' })).toBe('x');
  });

  it('finds the app’s command on the PATH, never an npm shim', () => {
    const dir = tmp();
    const shims = path.join(dir, 'node_modules', '.bin');
    fs.mkdirSync(shims, { recursive: true });
    const exe = process.platform === 'win32' ? 'escouade-plugin.exe' : 'escouade-plugin';
    fs.writeFileSync(path.join(shims, exe), 'MZ');
    const script = path.join(dir, 'scripts');
    fs.mkdirSync(script);
    fs.writeFileSync(path.join(script, exe), '#!/usr/bin/env node\n');
    const app = path.join(dir, 'app');
    fs.mkdirSync(app);
    fs.writeFileSync(path.join(app, exe), 'MZ');
    expect(nativeBin({ PATH: [shims, script, app].join(path.delimiter) })).toBe(path.join(app, exe));
    expect(nativeBin({ PATH: [shims, script].join(path.delimiter) }, 'freebsd')).toBeNull();
  });
});

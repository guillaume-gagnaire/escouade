// @vitest-environment node
// The browser's build of the interface (BC-1, ESC-704, vite.web.config.ts): each module `@tauri-apps/*` is a stub that
// refuses with an error of its own name; a module or a name without its stub fails the build; and a bundle that holds a
// mark of Tauri's IPC (`__TAURI_INTERNALS__`, `ipc.localhost`) fails it too. `npm run build:web` builds the real one
// with the same plugins (and crates/escouade-web's build.rs checks the files again before it embeds them).

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { build, type Rollup } from 'vite';
import { describe, expect, it } from 'vitest';
import webConfig, { noTauriInBundle, tauriMarks, tauriStubs, web } from '../../vite.web.config';
import { FORBIDDEN_IN_BUNDLE, STUBS, TauriUnavailableError, refusing, refusingAsync, refusingClass, stubSource } from './tauri-refused';

const ROOT = path.resolve(import.meta.dirname, '..', '..');

/** The sources of the interface that are not tests. */
function sources(dir: string): string[] {
  const out: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name !== 'test') out.push(...sources(full));
    } else if (/\.(ts|svelte)$/.test(entry.name) && !/\.test\.ts$/.test(entry.name)) out.push(full);
  }
  return out;
}

/** A small entry built with the web build's plugins, in a folder of its own; its chunks' code, or the build's error. */
async function buildEntry(code: string): Promise<string> {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-web-build-'));
  try {
    fs.writeFileSync(path.join(dir, 'entry.js'), code);
    const out = (await build({
      configFile: false,
      logLevel: 'silent',
      root: dir,
      plugins: [tauriStubs(), noTauriInBundle()],
      define: web.define,
      build: { write: false, minify: true, lib: { entry: path.join(dir, 'entry.js'), formats: ['es'], fileName: 'entry' } },
    })) as Rollup.RollupOutput | Rollup.RollupOutput[];
    const outputs = Array.isArray(out) ? out : [out];
    return outputs
      .flatMap((o) => o.output)
      .map((c) => (c.type === 'chunk' ? c.code : String(c.source)))
      .join('\n');
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

describe('the stubs of Tauri in the browser', () => {
  it('refuse with an error of their own name, never a value', async () => {
    const invoke = refusingAsync('@tauri-apps/api/core', 'invoke');
    await expect(invoke('fs_read')).rejects.toBeInstanceOf(TauriUnavailableError);
    await expect(invoke('fs_read')).rejects.toMatchObject({
      name: 'TauriUnavailableError',
      module: '@tauri-apps/api/core',
      member: 'invoke',
    });
    expect(() => refusing('@tauri-apps/api/window', 'getCurrentWindow')()).toThrow(TauriUnavailableError);
    const Channel = refusingClass('@tauri-apps/api/core', 'Channel');
    expect(() => new Channel()).toThrow(/Channel/);
  });

  it('have a stub for each module and each name the interface imports', () => {
    const missing: string[] = [];
    const pattern = /import\s*(?:\{([^}]*)\}\s*from\s*)?['"](@tauri-apps\/[^'"]+)['"]|import\(\s*['"](@tauri-apps\/[^'"]+)['"]\s*\)/g;
    for (const file of sources(path.join(ROOT, 'src'))) {
      const text = fs.readFileSync(file, 'utf8');
      for (const m of text.matchAll(pattern)) {
        const module = m[2] ?? m[3]!;
        const stub = STUBS[module];
        if (!stub) {
          missing.push(`${path.relative(ROOT, file)}: ${module}`);
          continue;
        }
        for (const raw of (m[1] ?? '').split(',')) {
          const name = raw
            .trim()
            .replace(/^type\s+/, '')
            .split(/\s+as\s+/)[0]!
            .trim();
          if (name && !raw.trim().startsWith('type ') && !(name in stub)) missing.push(`${path.relative(ROOT, file)}: ${module} › ${name}`);
        }
      }
    }
    expect(missing).toEqual([]);
  });

  it('are given by the plugin for @tauri-apps/* alone, and a module without a stub fails', () => {
    const plugin = tauriStubs();
    const resolve = plugin.resolveId as (this: unknown, id: string) => string | null;
    const load = plugin.load as (this: unknown, id: string) => string | null;
    const ctx = {
      error(message: string): never {
        throw new Error(message);
      },
    };
    expect(resolve.call(ctx, 'svelte')).toBeNull();
    const id = resolve.call(ctx, '@tauri-apps/plugin-dialog')!;
    expect(load.call(ctx, id)).toContain('export const open = refusingAsync("@tauri-apps/plugin-dialog", "open");');
    expect(() => resolve.call(ctx, '@tauri-apps/plugin-shell')).toThrow(/aucun bouchon/);
    expect(stubSource('@tauri-apps/plugin-shell', '/x.ts')).toBeNull();
  });
});

describe('the bundle of the browser', () => {
  it('finds the marks of Tauri’s IPC in any file', () => {
    expect(FORBIDDEN_IN_BUNDLE).toEqual(['__TAURI_INTERNALS__', 'ipc.localhost']);
    expect(
      tauriMarks({ 'a.js': 'ok', 'b.js': 'x.__TAURI_INTERNALS__', 'c.wasm': new TextEncoder().encode('http://ipc.localhost/') }),
    ).toEqual(['b.js: __TAURI_INTERNALS__', 'c.wasm: ipc.localhost']);
  });

  it('holds neither __TAURI_INTERNALS__ nor ipc.localhost: the stubs refuse and the test of Tauri falls away', async () => {
    const code = await buildEntry(
      [
        "import { invoke, Channel } from '@tauri-apps/api/core';",
        "import { getCurrentWindow } from '@tauri-apps/api/window';",
        "export const inTauri = !import.meta.env.ESCOUADE_WEB && '__TAURI_INTERNALS__' in window;",
        'export { invoke, Channel, getCurrentWindow };',
      ].join('\n'),
    );
    for (const mark of FORBIDDEN_IN_BUNDLE) expect(code).not.toContain(mark);
    expect(code).toContain('TauriUnavailableError');
  }, 60_000);

  it('fails to build when a file keeps a mark of Tauri', async () => {
    await expect(buildEntry("export const url = 'http://ipc.localhost/plugin';")).rejects.toThrow(/garde Tauri.*ipc\.localhost/s);
  }, 60_000);

  it('keeps public/ as the window has it: index.html’s scripts of public/ (What’s up ?) resolve and are copied as they are', async () => {
    // Every address of index.html outside src/ is a file of public/.
    const html = fs.readFileSync(path.join(ROOT, 'index.html'), 'utf8');
    const addresses = [...html.matchAll(/(?:src|href)="(\/[^"]+)"/g)].map((m) => m[1]!).filter((a) => !a.startsWith('/src/'));
    expect(addresses).toContain('/whats-up/whats-up.js');
    for (const a of addresses.filter((a) => !a.endsWith('/'))) expect(fs.existsSync(path.join(ROOT, 'public', a)), a).toBe(true);
    // A page that loads a script of its public/, built with the browser's configuration: resolved, copied, never bundled.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'escouade-web-public-'));
    try {
      fs.mkdirSync(path.join(dir, 'public', 'whats-up'), { recursive: true });
      fs.writeFileSync(
        path.join(dir, 'public', 'whats-up', 'whats-up.js'),
        'customElements.define("whats-up", class extends HTMLElement {});',
      );
      fs.writeFileSync(
        path.join(dir, 'index.html'),
        '<!doctype html><html><head><style id="escouade-style-nonce"></style></head><body><script type="module" src="/main.js"></script><script type="module" src="/whats-up/whats-up.js"></script></body></html>',
      );
      fs.writeFileSync(path.join(dir, 'main.js'), "import { invoke } from '@tauri-apps/api/core'; export { invoke };");
      const out = path.join(dir, 'dist-web');
      await build({
        configFile: false,
        logLevel: 'silent',
        root: dir,
        plugins: [tauriStubs(), noTauriInBundle()],
        define: web.define,
        publicDir: web.publicDir,
        build: { ...web.build, outDir: out },
      });
      expect(fs.readFileSync(path.join(out, 'index.html'), 'utf8')).toContain('src="/whats-up/whats-up.js"');
      expect(fs.existsSync(path.join(out, 'whats-up', 'whats-up.js'))).toBe(true);
    } finally {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  }, 60_000);

  it('is the desktop configuration with the stubs, into dist-web, with the same public/', () => {
    expect(webConfig.build?.outDir).toBe('dist-web');
    expect(webConfig.publicDir ?? 'public').toBe('public');
    expect(web.publicDir).toBeUndefined();
    expect(webConfig.define).toMatchObject({ 'import.meta.env.ESCOUADE_WEB': 'true' });
    const names = (webConfig.plugins ?? []).flat().map((p: unknown) => (p && typeof p === 'object' && 'name' in p ? p.name : ''));
    expect(names).toEqual(expect.arrayContaining(['escouade-tauri-stubs', 'escouade-no-tauri-in-bundle', 'vite-plugin-svelte']));
    const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8')) as { scripts: Record<string, string> };
    expect(pkg.scripts['build:web']).toBe('vite build -c vite.web.config.ts');
    expect(fs.readFileSync(path.join(ROOT, '.gitignore'), 'utf8')).toMatch(/^dist-web$/m);
  });
});

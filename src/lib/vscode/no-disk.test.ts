// The VS Code view reaches the disk through the core's commands alone (ESC-447): no way of its own to a file, nor a
// door the page could open (the file system plugin of Tauri, the asset protocol widened, `eval` for extensions).

import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = path.resolve(import.meta.dirname, '..', '..', '..');
const read = (rel: string) => fs.readFileSync(path.join(ROOT, rel), 'utf8');
const VIEW = [
  ...fs
    .readdirSync(path.join(ROOT, 'src/lib/vscode'))
    .filter((f) => f.endsWith('.ts') && !f.endsWith('.test.ts'))
    .map((f) => `src/lib/vscode/${f}`),
  'src/components/editor/VscodeView.svelte',
];

describe('the VS Code view and the disk', () => {
  it('has no way to a file but the core’s commands', () => {
    for (const file of VIEW) {
      const code = read(file);
      for (const banned of [
        '@tauri-apps/plugin-fs',
        'convertFileSrc',
        'asset://',
        'file://',
        'fetch(',
        'XMLHttpRequest',
        'createHTMLFileSystemProvider',
        'createIndexedDBProviders',
        "registerCustomProvider('file'",
        'showDirectoryPicker',
      ])
        expect(code, `${file}: ${banned}`).not.toContain(banned);
    }
  });

  it('gives the provider nothing but the commands of the editor', () => {
    const provider = read('src/lib/vscode/fs-provider.ts');
    const imports = [...provider.matchAll(/^import (type )?.* from '([^']+)';$/gm)].map((m) => ({ type: !!m[1], from: m[2] }));
    // Types alone, and the pure paths: the commands are handed to it (`CoreFiles`), the core's `api` by the workbench.
    expect(imports.filter((i) => !i.type).map((i) => i.from)).toEqual(['./tree']);
    expect(provider).not.toContain('invoke(');
    const workbench = read('src/lib/vscode/workbench.ts');
    expect(workbench).toMatch(/new EscouadeFileSystem<vscode\.Uri>\(\s*api,/);
    // The files of the sources are the overlay's first provider; nothing else serves `file:`.
    expect(workbench.match(/registerFileSystemOverlay\(/g)).toHaveLength(1);
    expect(workbench).not.toContain('registerCustomProvider');
  });

  it('opens no other door to the disk in the window', () => {
    const conf = JSON.parse(read('src-tauri/tauri.conf.json')) as { app: { security: { csp: string; assetProtocol?: unknown } } };
    const csp = conf.app.security.csp;
    // No extension code evaluated in the page (that is A5/A6's, from an origin apart).
    expect(csp).not.toContain("'unsafe-eval'");
    expect(conf.app.security.assetProtocol).toBeUndefined();
    const caps = read('src-tauri/capabilities/default.json');
    expect(caps).not.toMatch(/"fs:/);
    expect(read('package.json')).not.toContain('@tauri-apps/plugin-fs');
  });

  it('pins monaco-vscode-api and its packages to one version', () => {
    const deps = (JSON.parse(read('package.json')) as { dependencies: Record<string, string> }).dependencies;
    const vscode = Object.entries(deps).filter(([name, v]) => name.startsWith('@codingame/monaco-vscode-') || v.includes('@codingame/'));
    expect(vscode.length).toBeGreaterThan(20);
    for (const [name, v] of vscode) expect(v, name).toMatch(/(^|@)37\.6\.1$/);
  });
});

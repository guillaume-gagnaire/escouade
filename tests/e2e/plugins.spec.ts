import fs from 'node:fs';
import path from 'node:path';
import { expect, test } from './fixture';

// ESC-173: a declarative plugin installed from a folder by the app's commands (Réglages › Plugins comes with PLG-4) is
// copied into the app's data, pinned by its files, and found again once the app starts anew; a folder whose manifest
// does not hold loads nothing, and says why, field by field.

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<{ ok: T } | { err: unknown }>;

function plugin(dir: string, manifest: unknown) {
  fs.mkdirSync(path.join(dir, 'themes'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'escouade-plugin.json'), JSON.stringify(manifest, null, 2));
  fs.writeFileSync(path.join(dir, 'themes', 'contrast.json'), '{}');
}

interface View {
  id: string;
  name: string;
  version: string;
  declarative: boolean;
  fingerprint: string;
  folder: string;
  origin: { kind: string; path?: string };
  state: { kind: string; reason?: string };
  ignored: string[];
}

test('a declarative plugin is installed from a folder, and an invalid one says why, field by field', async ({ app }) => {
  const invokeOn =
    (page: typeof app.page): Invoke =>
    (cmd, args = {}) =>
      page.evaluate(
        async ([cmd, args]) => {
          const t = (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a: unknown): Promise<unknown> } }).__TAURI_INTERNALS__;
          try {
            return { ok: await t.invoke(cmd as string, args) };
          } catch (err) {
            return { err };
          }
        },
        [cmd, args] as const,
      ) as never;
  let invoke = invokeOn(app.page);

  const src = path.join(app.root, 'contrast-plugin');
  plugin(src, {
    $schema: 'https://escouade.dev/schemas/plugin-1.json',
    id: 'dev.exemple.contraste',
    name: { fr: 'Contraste élevé', en: 'High contrast' },
    version: '1.0.0',
    escouade: { api: '1', min: '2.1.0' },
    contributes: { themes: [{ id: 'contrast' }] },
    later: { field: 'of a later Escouade' },
  });

  expect(await invoke<View[]>('plugins_list')).toEqual({ ok: [] });
  const installed = await invoke<View>('plugin_install_folder', { path: src });
  expect('ok' in installed, JSON.stringify(installed)).toBe(true);
  const view = (installed as { ok: View }).ok;
  expect(view).toMatchObject({
    id: 'dev.exemple.contraste',
    name: 'Contraste élevé',
    version: '1.0.0',
    declarative: true,
    origin: { kind: 'folder', path: src },
    state: { kind: 'ready' },
    ignored: ['later'],
  });
  expect(view.fingerprint).toMatch(/^sha256:[0-9a-f]{64}$/);
  // Copied into the plugins' folder of the app's data.
  const copy = path.join(app.data, 'plugins', 'dev.exemple.contraste');
  expect(path.resolve(view.folder)).toBe(path.resolve(copy));
  expect(fs.existsSync(path.join(copy, 'themes', 'contrast.json'))).toBe(true);

  // A folder whose manifest does not hold: nothing loaded, each field said.
  const bad = path.join(app.root, 'bad-plugin');
  plugin(bad, { id: 'Contraste', name: '', version: '1.0', escouade: { api: '1' } });
  const refused = await invoke('plugin_install_folder', { path: bad });
  expect('err' in refused).toBe(true);
  const err = (refused as { err: { message: string; problems: { path: string; code: string }[] } }).err;
  expect(err.problems.map((p) => `${p.path} ${p.code}`)).toEqual(['id id', 'name empty', 'version version', 'escouade.min required']);
  expect(err.message).toContain("Le manifeste (escouade-plugin.json) n'est pas valide ; rien n'a été chargé :");
  expect(err.message).toContain('- `escouade.min` manque.');
  expect(fs.readdirSync(path.join(app.data, 'plugins'))).toEqual(['dev.exemple.contraste']);

  // Started anew, the app finds it as it was installed.
  invoke = invokeOn(await app.restart());
  const again = await invoke<View[]>('plugins_list');
  expect(again).toMatchObject({ ok: [{ id: 'dev.exemple.contraste', fingerprint: view.fingerprint, state: { kind: 'ready' } }] });

  // PLG-13: its icon by its id alone, as an address the window shows; none without one; an unknown id refused.
  expect(await invoke('plugin_icon', { id: 'dev.exemple.contraste' })).toEqual({ ok: null });
  expect('err' in (await invoke('plugin_icon', { id: '../contrast-plugin' }))).toBe(true);
  const logo = path.join(app.root, 'logo-plugin');
  plugin(logo, { id: 'dev.exemple.logo', name: 'Logo', version: '1.0.0', escouade: { api: '1', min: '2.1.0' }, icon: 'icon.png' });
  const png = Buffer.alloc(600);
  Buffer.from('\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR', 'latin1').copy(png);
  png.writeUInt32BE(256, 16);
  png.writeUInt32BE(256, 20);
  fs.writeFileSync(path.join(logo, 'icon.png'), png);
  expect('ok' in (await invoke('plugin_install_folder', { path: logo }))).toBe(true);
  const icon = (await invoke<string>('plugin_icon', { id: 'dev.exemple.logo' })) as { ok: string };
  expect(icon.ok).toBe(`data:image/png;base64,${png.toString('base64')}`);
  expect(await invoke('plugin_remove', { id: 'dev.exemple.logo' })).toEqual({ ok: true });

  // Uninstalled: its copy goes, the folder it came from stays.
  expect(await invoke('plugin_remove', { id: 'dev.exemple.contraste' })).toEqual({ ok: true });
  expect(fs.existsSync(copy)).toBe(false);
  expect(fs.existsSync(path.join(src, 'escouade-plugin.json'))).toBe(true);
  expect(await invoke<View[]>('plugins_list')).toEqual({ ok: [] });
});

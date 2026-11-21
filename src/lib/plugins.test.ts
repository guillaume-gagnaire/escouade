import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { askOf, categoriesOf, contributionsOf, piecesOf, sourceUrl } from './plugins';
import { addsOf } from './plugins-settings';

// Where a store plugin comes from (ESC-313): its repository, or its folder at its tag when it is in one.
describe('the source of a store plugin', () => {
  it('links the folder at the tag, and the repository for a plugin at its root or an old version.json', () => {
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'slack@1.0.0', path: 'plugins/slack' })).toBe(
      'https://github.com/o/r/tree/slack@1.0.0/plugins/slack',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r/', tag: 'plugins/x/v1.0.0', path: 'plugins/x' })).toBe(
      'https://github.com/o/r/tree/plugins/x/v1.0.0/plugins/x',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0', path: '' })).toBe('https://github.com/o/r');
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0' })).toBe('https://github.com/o/r');
  });
});

describe('the network a plugin asks for', () => {
  const inWasm = { run: { js: 'dist/plugin.mjs' } } as never;

  it('names a host, what Escouade applies under it', () => {
    const ask = askOf('network:api.exemple.dev', inWasm, 'fr');
    expect(ask.detail).toBe('api.exemple.dev');
    expect(ask.note).toBeTruthy();
  });

  it('says the instances the user gives, and the local server of its harness, without a host to name', () => {
    const account = askOf('network:{account}', inWasm, 'fr');
    expect(account.text).toBe('Se connecte aux instances que tu lui donnes');
    expect(account.detail).toBe('');
    const shared = askOf('network:local:shared', inWasm, 'fr');
    expect(shared.text).toBe('Parle au serveur local du harness, lancé par Escouade');
    expect(shared.detail).toBe('');
  });
});

// The jokers of a command (ESC-528): each a pill of words in place of the word the host checks, in both languages.
describe('the jokers of a command', () => {
  const inWasm = { run: { wasm: 'run.wasm' } } as never;

  it('says each joker in words, the other words as written', () => {
    setLang('fr');
    const ask = askOf('commands:unshare -Urn openvpn --config {path:temp} --script-security 1', inWasm, 'fr');
    expect(ask.detail).toBe('unshare -Urn openvpn --config {path:temp} --script-security 1');
    expect(ask.pieces).toEqual([
      { text: 'unshare' },
      { text: '-Urn' },
      { text: 'openvpn' },
      { text: '--config' },
      { text: 'un fichier de son dossier temporaire', joker: '{path:temp}' },
      { text: '--script-security' },
      { text: '1' },
    ]);
    const all = piecesOf('tool {int} {port} {name} {pid} {path:temp} {path:plugin}')!;
    expect(all.slice(1).map((p) => p.text)).toEqual([
      'un nombre',
      'un port',
      'un nom',
      'un programme qu’il a lancé',
      'un fichier de son dossier temporaire',
      'un fichier de son dossier',
    ]);
    setLang('en');
    expect(
      piecesOf('tool {int} {port} {name} {pid} {path:temp} {path:plugin}')!
        .slice(1)
        .map((p) => p.text),
    ).toEqual(['a number', 'a port', 'a name', 'a program it started', 'a file of its temporary folder', 'a file of its folder']);
    setLang('fr');
  });

  it('leaves a command without a joker, or with a brace in a word, as written', () => {
    expect(askOf('commands:isola ls --json', inWasm, 'fr').pieces).toBeUndefined();
    expect(piecesOf('tool --pid={int}')).toBeNull();
    expect(piecesOf('tool {float}')).toBeNull();
    // Cut as the host cuts it: a non-breaking space is a space, U+FEFF is not.
    expect(piecesOf('tool {int}')?.[1]).toEqual({ text: 'un nombre', joker: '{int}' });
    expect(piecesOf('tool﻿{int}')).toBeNull();
  });
});

// The variables a plugin declares (ESC-424): each shown before it is installed, what Escouade applies under it.
describe('the variables a plugin sets', () => {
  it('names each variable, and says that no other is set', () => {
    const ask = askOf('env:MON_OUTIL_MODE', { run: { js: 'dist/plugin.mjs' } } as never, 'fr');
    expect(ask.text).toBe('Pose cette variable d’environnement sur ce qu’il lance');
    expect(ask.detail).toBe('MON_OUTIL_MODE');
    expect(ask.note).toContain('aucune autre variable');
  });
});

// What else a harness's manifest says (SEC-30): its commands, shown as written by the plugin, and the variables it takes off.
describe('what a harness of a plugin says beside what it runs', () => {
  it('shows the commands that install and update it, said to be written by the plugin', () => {
    const install = askOf('harnessInstall:npm install -g outil', null, 'fr');
    expect(install.text).toBe('Donne la commande qui installe son harness');
    expect(install.detail).toBe('npm install -g outil');
    expect(install.note).toContain('écrite par le plugin');
    const update = askOf('harnessUpdate:outil upgrade: --yes', null, 'fr');
    expect(update.text).toBe('Donne la commande qui met à jour son harness');
    expect(update.detail).toBe('outil upgrade: --yes');
  });

  it('names each variable it takes off, never the proxy nor the authorities Escouade gives', () => {
    const unset = askOf('harnessUnset:OUTIL_TOKEN', null, 'fr');
    expect(unset.text).toBe('Retire cette variable d’environnement de ce que lance son harness');
    expect(unset.detail).toBe('OUTIL_TOKEN');
    expect(unset.note).toContain('jamais le proxy');
  });
});

describe('the modes of a harness, at its installation (ESC-489)', () => {
  it('says that its agents run in Bypass alone, or that it declares no mode, and what Escouade does of it', () => {
    const bypass = askOf('harnessBypass:Unharnessed', null, 'fr');
    expect(bypass.text).toBe('Ses agents ne tournent qu’en Bypass : ils ne demandent jamais avant d’agir');
    expect(bypass.detail).toBe('Unharnessed');
    expect(bypass.red).toBe(true);
    expect(bypass.note).toContain('Autoriser le mode Bypass');
    const none = askOf('harnessNoMode:Outil', null, 'fr');
    expect(none.text).toBe('Ne déclare aucun mode : Escouade tient ses agents pour Bypass');
    expect(none.note).toContain('Autoriser le mode Bypass');
  });
});

// The points the manifest reserves (ESC-524, study T7): read, never served, so never said nor ranked.
describe('the points a manifest reserves', () => {
  const RESERVED = { menus: [{ id: 'm' }], statusBar: [{ id: 's' }], badges: [{ id: 'b' }], projectTemplates: [{ name: 'Modèle' }] };
  const manifest = (contributes: Record<string, unknown>) =>
    ({ id: 'dev.exemple.x', name: 'X', contributes, permissions: {}, activation: [] }) as never;
  const view = (contributes: Record<string, unknown>) =>
    ({ id: 'dev.exemple.x', description: 'Sa description', manifest: manifest(contributes) }) as never;

  it('says nothing of them in what a plugin adds: one that declares only them adds nothing', () => {
    expect(contributionsOf(manifest(RESERVED), 'fr')).toEqual([]);
    expect(contributionsOf(manifest({ ...RESERVED, commands: [{ name: 'lint' }] }), 'fr')).toEqual(['Une commande : lint']);
  });

  it('never ranks a plugin in « Outils » for them', () => {
    expect(categoriesOf(view(RESERVED))).toEqual([]);
    for (const [k, v] of Object.entries(RESERVED)) expect(categoriesOf(view({ [k]: v }))).toEqual([]);
    expect(categoriesOf(view({ ...RESERVED, commands: [{ name: 'lint' }] }))).toEqual(['tools']);
  });

  it('leaves them out of the line of Réglages › Plugins, which says its description instead', () => {
    expect(addsOf(view(RESERVED), 'fr')).toBe('Sa description');
  });

  it('is declared by no plugin of Escouade’s own: their ranks and what they add stay as they were', () => {
    const dir = resolve(process.cwd(), 'plugins');
    const manifests = readdirSync(dir)
      .map((d) => join(dir, d, 'escouade-plugin.json'))
      .filter((f) => existsSync(f));
    expect(manifests.length).toBeGreaterThanOrEqual(30);
    for (const f of manifests) {
      const m = JSON.parse(readFileSync(f, 'utf8')) as { contributes?: Record<string, unknown> };
      for (const k of [...Object.keys(RESERVED), 'escouadeYml']) expect(m.contributes ?? {}, f).not.toHaveProperty(k);
    }
  });
});

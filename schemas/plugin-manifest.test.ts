import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  belowContract,
  CONTRACT_MIN,
  FIELDS,
  FILE_ACCESS,
  FILE_ROOTS,
  hostCovered,
  message,
  reservedMessage,
  validate,
  validateText,
  writtenHost,
} from './plugin-manifest.mjs';

// The store's library on the cases it shares with the app's reader (src-tauri/src/plugins_tests.rs): the same fields
// wrong, for the same reasons, and the same fields ignored.

interface Case {
  name: string;
  builtin?: boolean;
  firstParty?: boolean;
  manifest: unknown;
  files?: Record<string, { png?: [number, number]; size?: number; text?: string }>;
  errors: string[];
  ignored: string[];
  /** The reserved points it declares, none when absent (ESC-524). */
  reserved?: string[];
}

const file = path.join(__dirname, '..', 'tests', 'fixtures', 'plugins', 'manifests.json');
const { cases } = JSON.parse(fs.readFileSync(file, 'utf8')) as { cases: Case[] };

/** The bytes of a case's file: a PNG's header of its width and height, padded to its size; or its text. */
function bytesOf(f: { png?: [number, number]; size?: number; text?: string }): Uint8Array {
  if (f.text !== undefined) return Uint8Array.from(f.text, (c) => c.charCodeAt(0));
  const b = new Uint8Array(Math.max(f.size ?? 64, 33));
  b.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]);
  const view = new DataView(b.buffer);
  view.setUint32(16, f.png![0]);
  view.setUint32(20, f.png![1]);
  b.set([8, 6, 0, 0, 0], 24);
  return b;
}

/** The reader of a case's folder, if it has files. */
const folderOf = (c: Case) => (c.files ? (p: string) => (c.files![p] ? bytesOf(c.files![p]!) : null) : undefined);

describe('plugin-manifest', () => {
  it('has the shared cases', () => expect(cases.length).toBeGreaterThanOrEqual(10));

  for (const c of cases) {
    it(c.name, () => {
      const r = validate(c.manifest, { builtin: c.builtin, firstParty: c.firstParty, file: folderOf(c) });
      const errors = r.errors.map((e) => `${e.path} ${e.code}`);
      expect(errors.sort()).toEqual([...c.errors].sort());
      expect(r.ignored.sort()).toEqual([...c.ignored].sort());
      expect(r.reserved.sort()).toEqual([...(c.reserved ?? [])].sort());
      expect(r.ok).toBe(c.errors.length === 0);
    });
  }

  it('says each field and why in the app’s words, in both languages', () => {
    const r = validate({
      id: 'Slack',
      name: 'Slack',
      version: '1.2.0',
      escouade: { api: 'v1', min: '2.1.0' },
      contributes: { settings: [{ key: 'a', type: 'colour', title: 'A' }] },
    });
    expect(r.errors.map((e) => message(e, 'fr'))).toEqual([
      "`id` n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).",
      '`escouade.api` n\'est pas une version majeure de l\'API des plugins (`"1"`).',
      "`contributes.settings[0].type` vaut l'un de : string, number, boolean, select, profile.",
    ]);
    expect(message(r.errors[0], 'en')).toBe('`id` is no reversed domain name in lower case (`dev.exemple.slack`).');
    expect(message(validate({}).errors[0], 'fr')).toBe('`id` manque.');
    expect(message(validateText('[]').errors[0], 'fr')).toBe('Le manifeste devrait être un objet.');
    expect(validateText('{ id: ').errors).toEqual([{ path: '', code: 'type', expected: 'object' }]);
    const icon = (png: [number, number], size?: number) =>
      validate(
        { id: 'dev.exemple.logo', name: 'Logo', version: '1.0.0', escouade: { api: '1', min: '2.2.0' }, icon: './icon.png' },
        { file: (p) => (p === 'icon.png' ? bytesOf({ png, size }) : null) },
      ).errors.map((e) => message(e, 'fr'));
    expect(icon([512, 512])).toEqual([]);
    expect(icon([200, 100], 2_000_000)).toEqual([
      '`icon` pèse plus de 1 Mo.',
      "`icon` n'est pas carrée (un PNG carré de 256 px au moins, 512 conseillé).",
      '`icon` fait moins de 256 px de côté (512 conseillé).',
    ]);
    const both = validate({ run: { command: 'bin/slack', js: 'dist/plugin.mjs' } }).errors.find((e) => e.path === 'run.js')!;
    expect(message(both, 'fr')).toBe(
      '`run.js` ne va pas avec `run.command`, `run.js` ou `run.wasm` : un programme est une commande, un bundle JS/TS ou un module Rust, un seul des trois.',
    );
    expect(message(both, 'en')).toBe(
      '`run.js` does not go with `run.command`, `run.js` or `run.wasm`: a program is a command, a JS/TS bundle or a Rust module, one of the three.',
    );
    const svg = validate({ icon: 'logo.svg' }).errors.find((e) => e.path === 'icon')!;
    expect(message(svg, 'en')).toBe('`icon` is no PNG (no SVG nor JPEG: a square PNG of 256 px at least).');
  });

  it('says the field of the contract and why it is refused there, as the core says it (ESC-531)', () => {
    const r = validate({
      id: 'dev.exemple.tunnel',
      name: 'Tunnel',
      version: '1.0.0',
      escouade: { api: '1', min: '2.2.0' },
      permissions: {
        secrets: ['profile:wifi'],
        commands: ['openvpn --config {path:project}', 'slirp4netns --pid={pid}', '{int}', 'nsenter * -t {pid}', 'a {float}'],
      },
      limits: { processes: 8 },
      contributes: {
        groups: [
          { id: 'g', title: 'G', fields: ['nowhere'] },
          { id: 'h', title: 'H', fields: [] },
        ],
        settings: [{ key: 'n', type: 'number', title: 'N', multiline: true, of: 'vpn' }],
        profiles: [
          {
            id: 'vpn',
            title: 'VPN',
            nameField: 'password',
            fields: [
              { key: 'password', secret: true, type: 'string', title: 'P' },
              { key: 'server', title: 'S', scope: 'project' },
            ],
          },
        ],
      },
    });
    const fr = r.errors.map((e) => message(e, 'fr'));
    for (const line of [
      "`permissions.commands[0]` `{path:project}` n'est pas un joker : un chemin n'est que `{path:temp}` ou `{path:plugin}`, jamais un fichier du projet ni d'un agent, que le dépôt choisirait.",
      '`permissions.commands[1]` `--pid={pid}` colle un joker à autre chose : un joker est un mot entier (`--pid {pid}`, jamais `--pid={pid}`).',
      "`permissions.commands[2]` commence par `{int}` : le premier mot est le nom d'un programme, trouvé sur le PATH, jamais un chemin, un `*` ni un joker.",
      "`permissions.commands[3]` a un `*` avant son dernier mot : `*` ne vient qu'à la fin, pour les mots qui suivent.",
      "`permissions.commands[4]` `{float}` n'est pas un joker (`{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`) : une accolade n'a pas sa place dans un autre mot.",
      '`permissions.secrets[0]` nomme un genre que `contributes.profiles` ne déclare pas, ou sans champ secret : `profile:<genre>` demande les secrets des profils de ce genre.',
      "`limits.reason` manque : au-delà de 4 programmes à la fois, une phrase dit pourquoi, que l'utilisateur lit à l'installation.",
      '`contributes.groups[0].fields[0]` ne nomme aucun réglage ni secret du plugin (`contributes.settings`, `contributes.secrets`).',
      '`contributes.groups[1].fields` est une liste vide : au moins un champ.',
      '`contributes.settings[0].multiline` ne va que sur un réglage `string` ou un secret.',
      '`contributes.settings[0].of` ne va que sur un réglage `"type": "profile"`, dont il dit le genre.',
      '`contributes.profiles[0].nameField` doit nommer un champ `string` du profil, ni secret ni sur plusieurs lignes : le nom du profil dans les listes.',
      "`contributes.profiles[0].fields[0].type` n'a pas sa place sur un champ secret d'un profil : un secret est un texte.",
      "`contributes.profiles[0].fields[1].scope` n'a pas sa place sur un champ de profil : un profil vaut pour chaque projet qui le choisit.",
      '`contributes.profiles[0].fields[1].type` manque : un champ de profil est d\'un type (`string`, `number`, `boolean`, `select`), ou `"secret": true`.',
    ])
      expect(fr).toContain(line);
    expect(r.errors.map((e) => message(e, 'en'))).toContain(
      '`limits.reason` is missing: beyond 4 programs at once, a sentence says why, which the user reads at the installation.',
    );
    const over = validate({
      id: 'dev.exemple.t',
      name: 'T',
      version: '1.0.0',
      escouade: { api: '1', min: '2.2.0' },
      limits: { processes: 17, reason: 'R' },
    });
    expect(over.errors.map((e) => message(e, 'en'))).toEqual([
      '`limits.processes` must be a whole number from 1 to 16: the programs at once, 4 without asking.',
    ]);
  });

  it('warns a plugin of the contract whose escouade.min is older, in the core’s words, never refusing it (ESC-531)', () => {
    const m = {
      id: 'dev.exemple.tunnel',
      name: 'Tunnel',
      version: '1.0.0',
      escouade: { api: '1', min: '2.2.0' },
      permissions: { commands: ['tunnel --port {port}'] },
      limits: { processes: 2 },
      contributes: { settings: [{ key: 'routes', type: 'string', scope: 'project', title: 'R' }] },
    };
    expect(validate(m).ok).toBe(true);
    expect(belowContract(m, 'en')).toBe(
      `\`escouade.min\` says 2.2.0, but this plugin takes settings of each project, jokers in \`permissions.commands\`, \`limits.processes\`, which Escouade serves since ${CONTRACT_MIN}: an older version would not serve them. \`escouade.min\` should say ${CONTRACT_MIN}.`,
    );
    expect(belowContract({ ...m, escouade: { api: '1', min: CONTRACT_MIN } })).toBeNull();
    expect(belowContract({ ...m, permissions: {}, limits: {}, contributes: {} })).toBeNull();
  });
});

describe('plugin-1.json', () => {
  const schema = JSON.parse(fs.readFileSync(path.join(__dirname, 'plugin-1.json'), 'utf8'));
  const pattern = (p: string) => new RegExp(p, 'u');

  it('holds patterns an editor can read, that say what the library says', () => {
    const inner = pattern(schema.properties.run.properties.command.pattern);
    for (const p of ['bin/x', 'bin\\x.exe', 'a..b/c']) expect(inner.test(p), p).toBe(true);
    for (const p of ['../x', 'bin/../../x', '/usr/bin/x', '\\x', 'C:\\x', 'bin\\..']) {
      expect(inner.test(p), p).toBe(false);
    }
    const id = pattern(schema.properties.id.pattern);
    expect(id.test('dev.exemple.slack')).toBe(true);
    expect(id.test('Slack')).toBe(false);
    expect(id.test('slack')).toBe(false);
    const version = pattern(schema.properties.version.pattern);
    expect(version.test('0.1.0-beta.1')).toBe(true);
    expect(version.test('1.2')).toBe(false);
    expect(schema.required).toEqual(['id', 'name', 'version', 'escouade']);
    const icon = pattern(schema.properties.icon.pattern);
    for (const p of ['icon.png', 'assets/Logo.PNG', './store/icon.png']) expect(icon.test(p), p).toBe(true);
    for (const p of ['icon.svg', '../icon.png', '/icon.png', 'C:\\icon.png', 'https://exemple.dev/icon.png', 'icon.png.svg']) {
      expect(icon.test(p), p).toBe(false);
    }
  });
});

describe('the hosts of permissions.network', () => {
  const corpus = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'tests', 'fixtures', 'plugins', 'manifests.json'), 'utf8'));

  it('cover a host as the app says, case by case', () => {
    for (const c of corpus.hosts.cases) expect(hostCovered(c.host, c.network), `${c.host} by ${c.network}`).toBe(c.covered);
  });

  it('are read from an address written whole, never from a template', () => {
    expect(writtenHost('https://user@Hooks.Slack.com:443/x?y#z')).toBe('hooks.slack.com');
    expect(writtenHost('https://hooks.slack.com@evil.dev/x')).toBe('evil.dev');
    expect(writtenHost('http://[::1]:8080/x')).toBe('::1');
    expect(writtenHost('{{secrets.webhook}}')).toBeNull();
    expect(writtenHost('https://{{data.host}}/x')).toBeNull();
    expect(writtenHost('ftp://hooks.slack.com')).toBeNull();
  });
});

// The two readers of a manifest, this library (the store, the SDK's `validate`, vendor/ in sdk/plugin) and the app's
// (crates/escouade-plugin-cli/src/manifest.rs, which writes plugin-1.json): the same fields, object by object (issue
// #43: a SDK that ignored `run.wasm`, `permissions.files` and `limits`). Their verdicts on the manifests of the
// repository are compared by src-tauri/src/plugins_tests.rs.
describe('the fields known', () => {
  const schema = JSON.parse(fs.readFileSync(path.join(__dirname, 'plugin-1.json'), 'utf8'));
  const at = (level: string) => (level === '' ? schema : schema.properties[level]);

  for (const level of Object.keys(FIELDS)) {
    it(`are those of plugin-1.json in ${level || 'the manifest'}`, () => {
      expect([...FIELDS[level]!].sort()).toEqual(Object.keys(at(level).properties).sort());
    });
  }

  it('cover every object of plugin-1.json', () => {
    const objects = Object.entries(schema.properties as Record<string, { properties?: object }>)
      .filter(([, v]) => v.properties)
      .map(([k]) => k);
    expect(
      Object.keys(FIELDS)
        .filter((k) => k !== '')
        .sort(),
    ).toEqual(objects.sort());
  });

  it('give the roots and the accesses of permissions.files of plugin-1.json', () => {
    const item = schema.properties.permissions.properties.files.items.properties;
    expect([...FILE_ROOTS].sort()).toEqual([...item.root.enum].sort());
    expect([...FILE_ACCESS].sort()).toEqual([...item.access.enum].sort());
  });
});

// The points the manifest reserves (ESC-524): said in the app's words, a warning beside a manifest that holds.
describe('the reserved points', () => {
  it('are said as reserved, with no effect in this version, in both languages, and refuse nothing', () => {
    const r = validate({
      id: 'dev.exemple.x',
      name: 'X',
      version: '1.0.0',
      escouade: { api: '1', min: '2.1.0' },
      contributes: { statusBar: [{ id: 's' }] },
    });
    expect(r.ok).toBe(true);
    expect(r.reserved).toEqual(['contributes.statusBar']);
    expect(reservedMessage('contributes.statusBar', 'fr')).toBe(
      "`contributes.statusBar` est réservé, sans effet dans cette version : Escouade n'en montre ni n'en appelle rien.",
    );
    expect(reservedMessage('contributes.statusBar', 'en')).toBe(
      '`contributes.statusBar` is reserved, with no effect in this version: Escouade shows and calls nothing of it.',
    );
  });
});

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import {
  createKey,
  keyFor,
  keychain,
  keysIn,
  locked,
  roleFor,
  readToken,
  reportVerdict,
  revoke,
  saveToken,
  sendReport,
  signVersion,
  summaryLines,
  type Admin,
} from '../../scripts/plugin-review/admin';
import { containerCompiler } from '../../scripts/plugin-review/container';
import { parseTarget } from '../../scripts/plugin-review/download';
import { review } from '../../scripts/plugin-review/review';
import { sandboxedCompiler } from '../../server/utils/compile';
import { readKeyFile, readPublicKey, readSignature, verifySigned } from '../../server/utils/minisign';
import { checkRevocations, type RevocationList } from '../../server/utils/revocation-list';
import { checkVersionSignature } from '../../server/utils/signature';
import { sha256 } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';
import { FAKE_ADMIN_TOKEN, hasRustFixture, startReviewStore } from '../fake/review-store.mjs';

// What the review sends to the store from the user's machine (scripts/plugin-review/admin.ts, docs/STORE.md §4.3, §4.5,
// §4.6): the report, the signature of a version, the revocation lists. Signed by a stand-in for minisign
// (tests/fake/fake-minisign.mjs) with keys made for the test in a temporary folder, never the store's, one for each
// role (ESC-592): the first hand's (store.key, without a password), the third parties' and the list's
// (third-party.key, locked), the backup (locked). When the real minisign is on the machine, a key is made and tried
// with it too, its password piped in.

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, '../../..');
const MINISIGN = join(here, '../fake/fake-minisign.mjs');
const FAKE_CLI = join(here, '../fake/fake-escouade-plugin.mjs');
const ENGINE = join(here, '../fake/fake-engine.mjs');
const TSX = resolve(here, '../../node_modules/tsx/dist/cli.mjs');

let root: string;
let signing: string;
let store: Awaited<ReturnType<typeof startReviewStore>>;
/** The key of the third parties and of the revocation list. */
const key = testKey();
const first = testKey();
const backup = testKey();
const stranger = testKey();
const keys = [readKeyFile('store.pub', first.pub)!, readKeyFile('third-party.pub', key.pub)!, readKeyFile('backup.pub', backup.pub)!];

beforeAll(async () => {
  root = mkdtempSync(join(tmpdir(), 'review-admin-test-'));
  signing = join(root, 'signing');
  mkdirSync(signing);
  store = await startReviewStore();
  for (const [name, text] of [
    ['store.key', first.open],
    ['third-party.key', key.locked],
    ['backup.key', backup.locked],
    ['stranger.key', stranger.locked],
    ['first-locked.key', first.locked],
    ['third-open.key', key.open],
  ] as const)
    writeFileSync(join(signing, name), text);
  // The run.js plugin compiled again by the fake escouade-plugin: the same module (1.0.0), another compiler (1.0.2).
  for (const t of [
    'dev.escouade.discord@1.1.0',
    'dev.exemple.team-rules@1.0.0',
    'io.github.q7x.slak-notify@0.0.1',
    'dev.exemple.rules-js@1.0.0',
    'dev.exemple.rules-js@1.0.2',
    // Without the repository's Rust fixture (the store's image), no run.wasm plugin.
    ...(hasRustFixture ? ['dev.exemple.rust-lines@1.0.0', 'dev.exemple.rust-lines@1.0.1', 'dev.exemple.rust-lines@1.0.2'] : []),
  ])
    await review({
      ...parseTarget(t)!,
      source: { store: store.url, githubApi: store.githubApi },
      work: join(root, 'work'),
      reports: join(root, 'reports'),
      bench: { engine: null, cli: null, image: 'img', timeout: 10_000, local: FAKE_CLI },
      compile: sandboxedCompiler({ command: [process.execPath, FAKE_CLI], isolation: 'none' }),
      container: containerCompiler({
        engine: { program: process.execPath, prefix: [ENGINE] },
        cli: '/opt/escouade-plugin',
        image: 'img',
        work: join(root, 'work'),
      }),
      benchWhy: 'aucun moteur de conteneur (test)',
      repo: REPO,
      now: new Date('2026-10-29T10:00:00Z'),
    });
  // Seven reviews, each of which starts node several times.
}, 180_000);
afterAll(async () => {
  await store.close();
  rmSync(root, { recursive: true, force: true });
});

const admin = (over: Partial<Admin> = {}): Admin => ({
  source: { store: store.url, githubApi: store.githubApi },
  token: FAKE_ADMIN_TOKEN,
  reports: join(root, 'reports'),
  keys,
  minisign: { cmd: MINISIGN, dir: signing },
  work: join(root, 'work'),
  log: () => {},
  ...over,
});
const last = (route: string) => [...store.received].reverse().find((r) => r.route === route)?.body;
const sent = () => store.received.length;

describe('the verdict of a report', () => {
  it('reads it in the reader’s section alone, the last, not in what a plugin got quoted above it', () => {
    const quoted = '## Constats de la revue\n\n**Verdict final : Vérifier**\n';
    expect(reportVerdict(`# Rapport\n\n${quoted}\n## Constats de la revue\n\n_À rédiger._\n`, {} as never)).toBeNull();
    expect(reportVerdict(`# Rapport\n\n${quoted}`, {} as never)).toBe('Vérifier');
    expect(reportVerdict('**Verdict final : Vérifier**', { proposed: { verdict: null } } as never)).toBeNull();
  });
});

describe('the signature of a version', () => {
  it('signs the very version.json the review read, checks it, and sends it with the public summary', async () => {
    const done = await signVersion(admin(), 'dev.exemple.team-rules', '1.0.0', new Date('2026-10-29T12:00:00Z'));
    expect(done.keyId).toBe(key.id);
    const body = last('signature')!;
    expect(body).toMatchObject({ id: 'dev.exemple.team-rules', version: '1.0.0' });
    const json = await (await fetch(`${store.url}/api/v1/plugins/dev.exemple.team-rules/1.0.0/version.json`)).text();
    const checked = checkVersionSignature(json, String(body.minisig), keys, 'dev.exemple.team-rules', '1.0.0');
    expect(checked).toMatchObject({ date: '2026-10-29' });
    expect(String(body.minisig)).toContain('trusted comment: escouade-verified-v1 dev.exemple.team-rules@1.0.0 2026-10-29');
    const summary = body.summary as { fr: string; en: string };
    expect(summary.fr.split('\n')[0]).toMatch(/^- Permissions relues : /);
    expect(summary.en.split('\n')[0]).toMatch(/^- Permissions read: /);
    expect(summary.fr).not.toContain('#');
    expect(readFileSync(join(root, 'reports', 'dev.exemple.team-rules', '1.0.0.minisig'), 'utf8')).toBe(body.minisig);
    // Nothing of the secret key went to the store.
    expect(JSON.stringify(store.received)).not.toContain('PRIVATE KEY');
  });

  it('signs with the backup key as well', async () => {
    const done = await signVersion(
      admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'backup.key') } }),
      'dev.exemple.team-rules',
      '1.0.0',
    );
    expect(done.keyId).toBe(backup.id);
  });

  it('sends nothing when the review found a blocker, the keys are missing, or the key is not the store’s', async () => {
    const before = sent();
    await expect(signVersion(admin(), 'io.github.q7x.slak-notify', '0.0.1')).rejects.toThrow(/empêche « Vérifier »/);
    await expect(signVersion(admin({ keys: [] }), 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(/aucune clé publique/);
    await expect(
      signVersion(
        admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'stranger.key') } }),
        'dev.exemple.team-rules',
        '1.0.0',
      ),
    ).rejects.toThrow(/ne passe pas avec les clés publiques/);
    await expect(
      signVersion(
        admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'nowhere.key') } }),
        'dev.exemple.team-rules',
        '1.0.0',
      ),
    ).rejects.toThrow(/pas de clé de signature/);
    await expect(signVersion(admin(), 'dev.exemple.high-contrast', '1.0.0')).rejects.toThrow(/lance d’abord npm run plugin-review/);
    process.env.FAKE_MINISIGN_FAIL = '1';
    try {
      await expect(signVersion(admin(), 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(/minisign a échoué/);
    } finally {
      delete process.env.FAKE_MINISIGN_FAIL;
    }
    expect(sent()).toBe(before);
  });

  it('signs a run.js plugin whose module the review compiled again to the same fingerprint (ESC-324)', async () => {
    const done = await signVersion(admin(), 'dev.exemple.rules-js', '1.0.0');
    expect(done.keyId).toBe(key.id);
    expect(last('signature')).toMatchObject({ id: 'dev.exemple.rules-js', version: '1.0.0' });
    expect((last('signature')!.summary as { fr: string }).fr).toContain('module WASM recompilé, même empreinte');
  });

  it('refuses a run.js plugin whose module was not compiled again, or not to the same fingerprint (ESC-324)', async () => {
    const before = sent();
    // Another escouade-plugin than the store's: not compiled again.
    await expect(signVersion(admin(), 'dev.exemple.rules-js', '1.0.2')).rejects.toThrow(
      /la recompilation du module WASM n’a pas donné la même empreinte : .*escouade-plugin 2.1.9/,
    );
    const data = join(root, 'reports', 'dev.exemple.rules-js', '1.0.0.json');
    const kept = readFileSync(data, 'utf8');
    try {
      // A review written before the recompilation, or by hand: none is noted.
      const j = JSON.parse(kept);
      delete j.recompile;
      writeFileSync(data, JSON.stringify(j));
      await expect(signVersion(admin(), 'dev.exemple.rules-js', '1.0.0')).rejects.toThrow(/elle n’a pas été faite/);
      // Its module noted, but another than version.json's.
      j.recompile = { ...JSON.parse(kept).recompile, sha256: sha256('other') };
      writeFileSync(data, JSON.stringify(j));
      await expect(signVersion(admin(), 'dev.exemple.rules-js', '1.0.0')).rejects.toThrow(/n’a pas été recompilé à la même empreinte/);
    } finally {
      writeFileSync(data, kept);
    }
    expect(sent()).toBe(before);
  });

  it.skipIf(!hasRustFixture)('signs a run.wasm whose module the review compiled again, and nothing else (ESC-337)', async () => {
    const done = await signVersion(admin(), 'dev.exemple.rust-lines', '1.0.0');
    expect(done.keyId).toBe(key.id);
    expect((last('signature')!.summary as { fr: string }).fr).toContain('module WASM recompilé, même empreinte');
    const before = sent();
    // Another fingerprint: the review stopped there, « Refuser ».
    await expect(signVersion(admin(), 'dev.exemple.rust-lines', '1.0.1')).rejects.toThrow(
      /n’est pas la recompilation des sources de l’archive/,
    );
    // Another rustc-canon than the store's: not compiled again.
    await expect(signVersion(admin(), 'dev.exemple.rust-lines', '1.0.2')).rejects.toThrow(
      /la recompilation du module WASM n’a pas donné la même empreinte : .*rustc-canon dddddddddddd/,
    );
    expect(sent()).toBe(before);
  });

  it('refuses a version.json that is not the one the review read', async () => {
    const data = join(root, 'reports', 'dev.exemple.team-rules', '1.0.0.json');
    const kept = readFileSync(data, 'utf8');
    writeFileSync(data, kept.replace(/"versionSha256": "[0-9a-f]+"/, `"versionSha256": "${sha256('other')}"`));
    try {
      await expect(signVersion(admin(), 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(/a changé depuis la revue/);
    } finally {
      writeFileSync(data, kept);
    }
  });

  it('is refused by the store without the administration token', async () => {
    await expect(signVersion(admin({ token: 'esc_admin_wrong' }), 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(
      /le store refuse \(401\)/,
    );
  });
});

describe('the key of each role (ESC-592)', () => {
  it('signs dev.escouade.* with the key of the first hand, without a password, and the others with the third parties’', async () => {
    expect(roleFor('dev.escouade.discord')).toBe('first-hand');
    expect(roleFor('dev.exemple.team-rules')).toBe('third-party');
    expect(roleFor('dev.escouadex.trap')).toBe('third-party');
    expect(keyFor(admin(), 'first-hand')).toBe(join(signing, 'store.key'));
    expect(keyFor(admin(), 'third-party')).toBe(join(signing, 'third-party.key'));
    expect(keyFor(admin(), 'revocation')).toBe(join(signing, 'third-party.key'));
    const said: string[] = [];
    const first = await signVersion(admin({ log: (l) => said.push(l) }), 'dev.escouade.discord', '1.1.0');
    expect(first).toMatchObject({ keyId: keys[0]!.id, role: 'first-hand' });
    expect(said.join('\n')).toContain('la clé de la première main');
    expect(said.join('\n')).not.toContain('mot de passe');
    said.length = 0;
    const third = await signVersion(admin({ log: (l) => said.push(l) }), 'dev.exemple.team-rules', '1.0.0');
    expect(third).toMatchObject({ keyId: key.id, role: 'third-party' });
    expect(said.join('\n')).toContain('la clé des tiers :');
    expect(said.join('\n')).toContain('minisign demande son mot de passe');
  });

  it('never seals a third party with the key of the first hand, even locked and given by hand', async () => {
    const before = sent();
    const forced = admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'first-locked.key') } });
    await expect(signVersion(forced, 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(
      /ne passe pas .*of the first hand, which seals dev\.escouade\.\* plugins only/,
    );
    // Nor the first hand with the third parties’.
    const other = admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'third-party.key') } });
    await expect(signVersion(other, 'dev.escouade.discord', '1.1.0')).rejects.toThrow(/does not seal Escouade's plugins/);
    // Nor the revocation list with the first hand’s.
    await expect(
      revoke(forced, { id: 'x.y', versions: '*', action: 'disable', reason: { fr: 'a', en: 'b' }, notify: false }),
    ).rejects.toThrow(/ne passe pas .*signs no revocation list/);
    expect(sent()).toBe(before);
  });

  it('refuses a key of the third parties, of the list or the backup without a password, before minisign runs', async () => {
    const before = sent();
    const open = admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'third-open.key') } });
    await expect(signVersion(open, 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(/n’a pas de mot de passe/);
    await expect(
      revoke(open, { id: 'x.y', versions: '*', action: 'disable', reason: { fr: 'a', en: 'b' }, notify: false }),
    ).rejects.toThrow(/n’a pas de mot de passe/);
    const dir = mkdtempSync(join(root, 'open-'));
    writeFileSync(join(dir, 'third-party.key'), key.open);
    await expect(signVersion(admin({ minisign: { cmd: MINISIGN, dir } }), 'dev.exemple.team-rules', '1.0.0')).rejects.toThrow(
      /la clé des tiers doit en avoir un/,
    );
    await expect(
      signVersion(admin({ minisign: { cmd: MINISIGN, dir: join(root, 'none') } }), 'dev.exemple.team-rules', '1.0.0'),
    ).rejects.toThrow(/create-key third-party/);
    writeFileSync(join(dir, 'third-party.key'), 'not a key');
    expect(() => keyFor(admin({ minisign: { cmd: MINISIGN, dir } }), 'third-party')).toThrow(/ne se lit pas/);
    expect(sent()).toBe(before);
    expect(locked(key.locked)).toBe(true);
    expect(locked(key.open)).toBe(false);
    expect(locked(key.secret)).toBeNull();
  });

  it('makes the key of the third parties with a password, beside nothing it would replace', () => {
    const dir = mkdtempSync(join(root, 'create-'));
    const o = { cmd: MINISIGN, dir: join(dir, 'signing'), keys: join(dir, 'keys') };
    const made = createKey(o);
    expect(made.secret).toBe(join(dir, 'signing', 'third-party.key'));
    expect(made.pub).toBe(join(dir, 'keys', 'third-party.pub'));
    expect(locked(readFileSync(made.secret, 'utf8'))).toBe(true);
    const pub = readKeyFile('third-party.pub', readFileSync(made.pub, 'utf8'))!;
    expect(pub).toMatchObject({ id: made.id, roles: ['third-party', 'revocation'] });
    expect(readPublicKey(made.line)!.id).toBe(made.id);
    // Never replaced.
    expect(() => createKey(o)).toThrow(/existe déjà/);
    // A key made without a password is taken off.
    const bare = { ...o, dir: join(dir, 'bare'), keys: join(dir, 'bare-keys') };
    process.env.FAKE_MINISIGN_OPEN = '1';
    try {
      expect(() => createKey(bare)).toThrow(/sans mot de passe/);
    } finally {
      delete process.env.FAKE_MINISIGN_OPEN;
    }
    expect(existsSync(join(bare.dir, 'third-party.key'))).toBe(false);
    expect(existsSync(join(bare.keys, 'third-party.pub'))).toBe(false);
  });

  // The command the user runs, with the real minisign when the machine has it: the password goes to minisign by its
  // input (as typed in a terminal), the key is locked, it signs with that password and not with another.
  const real = spawnSync('minisign', ['-v'], { encoding: 'utf8' });
  it.skipIf(real.status !== 0)('makes a real key with create-key third-party, which signs with its password only', () => {
    const dir = mkdtempSync(join(root, 'real-'));
    const env = { ...process.env, ESCOUADE_SIGNING_DIR: join(dir, 'signing'), STORE_KEYS_DIR: join(dir, 'keys') };
    delete env.ESCOUADE_MINISIGN;
    const made = spawnSync(process.execPath, [TSX, join(here, '../../scripts/plugin-review/index.ts'), 'create-key', 'third-party'], {
      input: 'a test password\na test password\n',
      encoding: 'utf8',
      env,
    });
    expect(made.status, made.stderr).toBe(0);
    expect(made.stdout).toContain('La clé des tiers et de la révocation est créée');
    expect(made.stdout).toContain('roles: &[Role::ThirdParty, Role::Revocation]');
    const secret = join(dir, 'signing', 'third-party.key');
    expect(locked(readFileSync(secret, 'utf8'))).toBe(true);
    const pub = readKeyFile('third-party.pub', readFileSync(join(dir, 'keys', 'third-party.pub'), 'utf8'))!;
    const file = join(dir, 'version.json');
    writeFileSync(file, '{"format":1,"id":"dev.exemple.slack","version":"1.2.0"}');
    const comment = 'escouade-verified-v1 dev.exemple.slack@1.2.0 2026-11-22';
    const wrong = spawnSync('minisign', ['-S', '-s', secret, '-m', file, '-t', comment], { input: 'another one\n', encoding: 'utf8' });
    expect(wrong.status).not.toBe(0);
    const signed = spawnSync('minisign', ['-S', '-s', secret, '-m', file, '-t', comment], { input: 'a test password\n', encoding: 'utf8' });
    expect(signed.status, signed.stderr).toBe(0);
    const minisig = readFileSync(`${file}.minisig`, 'utf8');
    expect(verifySigned(readFileSync(file), readSignature(minisig)!, [pub])?.id).toBe(pub.id);
    expect(checkVersionSignature(readFileSync(file), minisig, [pub], 'dev.exemple.slack', '1.2.0')).toMatchObject({ role: 'third-party' });
    // A second run replaces nothing.
    const again = spawnSync(process.execPath, [TSX, join(here, '../../scripts/plugin-review/index.ts'), 'create-key', 'third-party'], {
      input: 'x\nx\n',
      encoding: 'utf8',
      env,
    });
    expect(again.status).not.toBe(0);
    expect(again.stderr).toContain('existe déjà');
  });
});

describe('the report', () => {
  it('goes to the queue with the verdict the reader settled, else the proposed one', async () => {
    expect(await sendReport(admin(), 'dev.exemple.team-rules', '1.0.0')).toEqual({ verdict: null });
    expect(last('report')).toMatchObject({ id: 'dev.exemple.team-rules', version: '1.0.0', report: expect.stringContaining('# Revue de') });
    expect(await sendReport(admin(), 'io.github.q7x.slak-notify', '0.0.1')).toEqual({ verdict: 'Demander des changements' });
    expect(reportVerdict('## Constats de la revue\n\n- **Verdict final : Refuser** : exfiltre ~/.ssh.\n', {} as never)).toBe('Refuser');
    expect(
      reportVerdict('- Verdict proposé : **Vérifier**\n## Constats de la revue\n\nrien\n', { proposed: { verdict: null } } as never),
    ).toBeNull();
  });
});

describe('the revocation list', () => {
  it('goes up by one serial from the store’s, signed, its entries kept, and is checked before it is sent', async () => {
    const a = admin();
    const reason = { fr: 'Envoie ~/.ssh à un hôte non déclaré', en: 'Sends ~/.ssh to an undeclared host' };
    expect(await revoke(a, { id: 'io.github.q7x.slak-notify', versions: '*', action: 'disable', reason, notify: false })).toEqual({
      serial: 1,
    });
    let body = last('revocations')!;
    expect(body.notify).toBe(false);
    const one = checkRevocations(String(body.json), String(body.minisig), keys);
    if ('error' in one) throw new Error(one.error);
    expect(one.list.entries).toEqual([
      { id: 'io.github.q7x.slak-notify', versions: '*', action: 'disable', reason, date: expect.any(String) },
    ]);
    expect(
      await revoke(a, { id: 'dev.exemple.slack', versions: ['1.1.0'], action: 'unverify', reason: { fr: 'a', en: 'b' }, notify: true }),
    ).toEqual({ serial: 2 });
    expect(
      await revoke(a, {
        id: 'dev.exemple.slack',
        versions: ['1.1.0'],
        action: 'unverify',
        reason: { fr: '', en: '' },
        lift: true,
        notify: true,
      }),
    ).toEqual({
      serial: 3,
    });
    body = last('revocations')!;
    const three = checkRevocations(String(body.json), String(body.minisig), keys, 2) as { list: RevocationList };
    expect(three.list.entries.map((e) => e.id)).toEqual(['io.github.q7x.slak-notify']);
    // A key revoked, signed by the backup key: the list keeps it from then on.
    const b = admin({ minisign: { cmd: MINISIGN, dir: signing, key: join(signing, 'backup.key') } });
    expect(
      await revoke(b, {
        id: '',
        versions: '*',
        action: 'unverify',
        reason: { fr: 'perdue', en: 'lost' },
        revokeKeys: [key.id],
        notify: false,
      }),
    ).toEqual({
      serial: 4,
    });
    const four = checkRevocations(String(last('revocations')!.json), String(last('revocations')!.minisig), keys, 3) as {
      list: RevocationList;
    };
    expect(four.list.keys_revoked).toEqual([key.id]);
    expect(four.list.entries).toHaveLength(1);
    // The revoked key cannot sign the next one.
    await expect(revoke(a, { id: 'x.y', versions: '*', action: 'disable', reason: { fr: 'a', en: 'b' }, notify: true })).rejects.toThrow(
      /ne passe pas/,
    );
    expect(existsSync(join(root, 'reports', 'revocations', '4.json.minisig'))).toBe(true);
  });

  it('wants a reason in both languages, and keys', async () => {
    await expect(
      revoke(admin(), { id: 'x.y', versions: '*', action: 'disable', reason: { fr: 'a', en: '' }, notify: true }),
    ).rejects.toThrow(/la raison/);
    await expect(
      revoke(admin({ keys: [] }), { id: 'x.y', versions: '*', action: 'disable', reason: { fr: 'a', en: 'b' }, notify: true }),
    ).rejects.toThrow(/aucune clé publique/);
  });
});

describe('the token and the keys', () => {
  it('comes from ESCOUADE_STORE_TOKEN, else the keychain, and is written to it through the standard input', () => {
    const calls: { cmd: string; args: string[]; input?: string }[] = [];
    const exec = (cmd: string, args: string[], input?: string) => {
      calls.push({ cmd, args, input });
      return { status: 0, stdout: 'esc_admin_from-keychain\n', stderr: '', pid: 0, output: [], signal: null };
    };
    expect(readToken({ ESCOUADE_STORE_TOKEN: ' esc_admin_env ' }, 'linux', exec)).toBe('esc_admin_env');
    expect(calls).toHaveLength(0);
    for (const platform of ['linux', 'darwin', 'win32'] as const) {
      expect(readToken({}, platform, exec)).toBe('esc_admin_from-keychain');
      expect(saveToken('esc_admin_secret', platform, exec)).toBe(true);
      const write = calls.at(-1)!;
      expect(write.args.join(' ')).not.toContain('esc_admin_secret');
      expect(write.input).toContain('esc_admin_secret');
    }
    expect(keychain('aix')).toBeNull();
    expect(readToken({}, 'aix', exec)).toBeNull();
  });

  it('reads the public keys of a folder, and the summary’s lines', () => {
    const dir = join(root, 'keys');
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, 'store.pub'), first.pub);
    writeFileSync(join(dir, 'backup.pub'), backup.pub);
    writeFileSync(join(dir, 'third-party.pub'), key.pub);
    writeFileSync(join(dir, 'other.pub'), stranger.pub);
    writeFileSync(join(dir, 'README.md'), '# not a key');
    // Each key with the roles of its file's name; a file of no role is left out.
    expect(keysIn(dir).map((k) => [k.id, k.roles])).toEqual([
      [backup.id, ['backup']],
      [first.id, ['first-hand']],
      [key.id, ['third-party', 'revocation']],
    ]);
    expect(keysIn(join(root, 'none'))).toEqual([]);
    expect(keysIn(resolve(here, '../../keys'))).toEqual(keysIn(resolve(here, '../../keys')));
    expect(summaryLines('# t\n\nRelu le x.\n\n- a\n- b\n')).toBe('- a\n- b');
  });
});

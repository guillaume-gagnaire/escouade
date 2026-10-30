import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import {
  keychain,
  keysIn,
  readToken,
  reportVerdict,
  revoke,
  saveToken,
  sendReport,
  signVersion,
  summaryLines,
  type Admin,
} from '../../scripts/plugin-review/admin';
import { parseTarget } from '../../scripts/plugin-review/download';
import { review } from '../../scripts/plugin-review/review';
import { readPublicKey } from '../../server/utils/minisign';
import { checkRevocations, type RevocationList } from '../../server/utils/revocation-list';
import { checkVersionSignature } from '../../server/utils/signature';
import { sha256 } from '../../server/utils/versions';
import { testKey } from '../fake/minisign.mjs';
import { FAKE_ADMIN_TOKEN, startReviewStore } from '../fake/review-store.mjs';

// What the review sends to the store from the user's machine (scripts/plugin-review/admin.ts, docs/STORE.md §4.3, §4.5,
// §4.6): the report, the signature of a version, the revocation lists. Signed by a stand-in for minisign
// (tests/fake/fake-minisign.mjs) with keys made for the test in a temporary folder, never the store's.

const here = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(here, '../../..');
const MINISIGN = join(here, '../fake/fake-minisign.mjs');

let root: string;
let store: Awaited<ReturnType<typeof startReviewStore>>;
const key = testKey();
const backup = testKey();
const stranger = testKey();
const keys = [readPublicKey(key.pub)!, readPublicKey(backup.pub)!];

beforeAll(async () => {
  root = mkdtempSync(join(tmpdir(), 'review-admin-test-'));
  store = await startReviewStore();
  for (const [name, k] of [
    ['store.key', key],
    ['backup.key', backup],
    ['stranger.key', stranger],
  ] as const)
    writeFileSync(join(root, name), k.secret);
  for (const t of ['dev.exemple.team-rules@1.0.0', 'io.github.q7x.slak-notify@0.0.1'])
    await review({
      ...parseTarget(t)!,
      source: { store: store.url, githubApi: store.githubApi },
      work: join(root, 'work'),
      reports: join(root, 'reports'),
      bench: { engine: null, cli: null, image: 'img', timeout: 10_000 },
      benchWhy: 'aucun moteur de conteneur (test)',
      repo: REPO,
      now: new Date('2026-10-29T10:00:00Z'),
    });
});
afterAll(async () => {
  await store.close();
  rmSync(root, { recursive: true, force: true });
});

const admin = (over: Partial<Admin> = {}): Admin => ({
  source: { store: store.url, githubApi: store.githubApi },
  token: FAKE_ADMIN_TOKEN,
  reports: join(root, 'reports'),
  keys,
  minisign: { cmd: MINISIGN, key: join(root, 'store.key') },
  work: join(root, 'work'),
  ...over,
});
const last = (route: string) => [...store.received].reverse().find((r) => r.route === route)?.body;
const sent = () => store.received.length;

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
      admin({ minisign: { cmd: MINISIGN, key: join(root, 'backup.key') } }),
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
      signVersion(admin({ minisign: { cmd: MINISIGN, key: join(root, 'stranger.key') } }), 'dev.exemple.team-rules', '1.0.0'),
    ).rejects.toThrow(/ne passe pas avec les clés publiques/);
    await expect(
      signVersion(admin({ minisign: { cmd: MINISIGN, key: join(root, 'nowhere.key') } }), 'dev.exemple.team-rules', '1.0.0'),
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
    const b = admin({ minisign: { cmd: MINISIGN, key: join(root, 'backup.key') } });
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
    writeFileSync(join(dir, 'store.pub'), key.pub);
    writeFileSync(join(dir, 'backup.pub'), backup.pub);
    writeFileSync(join(dir, 'README.md'), '# not a key');
    expect(keysIn(dir).map((k) => k.id)).toEqual([backup.id, key.id]);
    expect(keysIn(join(root, 'none'))).toEqual([]);
    expect(keysIn(resolve(here, '../../keys'))).toEqual(keysIn(resolve(here, '../../keys')));
    expect(summaryLines('# t\n\nRelu le x.\n\n- a\n- b\n')).toBe('- a\n- b');
  });
});

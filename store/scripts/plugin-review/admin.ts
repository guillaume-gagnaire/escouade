import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { join } from 'node:path';
import { FIRST_HAND_PREFIX, readKeyFile, readPublicKey, type PublicKey } from '../../server/utils/minisign';
import {
  checkRevocations,
  parseRevocations,
  REVOCATIONS_COMMENT,
  type RevocationEntry,
  type RevocationList,
} from '../../server/utils/revocation-list';
import { checkVersionSignature, verifiedComment } from '../../server/utils/signature';
import { sha256, wasmOf } from '../../server/utils/versions';
import { catalog, Integrity, type Source } from './download';
import { blockers, type ReviewData } from './report';

// What the review sends to the store from the user's machine (docs/STORE.md §4.3, §4.5, §4.6): the report, the
// signature of a version the user decided to verify, and the revocation list. The signature is made here by minisign
// with the key of its role (ESC-592, audit SEC-03): `~/.escouade-signing/store.key`, the key of the first hand, for
// `dev.escouade.*` alone; `~/.escouade-signing/third-party.key`, with a password, for every other plugin and for the
// revocation list. A key of the third parties or of the list without a password is refused before minisign runs; its
// password is asked in the terminal by minisign itself at each signature, and never passes through this script. The
// key never leaves the machine, the store gets the `.minisig` alone and checks it with the public keys (and their
// roles) before taking it in. Nothing is sent before the same check passes here: a mistake is not published. The administration token
// comes from ESCOUADE_STORE_TOKEN or the system's keychain, never from a file of the repository nor an argument.

export interface Admin {
  source: Source;
  /** The administration token (shown once by the store, kept in the keychain). */
  token: string;
  /** The reports of the review (~/.escouade-review). */
  reports: string;
  /** The public keys the app embeds (`store/keys/*.pub`). */
  keys: PublicKey[];
  /**
   * How minisign is run: its command (a `.mjs` runs with node, for the tests), the folder of the secret keys
   * (`~/.escouade-signing`, where each role has its file) and, given, the one key to sign with whatever the role (the
   * backup, ESCOUADE_SIGNING_KEY).
   */
  minisign: { cmd: string; dir: string; key?: string };
  /** Where the version.json and the lists to sign are written, out of the repository. */
  work: string;
  log?: (line: string) => void;
}

/** The public keys of `dir` (`*.pub`), each with the roles of its name (`ROLES_BY_FILE`). */
export function keysIn(dir: string): PublicKey[] {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => n.endsWith('.pub'))
    .sort()
    .flatMap((n) => {
      const k = readKeyFile(n, readFileSync(join(dir, n), 'utf8'));
      return k ? [k] : [];
    });
}

/** The folder of the secret keys: ESCOUADE_SIGNING_DIR, else `~/.escouade-signing` (docs/STORE.md §4.5). */
export const signingDir = (env: NodeJS.ProcessEnv) => env.ESCOUADE_SIGNING_DIR ?? join(homedir(), '.escouade-signing');

/** What a signature is for: a plugin of the first hand, a third party's, or the revocation list. */
export type SigningRole = 'first-hand' | 'third-party' | 'revocation';

/** The role a seal of plugin `id` is signed in: the first hand for `dev.escouade.*`, the third parties' otherwise. */
export const roleFor = (id: string): SigningRole => (id.startsWith(FIRST_HAND_PREFIX) ? 'first-hand' : 'third-party');

/** The secret key's file of each role in the folder of the keys: the third parties' signs the list as well. */
export const KEY_FILES: Readonly<Record<SigningRole, string>> = {
  'first-hand': 'store.key',
  'third-party': 'third-party.key',
  revocation: 'third-party.key',
};

/** The key's name, for a person. */
const KEY_NAMES: Readonly<Record<SigningRole, string>> = {
  'first-hand': 'la clé de la première main',
  'third-party': 'la clé des tiers',
  revocation: 'la clé des tiers et de la révocation',
};

/**
 * Whether the minisign secret key `text` is locked by a password: its second line, in base64, starts with the
 * algorithm (`Ed`) then the one that derives the key from the password (`Sc`, scrypt; two zero bytes without one).
 * Nothing else of the key is read. Null when it does not read as a key of minisign.
 */
export function locked(text: string): boolean | null {
  const line = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean)[1];
  if (!line || !/^[A-Za-z0-9+/]+={0,2}$/.test(line)) return null;
  const head = Buffer.from(line.slice(0, 8), 'base64').subarray(0, 4);
  if (head.toString('latin1', 0, 2) !== 'Ed') return null;
  if (head.toString('latin1', 2, 4) === 'Sc') return true;
  if (head[2] === 0 && head[3] === 0) return false;
  return null;
}

/**
 * The secret key that signs in `role`: the one given (ESCOUADE_SIGNING_KEY, the backup) or the role's file. Every key but
 * the first hand's must have a password (the third parties', the list's, the backup): one without is refused, nothing
 * is signed.
 */
export function keyFor(a: Admin, role: SigningRole): string {
  const path = a.minisign.key ?? join(a.minisign.dir, KEY_FILES[role]);
  if (!existsSync(path))
    throw new Error(
      role === 'first-hand' || a.minisign.key
        ? `pas de clé de signature à ${path} : la procédure de docs/STORE.md §4.5 la crée (ESCOUADE_SIGNING_KEY pour une autre)`
        : `pas de ${KEY_NAMES[role]} à ${path} : crée-la sur ton poste par npm run plugin-review -- create-key third-party (docs/STORE.md §4.5)`,
    );
  if (role !== 'first-hand' || a.minisign.key) {
    const l = locked(readFileSync(path, 'utf8'));
    if (l !== true)
      throw new Error(
        l === false
          ? `${path} n’a pas de mot de passe : ${a.minisign.key ? 'la clé donnée' : KEY_NAMES[role]} doit en avoir un (minisign -C -s ${path} lui en donne un) ; rien n’est signé`
          : `${path} ne se lit pas comme une clé secrète de minisign : rien n’est signé`,
      );
  }
  return path;
}

// ---- The token, in the keychain ----

const SERVICE = 'escouade-store';
const ACCOUNT = 'admin';

type Run = (cmd: string, args: string[], input?: string) => SpawnSyncReturns<string>;
const run: Run = (cmd, args, input) => spawnSync(cmd, args, { input, encoding: 'utf8', windowsHide: true });

/** The keychain's commands of each system: read the token, write it (given on the standard input, never as an argument). */
export function keychain(
  platform: NodeJS.Platform,
): { read: [string, string[]]; write: (token: string) => [string, string[], string] } | null {
  if (platform === 'darwin')
    return {
      read: ['security', ['find-generic-password', '-s', SERVICE, '-a', ACCOUNT, '-w']],
      // `security -i` reads its commands from the standard input: the token is not in the process's arguments.
      write: (t) => ['security', ['-i'], `add-generic-password -U -s ${SERVICE} -a ${ACCOUNT} -w "${t}"\n`],
    };
  if (platform === 'linux')
    return {
      read: ['secret-tool', ['lookup', 'service', SERVICE, 'account', ACCOUNT]],
      write: (t) => ['secret-tool', ['store', '--label=Escouade Store', 'service', SERVICE, 'account', ACCOUNT], t],
    };
  if (platform === 'win32') {
    // The Credential Locker of Windows (PasswordVault), through Windows PowerShell 5.1 (PowerShell 7 does not load WinRT).
    const vault =
      '[void][Windows.Security.Credentials.PasswordVault,Windows.Security.Credentials,ContentType=WindowsRuntime];$v=New-Object Windows.Security.Credentials.PasswordVault;';
    return {
      read: [
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          `${vault}$c=$v.Retrieve('${SERVICE}','${ACCOUNT}');$c.RetrievePassword();$c.Password`,
        ],
      ],
      write: (t) => [
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          `${vault}$t=[Console]::In.ReadLine();$v.Add((New-Object Windows.Security.Credentials.PasswordCredential('${SERVICE}','${ACCOUNT}',$t)))`,
        ],
        `${t}\n`,
      ],
    };
  }
  return null;
}

/** The token: ESCOUADE_STORE_TOKEN, else the keychain; null if neither has one. */
export function readToken(env: NodeJS.ProcessEnv, platform = process.platform, exec: Run = run): string | null {
  const fromEnv = (env.ESCOUADE_STORE_TOKEN ?? '').trim();
  if (fromEnv) return fromEnv;
  const k = keychain(platform);
  if (!k) return null;
  const r = exec(k.read[0], k.read[1]);
  const t = r.status === 0 ? (r.stdout ?? '').trim() : '';
  return t.startsWith('esc_admin_') ? t : null;
}

/** Keeps `token` in the keychain; false if the system has none the script knows, or it refused. */
export function saveToken(token: string, platform = process.platform, exec: Run = run): boolean {
  const k = keychain(platform);
  if (!k) return false;
  const [cmd, args, input] = k.write(token);
  return exec(cmd, args, input).status === 0;
}

// ---- The store's review API ----

async function send(a: Admin, path: string, body: unknown): Promise<unknown> {
  const res = await (a.source.fetch ?? fetch)(`${a.source.store}/api/v1/review/${path}`, {
    method: 'POST',
    headers: { authorization: `Bearer ${a.token}`, 'content-type': 'application/json', 'user-agent': 'escouade-plugin-review' },
    body: JSON.stringify(body),
    redirect: 'error',
  });
  const text = await res.text();
  if (!res.ok) {
    let said = text;
    try {
      const j = JSON.parse(text) as { statusMessage?: string; message?: string };
      said = j.statusMessage ?? j.message ?? text;
    } catch {
      /* the text as it is */
    }
    throw new Error(`le store refuse (${res.status}) : ${said}`);
  }
  return JSON.parse(text);
}

/**
 * Runs minisign to sign `file` with the key of `role` and `trusted` as its comment; gives the `.minisig`. For a key with
 * a password, minisign asks it in this terminal, each time: nothing of it passes through the script, nothing keeps it.
 */
export function minisignSign(a: Admin, file: string, trusted: string, role: SigningRole): string {
  const key = keyFor(a, role);
  (a.log ?? ((l: string) => process.stderr.write(`${l}\n`)))(
    `Signature avec ${a.minisign.key ? 'la clé donnée par ESCOUADE_SIGNING_KEY' : KEY_NAMES[role]}${
      role === 'first-hand' && !a.minisign.key ? '' : ' : minisign demande son mot de passe'
    }.`,
  );
  const sig = `${file}.minisig`;
  const args = ['-S', '-s', key, '-m', file, '-x', sig, '-t', trusted];
  const js = /\.[cm]?js$/.test(a.minisign.cmd);
  // stdio inherited: minisign asks the key's password in this terminal; nothing of it passes through the script.
  const r = spawnSync(js ? process.execPath : a.minisign.cmd, js ? [a.minisign.cmd, ...args] : args, { stdio: 'inherit' });
  if (r.error) throw new Error(`minisign ne se lance pas (${a.minisign.cmd}) : ${r.error.message} ; installe-le (docs/STORE.md §4.5)`);
  if (r.status !== 0) throw new Error(`minisign a échoué (code ${r.status}) : rien n’est envoyé`);
  return readFileSync(sig, 'utf8');
}

/**
 * Makes the key of the third parties and of the revocation list on the user's machine (ESC-592): minisign makes the
 * pair, its password asked twice in this terminal (never `-W`, never an argument, never seen by this script); the
 * secret key goes to `<dir>/third-party.key`, the public one to `<keys>/third-party.pub` (`store/keys/`: public, it is
 * committed and the app embeds it). A key already there is never replaced; one minisign made without a password is
 * taken off. Gives its id and its public line, for the app's `STORE_KEYS`.
 */
export function createKey(o: { cmd: string; dir: string; keys: string }): { id: string; line: string; secret: string; pub: string } {
  const secret = join(o.dir, KEY_FILES['third-party']);
  const pub = join(o.keys, 'third-party.pub');
  for (const p of [secret, pub])
    if (existsSync(p)) throw new Error(`${p} existe déjà : rien n’est créé, une clé ne se remplace pas (docs/STORE.md §7)`);
  mkdirSync(o.dir, { recursive: true, mode: 0o700 });
  mkdirSync(o.keys, { recursive: true });
  const args = ['-G', '-p', pub, '-s', secret];
  const js = /\.[cm]?js$/.test(o.cmd);
  // stdio inherited: minisign asks the password twice in this terminal.
  const r = spawnSync(js ? process.execPath : o.cmd, js ? [o.cmd, ...args] : args, { stdio: 'inherit' });
  if (r.error) throw new Error(`minisign ne se lance pas (${o.cmd}) : ${r.error.message} ; installe-le (docs/STORE.md §4.5)`);
  if (r.status !== 0 || !existsSync(secret) || !existsSync(pub)) throw new Error(`minisign a échoué (code ${r.status}) : aucune clé créée`);
  if (locked(readFileSync(secret, 'utf8')) !== true) {
    rmSync(secret, { force: true });
    rmSync(pub, { force: true });
    throw new Error('minisign a fait une clé sans mot de passe : elle est retirée ; relance et tape un mot de passe');
  }
  const text = readFileSync(pub, 'utf8');
  const key = readPublicKey(text);
  if (!key) throw new Error(`${pub} ne se lit pas comme une clé publique de minisign`);
  const line = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('untrusted comment:'))[0]!;
  return { id: key.id, line, secret, pub };
}

/** The lines of a public summary (`- …`), without its title. */
export function summaryLines(text: string): string {
  return text
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => /^[-*]\s+/.test(l))
    .join('\n');
}

/** The review's data the script wrote for `id@version`, or an error that says what to run first. */
function reviewed(a: Admin, id: string, version: string): ReviewData {
  const path = join(a.reports, id, `${version}.json`);
  if (!existsSync(path))
    throw new Error(`pas de revue de ${id}@${version} dans ${a.reports} : lance d’abord npm run plugin-review -- ${id}@${version}`);
  return JSON.parse(readFileSync(path, 'utf8')) as ReviewData;
}

/**
 * Signs version `version` of `id` (docs/STORE.md §4.5): its `version.json` is downloaded again and must be the very
 * one the review read (the same SHA-256, the catalogue's); the review must have found nothing that keeps it from
 * « Vérifier » (integrity, tag, entries, attestations, bench). minisign signs it with the comment
 * `escouade-verified-v1 <id>@<version> <date>`; the signature is checked here with the public keys, then sent with the
 * public summary, and kept beside the report.
 */
export async function signVersion(
  a: Admin,
  id: string,
  version: string,
  now = new Date(),
): Promise<{ keyId: string; role: string; minisig: string }> {
  const data = reviewed(a, id, version);
  const stop = blockers(data);
  if (stop.length) throw new Error(`la revue a relevé ce qui empêche « Vérifier » : ${stop.join(' ; ')}`);
  if (!a.keys.length) throw new Error('aucune clé publique dans store/keys/ : la procédure de docs/STORE.md §4.5 les y met');
  const cat = await catalog(a.source);
  const entry = cat.plugins.find((p) => p.id === id)?.versions.find((v) => v.version === version);
  if (!entry) throw new Integrity(`${id}@${version} n’est pas au catalogue du store`);
  const res = await (a.source.fetch ?? fetch)(`${a.source.store}${entry.files.version_json}`, { redirect: 'error' });
  if (!res.ok) throw new Integrity(`version.json : ${res.status}`);
  const json = Buffer.from(await res.arrayBuffer());
  const got = sha256(json);
  if (got !== entry.sha256) throw new Integrity(`version.json : empreinte ${got}, le catalogue dit ${entry.sha256}`);
  if (got !== data.versionSha256)
    throw new Integrity(`version.json a changé depuis la revue (${data.versionSha256} relu, ${got} servi) : relance la revue`);
  // The signature covers version.json, so the module it names (ESC-324): signed only when the review compiled the bundle
  // again to that very module.
  const wasm = wasmOf(json.toString('utf8'));
  if (wasm && (data.recompile?.state !== 'same' || data.recompile.sha256 !== wasm.sha256))
    throw new Error(
      `le module WASM ${wasm.sha256} n’a pas été recompilé à la même empreinte par la revue : relance-la avec l’escouade-plugin ${'prelude' in wasm.compiler ? wasm.compiler.prelude : 'de cette version'}`,
    );

  const fr = join(a.reports, id, `${version}.public.md`);
  const en = join(a.reports, id, `${version}.public.en.md`);
  const summary =
    existsSync(fr) && existsSync(en) ? { fr: summaryLines(readFileSync(fr, 'utf8')), en: summaryLines(readFileSync(en, 'utf8')) } : null;

  mkdirSync(a.work, { recursive: true });
  const dir = mkdtempSync(join(a.work, `sign-${id}-${version}-`));
  try {
    const file = join(dir, 'version.json');
    writeFileSync(file, json);
    const minisig = minisignSign(a, file, verifiedComment(id, version, now.toISOString().slice(0, 10)), roleFor(id));
    const checked = checkVersionSignature(json, minisig, a.keys, id, version);
    if ('error' in checked)
      throw new Error(`la signature ne passe pas avec les clés publiques de store/keys/ (${checked.error}) : rien n’est envoyé`);
    await send(a, 'signature', { id, version, minisig, summary });
    writeFileSync(join(a.reports, id, `${version}.minisig`), minisig);
    return { keyId: checked.key.id, role: checked.role, minisig };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const FINDINGS = '## Constats de la revue';

/**
 * The verdict the reader settled in the report (« Verdict : … » in its findings), else the one the script proposed. Their
 * section is the report's last: a heading the same quoted from a plugin, above it, settles nothing.
 */
export function reportVerdict(report: string, data: ReviewData & { proposed?: { verdict: string | null } }): string | null {
  const settled = /^[-*\s]*\**verdict(?: final)?\**\s*:\s*\**\s*(Vérifier|Demander des changements|Refuser)\b/im.exec(
    report.includes(FINDINGS) ? report.slice(report.lastIndexOf(FINDINGS)) : '',
  );
  return settled?.[1] ?? data.proposed?.verdict ?? null;
}

/** Sends the report of `id@version` to the queue (the administrators alone read it there). */
export async function sendReport(a: Admin, id: string, version: string): Promise<{ verdict: string | null }> {
  const data = reviewed(a, id, version);
  const report = readFileSync(join(a.reports, id, `${version}.md`), 'utf8');
  const verdict = reportVerdict(report, data);
  await send(a, 'report', { id, version, verdict, report });
  return { verdict };
}

export interface Revoke {
  id: string;
  /** `*`, or the versions. */
  versions: '*' | string[];
  action: RevocationEntry['action'];
  reason: { fr: string; en: string };
  /** Lift what the list says of these versions instead (an entry taken out). */
  lift?: boolean;
  /** Key ids no longer to trust (§7), signed then by the other key. */
  revokeKeys?: string[];
  /** Tell the authors (false when it would help an attacker). */
  notify: boolean;
}

/** The list the store serves now, checked with the keys; null before the first. */
async function currentList(a: Admin): Promise<RevocationList | null> {
  const f = a.source.fetch ?? fetch;
  const json = await f(`${a.source.store}/api/v1/revocations.json`, { redirect: 'error' });
  if (json.status === 404) return null;
  if (!json.ok) throw new Error(`revocations.json : ${json.status}`);
  const sig = await f(`${a.source.store}/api/v1/revocations.json.minisig`, { redirect: 'error' });
  const text = await json.text();
  const checked = checkRevocations(text, sig.ok ? await sig.text() : '', a.keys);
  if ('error' in checked) throw new Integrity(`la liste que sert le store ne passe pas (${checked.error}) : rien n’est signé`);
  return checked.list;
}

/**
 * Signs and sends the next revocation list (docs/STORE.md §4.6): the store's list, checked, with this entry added (or
 * lifted), its serial one above the highest seen here or there (a list kept under `~/.escouade-review/revocations/`).
 */
export async function revoke(a: Admin, r: Revoke, now = new Date()): Promise<{ serial: number }> {
  if (!a.keys.length) throw new Error('aucune clé publique dans store/keys/ : la procédure de docs/STORE.md §4.5 les y met');
  if (!r.lift && !r.revokeKeys?.length && (!r.reason.fr.trim() || !r.reason.en.trim()))
    throw new Error('la raison, en français et en anglais (--fr, --en)');
  const list = await currentList(a);
  const kept = join(a.reports, 'revocations');
  mkdirSync(kept, { recursive: true });
  const seen = readdirSync(kept)
    .map((n) => /^(\d+)\.json$/.exec(n)?.[1])
    .filter(Boolean)
    .map(Number);
  const serial = Math.max(list?.serial ?? 0, ...seen) + 1;
  const date = now.toISOString().slice(0, 10);
  let entries = [...(list?.entries ?? [])];
  const same = (e: RevocationEntry) => e.id === r.id && e.action === r.action;
  if (r.lift) {
    entries = entries.flatMap((e) => {
      if (!same(e)) return [e];
      if (r.versions === '*' || e.versions === '*') return [];
      const left = e.versions.filter((v) => !(r.versions as string[]).includes(v));
      return left.length ? [{ ...e, versions: left }] : [];
    });
  } else if (r.id) entries.push({ id: r.id, versions: r.versions, action: r.action, reason: r.reason, date });
  const next: RevocationList = {
    serial,
    issued_at: now.toISOString(),
    entries,
    ...(list?.keys_revoked?.length || r.revokeKeys?.length
      ? { keys_revoked: [...new Set([...(list?.keys_revoked ?? []), ...(r.revokeKeys ?? [])])] }
      : {}),
  };
  const parsed = parseRevocations(JSON.stringify(next));
  if ('error' in parsed) throw new Error(`liste mal formée : ${parsed.error}`);
  const json = `${JSON.stringify(next, null, 2)}\n`;
  mkdirSync(a.work, { recursive: true });
  const dir = mkdtempSync(join(a.work, `revocations-${serial}-`));
  try {
    const file = join(dir, 'revocations.json');
    writeFileSync(file, json);
    const minisig = minisignSign(a, file, `${REVOCATIONS_COMMENT} ${serial} ${now.toISOString()}`, 'revocation');
    const checked = checkRevocations(json, minisig, a.keys, list?.serial ?? 0);
    if ('error' in checked) throw new Error(`la liste signée ne passe pas (${checked.error}) : rien n’est envoyé`);
    await send(a, 'revocations', { json, minisig, notify: r.notify });
    writeFileSync(join(kept, `${serial}.json`), json);
    writeFileSync(join(kept, `${serial}.json.minisig`), minisig);
    return { serial };
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

/** The work folder of the review (out of the repository). */
export const workDir = (env: NodeJS.ProcessEnv) => env.ESCOUADE_REVIEW_WORK ?? join(tmpdir(), 'escouade-review');

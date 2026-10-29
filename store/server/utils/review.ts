import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { and, desc, eq, inArray, isNotNull, isNull, sql } from 'drizzle-orm';
import { accounts, adminTokens, installsDaily, plugins, reports, signatures, versions } from '../db/schema';
import { note, type Account } from './accounts';
import type { Tell } from './author';
import { forgetCatalog } from './catalog';
import type { CheckResult, Text } from './checks';
import { admins, type Env } from './config';
import type { Db } from './db';
import { announce } from './discord';
import { localized, permissionsOf, type Permission } from './listing';
import { reviewMail, sendMail, type ReviewEvent } from './mail';
import type { PublicKey } from './minisign';
import { parseRevocations, revoked, type RevocationList } from './revocation-list';
import { reportClosedMail, type Reason } from './reports';
import { acceptRevocations, lastRevocations } from './revocations';
import { checkVersionSignature } from './signature';
import { signatureIndex, verifiedBy } from './signatures';
import { readArchive } from './tar';
import { randomToken, tokenHash } from './tokens';
import { compareVersions, type Binary } from './versions';
import { hueOf, markOf, type Icon } from '../../shared/tile';

// The review queue and what the review sends to the store (docs/STORE.md, §4.1, §4.3, §4.5, §4.6): for the
// administrators alone (their GitHub ids in STORE_ADMINS). The page /review reads the queue with the session and asks
// for changes, refuses, closes a report or lets a name too close pass; the review's script, on the user's machine,
// reads it with an administration token and sends a report, a signature or a revocation list. The store never signs:
// it checks a signature with the public keys before it takes it in, and serves it beside the version.

/** Whether the GitHub account `githubId` administers the store. */
export const isAdmin = (env: Env, githubId: number) => admins(env).includes(githubId);

// ---- The administration tokens ----

/** What every administration token starts with: a token pasted in the wrong place is recognized. */
export const ADMIN_TOKEN_PREFIX = 'esc_admin_';
/** Tokens an administrator may hold at once. */
export const ADMIN_TOKENS_MAX = 5;

export interface AdminTokenLine {
  ref: string;
  label: string;
  hint: string;
  createdAt: Date;
  usedAt: Date | null;
}

/** The reference of a token in the list: the start of its hash (never enough to use it). */
const refOf = (id: string) => id.slice(0, 16);

/** Makes a token for `account`: the token itself is given once, here; the base keeps its hash. */
export async function makeAdminToken(
  db: Db,
  account: Account,
  label: string,
  now = new Date(),
): Promise<{ token: string; ref: string } | { error: 'label' | 'too-many' }> {
  const name = label.replace(/\s+/g, ' ').trim();
  if (!name || name.length > 60) return { error: 'label' };
  const held = await adminTokensOf(db, account);
  if (held.length >= ADMIN_TOKENS_MAX) return { error: 'too-many' };
  const token = `${ADMIN_TOKEN_PREFIX}${randomToken()}`;
  const id = tokenHash(token);
  await db.insert(adminTokens).values({ id, accountId: account.id, label: name, hint: `${token.slice(0, 14)}…`, createdAt: now });
  await note(db, 'review.token_made', { accountId: account.id, githubId: account.githubId }, { ref: refOf(id), label: name }, now);
  return { token, ref: refOf(id) };
}

/** The tokens `account` holds and has not revoked, newest first. */
export async function adminTokensOf(db: Db, account: Account): Promise<AdminTokenLine[]> {
  const rows = await db
    .select()
    .from(adminTokens)
    .where(and(eq(adminTokens.accountId, account.id), isNull(adminTokens.revokedAt)))
    .orderBy(desc(adminTokens.createdAt));
  return rows.map((r) => ({ ref: refOf(r.id), label: r.label, hint: r.hint, createdAt: r.createdAt, usedAt: r.usedAt }));
}

/** Revokes the token `ref` of `account`; false if they hold none by that reference. */
export async function revokeAdminToken(db: Db, account: Account, ref: string, now = new Date()): Promise<boolean> {
  if (!/^[0-9a-f]{16}$/.test(ref)) return false;
  const done = await db
    .update(adminTokens)
    .set({ revokedAt: now })
    .where(and(eq(adminTokens.accountId, account.id), isNull(adminTokens.revokedAt), sql`left(${adminTokens.id}, 16) = ${ref}`))
    .returning({ id: adminTokens.id });
  if (done.length) await note(db, 'review.token_revoked', { accountId: account.id, githubId: account.githubId }, { ref }, now);
  return done.length > 0;
}

/**
 * The administrator a request's `Authorization: Bearer <token>` acts for, or null: a token made and not revoked, whose
 * account is still in STORE_ADMINS. Its last use is noted (to the minute).
 */
export async function tokenAdmin(db: Db, env: Env, header: string | undefined | null, now = new Date()): Promise<Account | null> {
  const m = /^Bearer\s+(\S+)$/i.exec(header ?? '');
  if (!m || !m[1]!.startsWith(ADMIN_TOKEN_PREFIX) || m[1]!.length > 200) return null;
  const id = tokenHash(m[1]!);
  const [row] = await db
    .select({ token: adminTokens, account: accounts })
    .from(adminTokens)
    .innerJoin(accounts, eq(accounts.id, adminTokens.accountId))
    .where(and(eq(adminTokens.id, id), isNull(adminTokens.revokedAt)));
  if (!row || !isAdmin(env, row.account.githubId)) return null;
  if (!row.token.usedAt || now.getTime() - row.token.usedAt.getTime() > 60_000)
    await db.update(adminTokens).set({ usedAt: now }).where(eq(adminTokens.id, id));
  return row.account;
}

// ---- The queue ----

export interface Grown {
  text: Text;
  mono?: string;
  icon: Permission['icon'];
  tone: Permission['tone'];
  /** Not asked by the version the review compares with (the last verified, else the one before). */
  isNew: boolean;
}

export interface QueueVersion {
  versionId: number;
  id: string;
  version: string;
  name: Text;
  mark: string;
  hue: number;
  icon: Icon | null;
  author: string | null;
  /** The version before it on the store, if any: an update; none: a first version. */
  previous: string | null;
  /** The last version verified below it, and when: the review reads the diff from it. */
  base: { version: string; signedAt: string } | null;
  installs: number;
  publishedAt: string;
  permissions: Grown[];
  diff: { added: number; removed: number; files: number; paths: string[] } | null;
  report: { verdict: string | null; at: string; text: string } | null;
  reports: number;
  /** The review's answer so far, for the versions done. */
  state: 'verified' | 'changes' | 'refused' | null;
  message: string | null;
  reviewedAt: string | null;
  command: string;
}

export interface QueueReport {
  reportId: number;
  id: string;
  name: Text;
  mark: string;
  hue: number;
  version: string | null;
  reason: Reason;
  details: string;
  createdAt: string;
  /** The reports still open on this plugin, and those that say it is malicious. */
  open: number;
  malicious: number;
}

export interface QueueHeld {
  versionId: number;
  id: string;
  version: string | null;
  name: Text;
  mark: string;
  hue: number;
  repository: string;
  author: string | null;
  why: Text;
  createdAt: string;
}

export interface Queue {
  toReview: QueueVersion[];
  reports: QueueReport[];
  held: QueueHeld[];
  done: QueueVersion[];
  counts: { toReview: number; reports: number; held: number; total: number };
  keys: number;
  revocations: { serial: number } | null;
}

/** The archive of a version, from the volume. */
const archiveOf = (dir: string, sha256: string) => readFile(join(dir, 'sha256', sha256.slice(0, 2), `${sha256}.tar.gz`));

const diffs = new Map<string, QueueVersion['diff']>();

/**
 * The size of the diff between two archives: the files added, removed or changed, and the lines added and removed
 * (each line counted as many times as it appears more, or less, than before). The script reads the real diff.
 */
export function diffSize(before: Map<string, Buffer>, after: Map<string, Buffer>): NonNullable<QueueVersion['diff']> {
  let added = 0;
  let removed = 0;
  const paths: string[] = [];
  for (const path of [...new Set([...before.keys(), ...after.keys()])].sort()) {
    const a = before.get(path);
    const b = after.get(path);
    if (a && b && a.equals(b)) continue;
    paths.push(path);
    const binary = (x?: Buffer) => !!x && x.subarray(0, 8000).includes(0);
    if (binary(a) || binary(b)) continue;
    const count = new Map<string, number>();
    for (const l of a ? a.toString('utf8').split('\n') : []) count.set(l, (count.get(l) ?? 0) - 1);
    for (const l of b ? b.toString('utf8').split('\n') : []) count.set(l, (count.get(l) ?? 0) + 1);
    for (const n of count.values())
      if (n > 0) added += n;
      else removed -= n;
  }
  return { added, removed, files: paths.length, paths };
}

async function diffOf(dir: string, before: string, after: string): Promise<QueueVersion['diff']> {
  const key = `${before}:${after}`;
  if (!diffs.has(key)) {
    try {
      const [a, b] = await Promise.all([archiveOf(dir, before), archiveOf(dir, after)]);
      diffs.set(key, diffSize(readArchive(a).files, readArchive(b).files));
    } catch (e) {
      console.error('[store] the review could not size a diff:', e instanceof Error ? e.message : e);
      return null;
    }
  }
  return diffs.get(key)!;
}

const permissionKey = (p: Permission) => `${p.icon}:${p.text.en}:${p.mono ?? ''}`;

/** The queue as the page and the script read it (docs/STORE.md §4.1): the most installed first, then the oldest. */
export async function reviewQueue(db: Db, dir: string, keys: readonly PublicKey[]): Promise<Queue> {
  const rows = await db
    .select({ v: versions, plugin: plugins, login: accounts.login })
    .from(versions)
    .innerJoin(plugins, eq(plugins.id, versions.pluginId))
    .leftJoin(accounts, eq(accounts.id, plugins.accountId))
    .where(and(eq(versions.state, 'published'), isNull(plugins.deletedAt)));
  const stored = await lastRevocations(db);
  const parsed = stored ? parseRevocations(stored.json) : null;
  const list: RevocationList | null = parsed && !('error' in parsed) ? parsed : null;
  const signed = await signatureIndex(db);
  const installed = new Map(
    (
      await db
        .select({ id: installsDaily.pluginId, n: sql<number>`sum(${installsDaily.count})::int` })
        .from(installsDaily)
        .where(eq(installsDaily.event, 'install'))
        .groupBy(installsDaily.pluginId)
    ).map((r) => [r.id, Number(r.n)]),
  );
  const open = await db.select().from(reports).where(eq(reports.state, 'open')).orderBy(reports.createdAt);
  const openBy = new Map<string, { open: number; malicious: number }>();
  for (const r of open) {
    const c = openBy.get(r.pluginId) ?? { open: 0, malicious: 0 };
    c.open++;
    if (r.reason === 'malicious') c.malicious++;
    openBy.set(r.pluginId, c);
  }

  const byPlugin = new Map<string, typeof rows>();
  for (const r of rows) byPlugin.set(r.plugin.id, [...(byPlugin.get(r.plugin.id) ?? []), r]);
  const verifiedOf = (r: (typeof rows)[number]) => verifiedBy(signed, list, r.plugin.id, r.v.version!, r.v.versionSha256);

  const line = async (r: (typeof rows)[number]): Promise<QueueVersion> => {
    const all = (byPlugin.get(r.plugin.id) ?? []).filter((x) => compareVersions(x.v.version!, r.v.version!) < 0);
    all.sort((a, b) => compareVersions(b.v.version!, a.v.version!));
    const previous = all[0] ?? null;
    const baseRow = all.find((x) => verifiedOf(x)) ?? null;
    const compared = baseRow ?? previous;
    const m = r.v.manifest ?? {};
    const had = new Set(compared ? permissionsOf(compared.v.manifest ?? {}, compared.v.binaries).map(permissionKey) : []);
    const name = localized(m.name);
    const sig = baseRow ? verifiedOf(baseRow) : null;
    return {
      versionId: r.v.id,
      id: r.plugin.id,
      version: r.v.version!,
      name,
      mark: markOf(name.en),
      hue: hueOf(r.plugin.id),
      icon: r.v.icon ?? null,
      author: r.login,
      previous: previous?.v.version ?? null,
      base: baseRow && sig ? { version: baseRow.v.version!, signedAt: sig.signedAt.toISOString() } : null,
      installs: installed.get(r.plugin.id) ?? 0,
      publishedAt: r.v.publishedAt!.toISOString(),
      permissions: permissionsOf(m, r.v.binaries).map((p) => ({ ...p, isNew: !!compared && !had.has(permissionKey(p)) })),
      diff: compared?.v.archiveSha256 && r.v.archiveSha256 ? await diffOf(dir, compared.v.archiveSha256, r.v.archiveSha256) : null,
      report: r.v.reviewReport ? { verdict: r.v.reviewVerdict, at: r.v.reviewReportAt!.toISOString(), text: r.v.reviewReport } : null,
      reports: openBy.get(r.plugin.id)?.open ?? 0,
      state: verifiedOf(r) ? 'verified' : r.v.reviewState === 'changes' || r.v.reviewState === 'refused' ? r.v.reviewState : null,
      message: r.v.reviewMessage,
      reviewedAt: r.v.reviewedAt?.toISOString() ?? null,
      command: `npm run plugin-review -- ${r.plugin.id}@${r.v.version}${baseRow && previous && baseRow !== previous ? ` --since ${baseRow.v.version}` : ''}`,
    };
  };

  const live = rows.filter((r) => !r.v.yankedAt && !revoked(list, r.plugin.id, r.v.version!, 'disable'));
  const waiting = live.filter((r) => r.v.review && !verifiedOf(r) && !r.v.reviewState);
  const toReview = await Promise.all(waiting.map(line));
  toReview.sort((a, b) => b.installs - a.installs || a.publishedAt.localeCompare(b.publishedAt));
  const answered = rows
    .filter((r) => r.v.reviewedAt && (r.v.reviewState === 'verified' || r.v.reviewState === 'changes' || r.v.reviewState === 'refused'))
    .sort((a, b) => b.v.reviewedAt!.getTime() - a.v.reviewedAt!.getTime())
    .slice(0, 50);
  const done = await Promise.all(answered.map(line));

  const names = new Map(rows.map((r) => [r.plugin.id, localized((r.v.manifest ?? {}).name)]));
  const queued = open.map((r): QueueReport => {
    const name = names.get(r.pluginId) ?? { fr: r.pluginId, en: r.pluginId };
    const c = openBy.get(r.pluginId)!;
    return {
      reportId: r.id,
      id: r.pluginId,
      name,
      mark: markOf(name.en),
      hue: hueOf(r.pluginId),
      version: r.version,
      reason: r.reason as Reason,
      details: r.details,
      createdAt: r.createdAt.toISOString(),
      open: c.open,
      malicious: c.malicious,
    };
  });
  // A plugin several say is malicious comes first (§4.6); the decision stays a person's.
  queued.sort((a, b) => Number(b.malicious > 1) - Number(a.malicious > 1) || a.createdAt.localeCompare(b.createdAt));

  const heldRows = await db
    .select({ v: versions, login: accounts.login })
    .from(versions)
    .leftJoin(accounts, eq(accounts.id, versions.accountId))
    .where(eq(versions.state, 'held'))
    .orderBy(versions.createdAt);
  const held = heldRows.map(({ v, login }): QueueHeld => {
    const name = localized((v.manifest ?? {}).name);
    const hold = (v.checks as CheckResult[]).find((c) => c?.status === 'hold');
    return {
      versionId: v.id,
      id: v.pluginId ?? '',
      version: v.version,
      name,
      mark: markOf(name.en || v.pluginId || '?'),
      hue: hueOf(v.pluginId ?? ''),
      repository: v.repoFullName,
      author: login,
      why: hold?.detail ?? { fr: 'Revue manuelle', en: 'Manual review' },
      createdAt: v.createdAt.toISOString(),
    };
  });

  return {
    toReview,
    reports: queued,
    held,
    done,
    counts: {
      toReview: toReview.length,
      reports: queued.length,
      held: held.length,
      total: toReview.length + queued.length + held.length,
    },
    keys: keys.length,
    revocations: stored ? { serial: stored.serial } : null,
  };
}

/** How many wait for the review: the number beside « File de revue » in the header. */
export async function queueCount(db: Db): Promise<number> {
  const [a] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(versions)
    .innerJoin(plugins, eq(plugins.id, versions.pluginId))
    .where(
      and(
        eq(versions.state, 'published'),
        eq(versions.review, true),
        isNull(versions.reviewState),
        isNull(versions.yankedAt),
        isNull(plugins.deletedAt),
      ),
    );
  const [b] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(reports)
    .where(eq(reports.state, 'open'));
  const [c] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(versions)
    .where(eq(versions.state, 'held'));
  return Number(a?.n ?? 0) + Number(b?.n ?? 0) + Number(c?.n ?? 0);
}

// ---- What the review decides ----

async function tellAuthor(
  db: Db,
  accountId: number | null,
  how: Tell | null,
  event: ReviewEvent,
  w: { plugin: string; version: string; message?: string | Text },
) {
  if (!accountId || !how) return;
  const [a] = await db.select({ email: accounts.email, at: accounts.emailVerifiedAt }).from(accounts).where(eq(accounts.id, accountId));
  if (a?.email && a.at)
    await sendMail(how.env, reviewMail(a.email, event, { ...w, page: `${how.site}/account?plugin=${encodeURIComponent(w.plugin)}` }));
}

const MESSAGE_MIN = 10;
const MESSAGE_MAX = 4000;
const clean = (s: unknown) =>
  String(s ?? '')
    .replace(/\r\n?/g, '\n')
    .replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, '')
    .trim();

/**
 * Asks changes of a version in the queue, or refuses it (docs/STORE.md §4.3): it stays published, unverified; its
 * author is told why, by e-mail and in their account. A version already verified is not touched.
 */
export async function decide(
  db: Db,
  admin: Account,
  versionId: number,
  answer: 'changes' | 'refused',
  message: unknown,
  how: Tell | null,
  now = new Date(),
): Promise<{ ok: true } | { error: 'not-found' | 'message' | 'verified' }> {
  const text = clean(message);
  if (text.length < MESSAGE_MIN || text.length > MESSAGE_MAX) return { error: 'message' };
  const [v] = await db
    .select()
    .from(versions)
    .where(and(eq(versions.id, versionId), eq(versions.state, 'published'), isNotNull(versions.pluginId)));
  if (!v) return { error: 'not-found' };
  if (v.reviewState === 'verified') return { error: 'verified' };
  await db.update(versions).set({ reviewState: answer, reviewMessage: text, reviewedAt: now }).where(eq(versions.id, v.id));
  await note(db, `review.${answer}`, { accountId: admin.id, githubId: admin.githubId }, { plugin: v.pluginId, version: v.version }, now);
  const [p] = await db.select({ accountId: plugins.accountId }).from(plugins).where(eq(plugins.id, v.pluginId!));
  await tellAuthor(db, p?.accountId ?? null, how, answer, { plugin: v.pluginId!, version: v.version!, message: text });
  return { ok: true };
}

/** A version held for a name too close (§3.3): let it pass (it waits for its author's click), or refuse it. */
export async function settleHeld(
  db: Db,
  admin: Account,
  versionId: number,
  answer: 'release' | 'refuse',
  message: unknown,
  how: Tell | null,
  now = new Date(),
): Promise<{ ok: true } | { error: 'not-found' | 'message' }> {
  const text = clean(message);
  if (answer === 'refuse' && (text.length < MESSAGE_MIN || text.length > MESSAGE_MAX)) return { error: 'message' };
  const done = await db
    .update(versions)
    .set(
      answer === 'release'
        ? { state: 'ready', reviewedAt: now }
        : { state: 'rejected', reviewState: 'refused', reviewMessage: text, reviewedAt: now },
    )
    .where(and(eq(versions.id, versionId), eq(versions.state, 'held')))
    .returning();
  const v = done[0];
  if (!v) return { error: 'not-found' };
  await note(
    db,
    `review.held_${answer}`,
    { accountId: admin.id, githubId: admin.githubId },
    { plugin: v.pluginId, version: v.version },
    now,
  );
  await tellAuthor(db, v.accountId, how, answer === 'release' ? 'released' : 'held-refused', {
    plugin: v.pluginId ?? v.repoFullName,
    version: v.version ?? v.tag,
    message: text,
  });
  return { ok: true };
}

/** Closes a report (§4.6): `upheld` (the review acts: a revocation, changes asked) or `dismissed`; its sender is answered. */
export async function closeReport(
  db: Db,
  admin: Account,
  reportId: number,
  answer: 'upheld' | 'dismissed',
  words: unknown,
  env: Env | null,
  now = new Date(),
): Promise<{ ok: true } | { error: 'not-found' | 'message' }> {
  const text = clean(words);
  if (text.length > MESSAGE_MAX) return { error: 'message' };
  const done = await db
    .update(reports)
    .set({ state: answer, closedAt: now, answer: text || null })
    .where(and(eq(reports.id, reportId), eq(reports.state, 'open')))
    .returning();
  const r = done[0];
  if (!r) return { error: 'not-found' };
  await note(db, `review.report_${answer}`, { accountId: admin.id, githubId: admin.githubId }, { report: r.id, plugin: r.pluginId }, now);
  if (env)
    await sendMail(
      env,
      reportClosedMail(r.lang === 'en' ? 'en' : 'fr', r.email, { id: r.id, plugin: r.pluginId, upheld: answer === 'upheld', answer: text }),
    );
  return { ok: true };
}

// ---- What the script sends ----

const VERDICTS = ['Vérifier', 'Demander des changements', 'Refuser'] as const;
/** The largest report the store keeps: a long reading, not an archive. */
export const REPORT_MAX = 256 * 1024;
const SUMMARY_MAX = 2000;

/** Keeps the report of the review of a version (markdown, for the administrators alone) and the verdict it proposes. */
export async function postReport(
  db: Db,
  admin: Account,
  input: { id?: unknown; version?: unknown; verdict?: unknown; report?: unknown },
  now = new Date(),
): Promise<{ ok: true } | { error: 'not-found' | 'report' | 'verdict' }> {
  const report = typeof input.report === 'string' ? input.report.replace(/\r\n?/g, '\n') : '';
  if (!report.trim() || Buffer.byteLength(report) > REPORT_MAX) return { error: 'report' };
  const verdict = input.verdict === null || input.verdict === undefined ? null : String(input.verdict);
  if (verdict !== null && !(VERDICTS as readonly string[]).includes(verdict)) return { error: 'verdict' };
  const done = await db
    .update(versions)
    .set({ reviewReport: report, reviewVerdict: verdict, reviewReportAt: now })
    .where(
      and(
        eq(versions.pluginId, String(input.id ?? '')),
        eq(versions.version, String(input.version ?? '')),
        eq(versions.state, 'published'),
      ),
    )
    .returning({ id: versions.id });
  if (!done.length) return { error: 'not-found' };
  await note(
    db,
    'review.report',
    { accountId: admin.id, githubId: admin.githubId },
    { plugin: input.id, version: input.version, verdict },
    now,
  );
  return { ok: true };
}

/** The public summary sent with a signature: plain lines, in both languages, short. */
function summaryOf(v: unknown): { fr: string; en: string } | null | 'bad' {
  if (v === undefined || v === null) return null;
  if (typeof v !== 'object' || Array.isArray(v)) return 'bad';
  const o = v as Record<string, unknown>;
  const fr = clean(o.fr);
  const en = clean(o.en);
  if (!fr || !en || fr.length > SUMMARY_MAX || en.length > SUMMARY_MAX) return 'bad';
  return { fr, en };
}

export type Signing =
  | { ok: true; keyId: string; signedAt: string }
  | { error: 'not-found' | 'summary' | 'unattested' | 'disabled' | 'no-keys' | 'signature'; detail?: string };

/**
 * Takes in the review's signature of a version (docs/STORE.md §4.5), made on the user's machine: checked with the
 * store's public keys against the version's `version.json`, its trusted comment naming this id and this number, by a
 * key the revocation list does not revoke. A version with a binary that has no provenance attestation is not verified
 * (§4.4). Taken in, it makes the version verified in the catalogue and on its page, and its author is told.
 */
export async function acceptSignature(
  db: Db,
  admin: Account,
  keys: readonly PublicKey[],
  input: { id?: unknown; version?: unknown; minisig?: unknown; summary?: unknown },
  how: Tell | null,
  now = new Date(),
): Promise<Signing> {
  const id = String(input.id ?? '');
  const number = String(input.version ?? '');
  const [v] = await db
    .select()
    .from(versions)
    .where(and(eq(versions.pluginId, id), eq(versions.version, number), eq(versions.state, 'published')));
  if (!v?.versionJson || !v.versionSha256) return { error: 'not-found' };
  const summary = summaryOf(input.summary);
  if (summary === 'bad') return { error: 'summary' };
  const unattested = Object.entries((v.binaries ?? {}) as Record<string, Binary>).filter(([, b]) => !b.attested);
  if (unattested.length) return { error: 'unattested', detail: unattested.map(([p]) => p).join(', ') };
  const stored = await lastRevocations(db);
  const parsed = stored ? parseRevocations(stored.json) : null;
  const list = parsed && !('error' in parsed) ? parsed : null;
  if (revoked(list, id, number, 'disable')) return { error: 'disabled' };
  if (!keys.length) return { error: 'no-keys' };
  const checked = checkVersionSignature(v.versionJson, String(input.minisig ?? ''), keys, id, number, list?.keys_revoked ?? []);
  if ('error' in checked) return { error: 'signature', detail: checked.error };
  const row = {
    pluginId: id,
    version: number,
    versionSha256: v.versionSha256,
    minisig: String(input.minisig),
    keyId: checked.key.id,
    signedAt: new Date(checked.date),
    receivedAt: now,
    summary,
  };
  await db
    .insert(signatures)
    .values(row)
    .onConflictDoUpdate({ target: [signatures.pluginId, signatures.version], set: row });
  await db.update(versions).set({ reviewState: 'verified', reviewMessage: null, reviewedAt: now }).where(eq(versions.id, v.id));
  forgetCatalog();
  await note(
    db,
    'review.signed',
    { accountId: admin.id, githubId: admin.githubId },
    { plugin: id, version: number, key: checked.key.id },
    now,
  );
  const [p] = await db.select({ accountId: plugins.accountId }).from(plugins).where(eq(plugins.id, id));
  await tellAuthor(db, p?.accountId ?? null, how, 'verified', { plugin: id, version: number });
  void announce({ kind: 'verified', plugin: id, version: number });
  return { ok: true, keyId: checked.key.id, signedAt: row.signedAt.toISOString() };
}

/**
 * Takes in a revocation list signed on the user's machine (§4.6): checked by revocations.ts, then the catalogue is
 * built again. The authors of the versions it newly disables or unverifies are told, unless `notify` is false (when
 * telling them would help an attacker).
 */
export async function depositRevocations(
  db: Db,
  admin: Account,
  keys: readonly PublicKey[],
  input: { json?: unknown; minisig?: unknown; notify?: unknown },
  how: Tell | null,
  now = new Date(),
): Promise<{ serial: number } | { error: string }> {
  if (typeof input.json !== 'string' || typeof input.minisig !== 'string') return { error: 'json and minisig, as text' };
  if (!keys.length) return { error: 'the store has no public key yet (store/keys/*.pub)' };
  const before = await lastRevocations(db);
  const old = before ? parseRevocations(before.json) : null;
  const was: RevocationList | null = old && !('error' in old) ? old : null;
  const taken = await acceptRevocations(db, keys, input.json, input.minisig, now);
  if ('error' in taken) return taken;
  forgetCatalog();
  await note(db, 'review.revocations', { accountId: admin.id, githubId: admin.githubId }, { serial: taken.serial }, now);
  const list = parseRevocations(input.json) as RevocationList;
  // Discord: each new entry to the administrators; a new removal to the public too, unless the author is not told.
  const known = new Set((was?.entries ?? []).map((e) => JSON.stringify(e)));
  for (const entry of list.entries.filter((e) => !known.has(JSON.stringify(e)))) {
    void announce({ kind: 'revoked', serial: taken.serial, entry, quiet: input.notify === false });
    if (entry.action === 'disable' && input.notify !== false) void announce({ kind: 'removed', entry });
  }
  if (input.notify === false) return taken;
  const ids = [...new Set(list.entries.map((e) => e.id))];
  if (!ids.length) return taken;
  const rows = await db
    .select({ id: versions.pluginId, version: versions.version, accountId: plugins.accountId })
    .from(versions)
    .innerJoin(plugins, eq(plugins.id, versions.pluginId))
    .where(and(inArray(versions.pluginId, ids), eq(versions.state, 'published')));
  for (const r of rows)
    for (const action of ['disable', 'unverify'] as const) {
      const entry = revoked(list, r.id!, r.version!, action);
      if (!entry || revoked(was, r.id!, r.version!, action)) continue;
      await tellAuthor(db, r.accountId, how, action === 'disable' ? 'disabled' : 'unverified', {
        plugin: r.id!,
        version: r.version!,
        message: entry.reason,
      });
    }
  return taken;
}

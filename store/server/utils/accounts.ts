import { and, desc, eq, gt, gte, inArray, isNotNull, lt, sql } from 'drizzle-orm';
import {
  accountInstallations,
  accounts,
  audit,
  emailTokens,
  installationRepos,
  installations,
  sessions,
  webhookDeliveries,
} from '../db/schema';
import type { Db } from './db';
import type { GithubInstallation, GithubRepo, GithubUser, UserToken } from './github';
import { randomToken, seal, tokenHash, unseal } from './tokens';

// The accounts of the store (docs/STORE.md, §2): an author signs in with GitHub, keeps a session 30 days, may give an
// e-mail address checked by a link, export what the store keeps of them, and delete it all. Every step lands in the
// journal of actions, kept one year.

const DAY = 86_400_000;
export const SESSION_DAYS = 30;
/** The user's GitHub token is kept 8 hours at most, whatever GitHub says (docs/STORE.md, §2.3). */
export const GITHUB_TOKEN_HOURS = 8;
export const EMAIL_LINK_HOURS = 24;
/** E-mail links an account may ask for in an hour: enough for a typo, too few to send mail to strangers. */
export const EMAIL_LINKS_PER_HOUR = 5;
export const AUDIT_DAYS = 365;
export const DELIVERY_DAYS = 30;

export type Account = typeof accounts.$inferSelect;

/** Notes an action in the journal. */
export async function note(
  db: Db,
  action: string,
  who: { accountId?: number | null; githubId?: number | null },
  detail?: Record<string, unknown>,
  at = new Date(),
): Promise<void> {
  await db.insert(audit).values({ at, action, accountId: who.accountId ?? null, githubId: who.githubId ?? null, detail: detail ?? null });
}

/** Keeps what GitHub says of an installation and of the repositories it reaches (all of them: the list is GitHub's). */
async function keepInstallation(db: Db, i: GithubInstallation, repos: GithubRepo[] | null, now: Date): Promise<void> {
  const row = {
    ownerId: i.account.id,
    ownerLogin: i.account.login,
    ownerType: i.account.type,
    repositorySelection: i.repository_selection ?? 'selected',
    suspendedAt: i.suspended_at ? new Date(i.suspended_at) : null,
    updatedAt: now,
  };
  await db
    .insert(installations)
    .values({ id: i.id, ...row })
    .onConflictDoUpdate({ target: installations.id, set: row });
  if (repos) {
    await db.delete(installationRepos).where(eq(installationRepos.installationId, i.id));
    if (repos.length)
      await db
        .insert(installationRepos)
        .values(repos.map((r) => ({ installationId: i.id, repoId: r.id, fullName: r.full_name, private: r.private })));
  }
}

export interface SignedIn {
  account: Account;
  /** The session's token, for the cookie; the base keeps its hash only. */
  session: string;
}

/**
 * Signs in the GitHub user `user`: their account made or brought up to date (login, name and avatar are GitHub's),
 * a new session of 30 days holding their token sealed, and the App's installations they reach.
 */
export async function signIn(
  db: Db,
  input: { user: GithubUser; token: UserToken; installations: { installation: GithubInstallation; repos: GithubRepo[] }[] },
  secret: string,
  now = new Date(),
): Promise<SignedIn> {
  const { user, token } = input;
  const who = { login: user.login, name: user.name, avatarUrl: user.avatar_url, seenAt: now };
  const [account] = await db
    .insert(accounts)
    .values({ githubId: user.id, ...who, createdAt: now })
    .onConflictDoUpdate({ target: accounts.githubId, set: who })
    .returning();
  const cap = now.getTime() + GITHUB_TOKEN_HOURS * 3600_000;
  const tokenEnds = new Date(Math.min(token.expiresAt?.getTime() ?? cap, cap));
  const session = randomToken();
  await db.insert(sessions).values({
    id: tokenHash(session),
    accountId: account!.id,
    createdAt: now,
    expiresAt: new Date(now.getTime() + SESSION_DAYS * DAY),
    githubToken: seal(secret, token.token),
    githubTokenExpiresAt: tokenEnds,
  });
  for (const { installation, repos } of input.installations) await keepInstallation(db, installation, repos, now);
  await db.delete(accountInstallations).where(eq(accountInstallations.accountId, account!.id));
  if (input.installations.length)
    await db
      .insert(accountInstallations)
      .values(input.installations.map(({ installation }) => ({ accountId: account!.id, installationId: installation.id })));
  await note(db, 'account.signed_in', { accountId: account!.id, githubId: user.id }, { installations: input.installations.length }, now);
  return { account: account!, session };
}

/** The account a session's token opens, if the session has not ended. */
export async function sessionAccount(db: Db, session: string | undefined, now = new Date()): Promise<Account | null> {
  if (!session) return null;
  const rows = await db
    .select({ account: accounts })
    .from(sessions)
    .innerJoin(accounts, eq(accounts.id, sessions.accountId))
    .where(and(eq(sessions.id, tokenHash(session)), gt(sessions.expiresAt, now)));
  return rows[0]?.account ?? null;
}

/** Ends a session; gives back the GitHub token it held, still valid, for GitHub to revoke it. */
export async function signOut(db: Db, session: string, secret: string, now = new Date()): Promise<string | null> {
  const [gone] = await db
    .delete(sessions)
    .where(eq(sessions.id, tokenHash(session)))
    .returning();
  if (!gone) return null;
  const account = await db.select({ githubId: accounts.githubId }).from(accounts).where(eq(accounts.id, gone.accountId));
  await note(db, 'account.signed_out', { accountId: gone.accountId, githubId: account[0]?.githubId }, undefined, now);
  return gone.githubToken && gone.githubTokenExpiresAt && gone.githubTokenExpiresAt > now ? unseal(secret, gone.githubToken) : null;
}

/** An address the store would write to: one @, a dotted domain, no space, 254 characters at most. */
export function cleanEmail(raw: unknown): string | null {
  if (typeof raw !== 'string') return null;
  const email = raw.trim();
  if (email.length > 254 || !/^[^\s@<>()",;:\\]+@[^\s@<>()",;:\\]+\.[^\s@<>()",;:\\.]{2,}$/.test(email)) return null;
  const at = email.lastIndexOf('@');
  return `${email.slice(0, at)}@${email.slice(at + 1).toLowerCase()}`;
}

export type EmailAsk = { token: string; email: string } | { error: 'invalid' | 'too-many' | 'same' };

/** Asks for a link that checks `raw`; the address is kept only once the link is followed. */
export async function askEmail(db: Db, account: Account, raw: unknown, now = new Date()): Promise<EmailAsk> {
  const email = cleanEmail(raw);
  if (!email) return { error: 'invalid' };
  if (account.email === email && account.emailVerifiedAt) return { error: 'same' };
  const [recent] = await db
    .select({ n: sql<number>`count(*)::int` })
    .from(emailTokens)
    .where(and(eq(emailTokens.accountId, account.id), gte(emailTokens.createdAt, new Date(now.getTime() - 3600_000))));
  if ((recent?.n ?? 0) >= EMAIL_LINKS_PER_HOUR) return { error: 'too-many' };
  const token = randomToken();
  await db.insert(emailTokens).values({
    id: tokenHash(token),
    accountId: account.id,
    email,
    createdAt: now,
    expiresAt: new Date(now.getTime() + EMAIL_LINK_HOURS * 3600_000),
  });
  await note(db, 'account.email_asked', { accountId: account.id, githubId: account.githubId }, undefined, now);
  return { token, email };
}

/** Follows an e-mail link: its address becomes the account's, checked; the link and the others pending go. */
export async function verifyEmail(db: Db, token: string, now = new Date()): Promise<Account | null> {
  const [link] = await db
    .delete(emailTokens)
    .where(and(eq(emailTokens.id, tokenHash(token)), gt(emailTokens.expiresAt, now)))
    .returning();
  if (!link) return null;
  await db.delete(emailTokens).where(eq(emailTokens.accountId, link.accountId));
  const [account] = await db
    .update(accounts)
    .set({ email: link.email, emailVerifiedAt: now })
    .where(eq(accounts.id, link.accountId))
    .returning();
  if (account) await note(db, 'account.email_verified', { accountId: account.id, githubId: account.githubId }, undefined, now);
  return account ?? null;
}

/** Forgets the account's address and the links pending: the notices stay on the account's page. */
export async function removeEmail(db: Db, account: Account, now = new Date()): Promise<void> {
  await db.update(accounts).set({ email: null, emailVerifiedAt: null }).where(eq(accounts.id, account.id));
  await db.delete(emailTokens).where(eq(emailTokens.accountId, account.id));
  await note(db, 'account.email_removed', { accountId: account.id, githubId: account.githubId }, undefined, now);
}

/** The address waiting for its link to be followed, if any. */
async function pendingEmail(db: Db, accountId: number, now: Date): Promise<string | null> {
  const [row] = await db
    .select({ email: emailTokens.email })
    .from(emailTokens)
    .where(and(eq(emailTokens.accountId, accountId), gt(emailTokens.expiresAt, now)))
    .orderBy(desc(emailTokens.createdAt))
    .limit(1);
  return row?.email ?? null;
}

export interface InstallationView {
  id: number;
  owner: string;
  ownerType: string;
  all: boolean;
  suspended: boolean;
  repos: string[];
}

/** The App's installations the account manages, with their repositories. */
export async function accountInstallationsOf(db: Db, accountId: number): Promise<InstallationView[]> {
  const rows = await db
    .select({ i: installations })
    .from(accountInstallations)
    .innerJoin(installations, eq(installations.id, accountInstallations.installationId))
    .where(eq(accountInstallations.accountId, accountId))
    .orderBy(installations.ownerLogin);
  const ids = rows.map((r) => r.i.id);
  const repos = ids.length
    ? await db.select().from(installationRepos).where(inArray(installationRepos.installationId, ids)).orderBy(installationRepos.fullName)
    : [];
  return rows.map(({ i }) => ({
    id: i.id,
    owner: i.ownerLogin,
    ownerType: i.ownerType,
    all: i.repositorySelection === 'all',
    suspended: i.suspendedAt !== null,
    repos: repos.filter((r) => r.installationId === i.id).map((r) => r.fullName),
  }));
}

/** What the account's page shows of it. */
export async function accountView(db: Db, account: Account, now = new Date()) {
  return {
    login: account.login,
    name: account.name,
    githubId: account.githubId,
    email: account.emailVerifiedAt ? account.email : null,
    pendingEmail: await pendingEmail(db, account.id, now),
    installations: await accountInstallationsOf(db, account.id),
  };
}

/** Everything the store keeps of an account, for its export (RGPD, art. 15 and 20); no token, not even hashed. */
export async function exportAccount(db: Db, account: Account, now = new Date()) {
  const own = await db
    .select({ createdAt: sessions.createdAt, expiresAt: sessions.expiresAt, githubTokenUntil: sessions.githubTokenExpiresAt })
    .from(sessions)
    .where(eq(sessions.accountId, account.id))
    .orderBy(sessions.createdAt);
  const journal = await db
    .select({ at: audit.at, action: audit.action, detail: audit.detail })
    .from(audit)
    .where(eq(audit.accountId, account.id))
    .orderBy(audit.at, audit.id);
  await note(db, 'account.exported', { accountId: account.id, githubId: account.githubId }, undefined, now);
  return {
    exportedAt: now.toISOString(),
    store: 'https://store.escouade.dev',
    account: {
      provider: account.provider,
      githubId: account.githubId,
      login: account.login,
      name: account.name,
      avatarUrl: account.avatarUrl,
      email: account.email,
      emailVerifiedAt: account.emailVerifiedAt,
      pendingEmail: await pendingEmail(db, account.id, now),
      createdAt: account.createdAt,
      seenAt: account.seenAt,
    },
    sessions: own,
    installations: await accountInstallationsOf(db, account.id),
    journal,
  };
}

/**
 * Deletes the account: its sessions, its address and its links go with it; the journal keeps its entries one year,
 * by GitHub id, as it does for every action. Gives back the GitHub tokens still valid, for GitHub to revoke them.
 * (STORE-3 adds the plugins, which an author transfers or deletes first, docs/STORE.md §2.2.)
 */
export async function deleteAccount(db: Db, account: Account, secret: string, now = new Date()): Promise<string[]> {
  const open = await db
    .select({ token: sessions.githubToken })
    .from(sessions)
    .where(and(eq(sessions.accountId, account.id), isNotNull(sessions.githubToken), gt(sessions.githubTokenExpiresAt, now)));
  await db.delete(accounts).where(eq(accounts.id, account.id));
  await note(db, 'account.deleted', { githubId: account.githubId }, { login: account.login }, now);
  return open.flatMap((s) => {
    const t = s.token ? unseal(secret, s.token) : null;
    return t ? [t] : [];
  });
}

/** Forgets what has outlived its time: sessions (30 days), GitHub tokens (8 h), links (24 h), deliveries (30 days), journal (1 year). */
export async function purge(db: Db, now = new Date()) {
  const ago = (days: number) => new Date(now.getTime() - days * DAY);
  const s = await db.delete(sessions).where(lt(sessions.expiresAt, now)).returning({ id: sessions.id });
  await db
    .update(sessions)
    .set({ githubToken: null, githubTokenExpiresAt: null })
    .where(and(isNotNull(sessions.githubToken), lt(sessions.githubTokenExpiresAt, now)));
  const e = await db.delete(emailTokens).where(lt(emailTokens.expiresAt, now)).returning({ id: emailTokens.id });
  const d = await db
    .delete(webhookDeliveries)
    .where(lt(webhookDeliveries.receivedAt, ago(DELIVERY_DAYS)))
    .returning({ id: webhookDeliveries.id });
  const a = await db
    .delete(audit)
    .where(lt(audit.at, ago(AUDIT_DAYS)))
    .returning({ id: audit.id });
  return { sessions: s.length, emailLinks: e.length, deliveries: d.length, journal: a.length };
}

// ---- The App's webhooks ----

interface Payload {
  action?: string;
  sender?: { id?: number };
  installation?: GithubInstallation;
  repositories?: GithubRepo[];
  repositories_added?: GithubRepo[];
  repositories_removed?: { id: number }[];
  repository?: GithubRepo & { owner?: { login?: string } };
  changes?: Record<string, unknown>;
  ref?: string;
  ref_type?: string;
  release?: { tag_name?: string };
}

export type Handled = 'handled' | 'duplicate' | 'ignored';

/**
 * Applies a webhook the App received, once per delivery id: the installations and their repositories follow what
 * GitHub says, and the journal notes it. The tags (`create`, `release`) are noted for now: the submission reads them
 * (STORE-3).
 */
export async function applyWebhook(db: Db, event: string, delivery: string, p: Payload, now = new Date()): Promise<Handled> {
  const fresh = await db
    .insert(webhookDeliveries)
    .values({ id: delivery, event, action: p.action ?? null, receivedAt: now })
    .onConflictDoNothing()
    .returning({ id: webhookDeliveries.id });
  if (!fresh.length) return 'duplicate';
  const who = { githubId: p.sender?.id ?? null };
  const i = p.installation;

  if (event === 'installation' && i) {
    if (p.action === 'deleted') {
      await db.delete(installations).where(eq(installations.id, i.id));
    } else {
      if (p.action === 'suspend') i.suspended_at ??= now.toISOString();
      if (p.action === 'unsuspend') i.suspended_at = null;
      await keepInstallation(db, i, p.action === 'created' ? (p.repositories ?? []) : null, now);
    }
    await note(db, `installation.${p.action}`, who, { installation: i.id, owner: i.account?.login }, now);
    return 'handled';
  }
  if (event === 'installation_repositories' && i) {
    await keepInstallation(db, i, null, now);
    const added = p.repositories_added ?? [];
    const removed = (p.repositories_removed ?? []).map((r) => r.id);
    if (added.length)
      await db
        .insert(installationRepos)
        .values(added.map((r) => ({ installationId: i.id, repoId: r.id, fullName: r.full_name, private: r.private })))
        .onConflictDoUpdate({
          target: [installationRepos.installationId, installationRepos.repoId],
          set: { fullName: sql`excluded.full_name`, private: sql`excluded.private` },
        });
    if (removed.length)
      await db.delete(installationRepos).where(and(eq(installationRepos.installationId, i.id), inArray(installationRepos.repoId, removed)));
    await note(
      db,
      `installation.repositories_${p.action}`,
      who,
      { installation: i.id, added: added.map((r) => r.full_name), removed },
      now,
    );
    return 'handled';
  }
  if (event === 'repository' && p.repository) {
    const r = p.repository;
    if (p.action === 'deleted' || p.action === 'transferred' || p.action === 'privatized') {
      // A repository gone, moved away or made private no longer reaches the store through this installation;
      // a transfer comes back by the new owner's installation (docs/STORE.md, §2.2).
      await db.delete(installationRepos).where(eq(installationRepos.repoId, r.id));
    } else if (p.action === 'renamed' || p.action === 'publicized') {
      await db.update(installationRepos).set({ fullName: r.full_name, private: r.private }).where(eq(installationRepos.repoId, r.id));
    }
    await note(db, `repository.${p.action}`, who, { repository: r.full_name, id: r.id }, now);
    return 'handled';
  }
  if ((event === 'create' && p.ref_type === 'tag') || event === 'release') {
    await note(db, `${event}.${p.action ?? 'tag'}`, who, { repository: p.repository?.full_name, tag: p.release?.tag_name ?? p.ref }, now);
    return 'handled';
  }
  return 'ignored';
}

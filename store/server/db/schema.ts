import { sql } from 'drizzle-orm';
import {
  bigint,
  bigserial,
  date,
  boolean,
  index,
  integer,
  jsonb,
  pgTable,
  primaryKey,
  serial,
  text,
  timestamp,
  uniqueIndex,
} from 'drizzle-orm/pg-core';

// The store's tables. A migration only adds (tables, columns, indexes): a new container can fail after it and leave
// the old one online, still reading the old names (docs/STORE.md, §1.3).

const at = (name: string) => timestamp(name, { withTimezone: true });

/** What the service notes about itself: the last beat of the queue (`queue.beat`), read by /api/health. */
export const serviceState = pgTable('service_state', {
  key: text('key').primaryKey(),
  at: at('at').notNull().defaultNow(),
});

/**
 * An author's account: their GitHub identity (the numeric id, which does not change; the login, name and avatar are
 * read again at each sign-in), and the e-mail they gave for the notices, once its link is followed (docs/STORE.md, §2.3).
 */
export const accounts = pgTable('accounts', {
  id: serial('id').primaryKey(),
  provider: text('provider').notNull().default('github'),
  githubId: bigint('github_id', { mode: 'number' }).notNull().unique(),
  login: text('login').notNull(),
  name: text('name'),
  avatarUrl: text('avatar_url'),
  email: text('email'),
  emailVerifiedAt: at('email_verified_at'),
  createdAt: at('created_at').notNull().defaultNow(),
  seenAt: at('seen_at').notNull().defaultNow(),
});

/**
 * A session: the hash of its random token (the cookie holds the token, the base never does), 30 days. The user's
 * GitHub token, sealed (AES-256-GCM), reads their installations of the App; it goes at sign-out, and after 8 h at most.
 */
export const sessions = pgTable(
  'sessions',
  {
    id: text('id').primaryKey(),
    accountId: integer('account_id')
      .notNull()
      .references(() => accounts.id, { onDelete: 'cascade' }),
    createdAt: at('created_at').notNull().defaultNow(),
    expiresAt: at('expires_at').notNull(),
    githubToken: text('github_token'),
    githubTokenExpiresAt: at('github_token_expires_at'),
  },
  (t) => [index('sessions_account').on(t.accountId)],
);

/** The link sent to check an e-mail address: the hash of its token, the address it vouches for, 24 hours. */
export const emailTokens = pgTable(
  'email_tokens',
  {
    id: text('id').primaryKey(),
    accountId: integer('account_id')
      .notNull()
      .references(() => accounts.id, { onDelete: 'cascade' }),
    email: text('email').notNull(),
    createdAt: at('created_at').notNull().defaultNow(),
    expiresAt: at('expires_at').notNull(),
  },
  (t) => [index('email_tokens_account').on(t.accountId)],
);

/** An installation of the GitHub App: on a user or an organization, as its webhooks tell it. */
export const installations = pgTable('installations', {
  id: bigint('id', { mode: 'number' }).primaryKey(),
  ownerId: bigint('owner_id', { mode: 'number' }).notNull(),
  ownerLogin: text('owner_login').notNull(),
  ownerType: text('owner_type').notNull(),
  repositorySelection: text('repository_selection').notNull().default('selected'),
  suspendedAt: at('suspended_at'),
  createdAt: at('created_at').notNull().defaultNow(),
  updatedAt: at('updated_at').notNull().defaultNow(),
});

/** The repositories an installation reaches (all of an owner's, or the chosen ones). */
export const installationRepos = pgTable(
  'installation_repos',
  {
    installationId: bigint('installation_id', { mode: 'number' })
      .notNull()
      .references(() => installations.id, { onDelete: 'cascade' }),
    repoId: bigint('repo_id', { mode: 'number' }).notNull(),
    fullName: text('full_name').notNull(),
    private: boolean('private').notNull().default(false),
  },
  (t) => [primaryKey({ columns: [t.installationId, t.repoId] })],
);

/** Which installations an account may manage: those GitHub lists for them at sign-in (they administer them). */
export const accountInstallations = pgTable(
  'account_installations',
  {
    accountId: integer('account_id')
      .notNull()
      .references(() => accounts.id, { onDelete: 'cascade' }),
    installationId: bigint('installation_id', { mode: 'number' })
      .notNull()
      .references(() => installations.id, { onDelete: 'cascade' }),
  },
  (t) => [primaryKey({ columns: [t.accountId, t.installationId] })],
);

/** The webhooks received, by GitHub's delivery id: a delivery replayed is not handled twice. 30 days. */
export const webhookDeliveries = pgTable('webhook_deliveries', {
  id: text('id').primaryKey(),
  event: text('event').notNull(),
  action: text('action'),
  receivedAt: at('received_at').notNull().defaultNow(),
});

/**
 * The journal of actions (who signed in, gave an address, exported, deleted; what GitHub said of the App's
 * installations), for security and disputes: one year (docs/STORE.md, §2.3). The account's row may go before its
 * entries: they keep the GitHub id of whoever acted.
 */
export const audit = pgTable(
  'audit',
  {
    id: bigserial('id', { mode: 'number' }).primaryKey(),
    at: at('at').notNull().defaultNow(),
    accountId: integer('account_id').references(() => accounts.id, { onDelete: 'set null' }),
    githubId: bigint('github_id', { mode: 'number' }),
    action: text('action').notNull(),
    detail: jsonb('detail').$type<Record<string, unknown>>(),
  },
  (t) => [index('audit_account').on(t.accountId), index('audit_at').on(t.at)],
);

/**
 * A plugin of the store: its id (the manifest's, reserved to its account even once removed), the repository it is
 * published from and the installation of the App that reads it, what the author chose at the first publication
 * (categories, keywords), and whether a new tag may be published without their click (docs/STORE.md, §3.5).
 */
export const plugins = pgTable(
  'plugins',
  {
    id: text('id').primaryKey(),
    accountId: integer('account_id').references(() => accounts.id, { onDelete: 'set null' }),
    repoId: bigint('repo_id', { mode: 'number' }).notNull(),
    repoFullName: text('repo_full_name').notNull(),
    installationId: bigint('installation_id', { mode: 'number' }),
    categories: jsonb('categories').$type<string[]>().notNull().default([]),
    keywords: jsonb('keywords').$type<string[]>().notNull().default([]),
    autoPublish: boolean('auto_publish').notNull().default(false),
    createdAt: at('created_at').notNull().defaultNow(),
  },
  (t) => [index('plugins_account').on(t.accountId), index('plugins_repo').on(t.repoId)],
);

/**
 * A version as submitted (by its author, or by a new tag's webhook) and frozen (docs/STORE.md, §3.4): the tag resolved
 * once into its commit, the archive kept as GitHub sent it and its SHA-256, the release's binaries, the ten checks and
 * their verdict; once they pass, `version.json` and its SHA-256, the version's fingerprint. `state`: `checking`,
 * `error` (the store failed, the checks may run again), `rejected`, `held` (a person must look: a name too close),
 * `ready` (waits for the author's click), `published`. A published version never changes; its number is not used
 * again. A tag moved on the repository afterwards is noted, nothing else changes.
 */
export const versions = pgTable(
  'versions',
  {
    id: serial('id').primaryKey(),
    pluginId: text('plugin_id'),
    accountId: integer('account_id').references(() => accounts.id, { onDelete: 'set null' }),
    origin: text('origin').notNull().default('author'),
    installationId: bigint('installation_id', { mode: 'number' }).notNull(),
    repoId: bigint('repo_id', { mode: 'number' }).notNull(),
    repoFullName: text('repo_full_name').notNull(),
    tag: text('tag').notNull(),
    commit: text('commit').notNull(),
    tagDate: at('tag_date'),
    version: text('version'),
    state: text('state').notNull().default('checking'),
    step: text('step'),
    error: text('error'),
    checks: jsonb('checks').$type<unknown[]>().notNull().default([]),
    manifest: jsonb('manifest').$type<Record<string, unknown>>(),
    manifestSha256: text('manifest_sha256'),
    archiveSha256: text('archive_sha256'),
    archiveSize: integer('archive_size'),
    binaries: jsonb('binaries').$type<Record<string, unknown>>(),
    screenshots: jsonb('screenshots').$type<{ path: string; sha256: string; format: string; width: number; height: number }[]>(),
    versionJson: text('version_json'),
    versionSha256: text('version_sha256'),
    review: boolean('review').notNull().default(true),
    createdAt: at('created_at').notNull().defaultNow(),
    checkedAt: at('checked_at'),
    publishedAt: at('published_at'),
    tagMovedAt: at('tag_moved_at'),
    tagMovedTo: text('tag_moved_to'),
  },
  (t) => [
    index('versions_plugin').on(t.pluginId),
    index('versions_account').on(t.accountId),
    index('versions_repo_tag').on(t.repoId, t.tag),
    uniqueIndex('versions_published')
      .on(t.pluginId, t.version)
      .where(sql`${t.state} = 'published'`),
  ],
);

/**
 * The installations counted, by plugin, version, day and event (`install` or `update`): a counter and nothing else.
 * The app sends one only if its user accepted the statistics, without any id; the store keeps no address, no time
 * finer than the day (docs/STORE.md, §5.4).
 */
export const installsDaily = pgTable(
  'installs_daily',
  {
    pluginId: text('plugin_id').notNull(),
    version: text('version').notNull(),
    day: date('day', { mode: 'string' }).notNull(),
    event: text('event').notNull(),
    count: integer('count').notNull().default(0),
  },
  (t) => [primaryKey({ columns: [t.pluginId, t.version, t.day, t.event] })],
);

/**
 * The revocation lists, as the user signed them on their machine (docs/STORE.md, §4.6): the file's bytes and its
 * `.minisig`, checked with the public keys before they are taken in; the serial only goes up. The last one is served.
 */
export const revocationLists = pgTable('revocation_lists', {
  serial: integer('serial').primaryKey(),
  json: text('json').notNull(),
  minisig: text('minisig').notNull(),
  issuedAt: at('issued_at').notNull(),
  receivedAt: at('received_at').notNull().defaultNow(),
});

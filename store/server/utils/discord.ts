import { and, eq, sql } from 'drizzle-orm';
import { accounts, discordQuota, plugins, reports, signatures, versions } from '../db/schema';
import { closeNames } from './checks';
import type { Env } from './config';
import type { Db } from './db';
import type { Text } from './checks';
import { localized, permissionsOf, type Permission } from './listing';
import { REASON_WORDS, type Reason } from './reports';
import type { RevocationEntry } from './revocation-list';
import { otherPlugins } from './submissions';
import { compareVersions } from './versions';

// The announcements on Escouade's Discord (docs/STORE.md, §4.7, STORE-12): a private channel for the administrators,
// in French (a version to review, a report, a name too close, a tag moved, a revocation), and the public channel in
// French and in English, one webhook per language, as the changelog of the app (a new plugin, a new version, the
// « Vérifié par Escouade » badge, a plugin taken off). Each message is a pg-boss job of its own (store.discord): a
// submission never waits for Discord nor fails for it. Whatever comes from an author (name, description, login,
// version) is hostile: no mention can be made (`allowed_mentions` empty), Discord's Markdown is escaped, the links
// written in it are broken, every length is bounded, and the only links are to the store's own pages. A webhook's
// address is a secret: the job holds the channel's name only, the logs and the errors say its host alone.

export type Channel = 'admin' | 'fr' | 'en';
export const CHANNELS: readonly Channel[] = ['admin', 'fr', 'en'];

/** The variable of each channel's webhook (Coolify); without it, its channel gets nothing. */
export const WEBHOOK_VARS: Record<Channel, string> = {
  admin: 'STORE_DISCORD_ADMIN_WEBHOOK',
  fr: 'STORE_DISCORD_PUBLIC_WEBHOOK_FR',
  en: 'STORE_DISCORD_PUBLIC_WEBHOOK_EN',
};

/** The new plugins and versions of an author announced in a public channel in a UTC day; past it, one summary. */
export const PER_AUTHOR_DAY = 5;
/** The longest wait a 429 is waited for in the job; longer, the job fails and pg-boss tries it again later. */
export const RETRY_AFTER_MAX = 60;
/** The 429s waited for in one run of a job. */
export const RATE_LIMITED_MAX = 3;

/** What happened, by the ids the job reads again when it runs: nothing else of the author goes in the queue. */
export type Announcement =
  | { kind: 'review'; version: number }
  | { kind: 'report'; report: number }
  | { kind: 'held'; version: number }
  | { kind: 'tag-moved'; version: number }
  | { kind: 'revoked'; serial: number; entry: RevocationEntry; quiet: boolean }
  | { kind: 'new-plugin'; version: number }
  | { kind: 'new-version'; version: number }
  | { kind: 'verified'; plugin: string; version: string }
  | { kind: 'removed'; entry: RevocationEntry };

export type Kind = Announcement['kind'];
const ADMIN_KINDS: readonly Kind[] = ['review', 'report', 'held', 'tag-moved', 'revoked'];

/** A job of store.discord: one message, to one channel. */
export interface DiscordJob {
  channel: Channel;
  event: Announcement;
}

/** A job of store.discord-summary: the messages past the limit of an author, a day, a channel, in one. */
export interface SummaryJob {
  channel: Channel;
  login: string;
  day: string;
}

/** The channels a kind of event goes to. */
export const channelsOf = (kind: Kind): Channel[] => (ADMIN_KINDS.includes(kind) ? ['admin'] : ['fr', 'en']);

// ---- The webhooks ----

const DISCORD_HOSTS = new Set(['discord.com', 'discordapp.com', 'ptb.discord.com', 'canary.discord.com']);

/**
 * The webhook of a channel, or null when its variable is missing or is not a webhook of Discord (HTTPS on discord.com,
 * `/api/webhooks/…`; plain HTTP to this machine only, for the tests' fake Discord).
 */
export function webhookOf(env: Env, channel: Channel): URL | null {
  const raw = (env[WEBHOOK_VARS[channel]] ?? '').trim();
  if (!raw) return null;
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return null;
  }
  const local = url.protocol === 'http:' && (url.hostname === '127.0.0.1' || url.hostname === 'localhost');
  if (!local && !(url.protocol === 'https:' && DISCORD_HOSTS.has(url.hostname))) return null;
  if (!/^\/api\/webhooks\/\d+\/[\w-]+\/?$/.test(url.pathname) || url.username || url.password) return null;
  return url;
}

/** The variables set that are not a webhook of Discord: one line each, at start, never their values. */
export function webhookProblems(env: Env): string[] {
  return CHANNELS.filter((c) => (env[WEBHOOK_VARS[c]] ?? '').trim() && !webhookOf(env, c)).map(
    (c) =>
      `[store] discord ${c} off (${WEBHOOK_VARS[c]} is not a Discord webhook, https://discord.com/api/webhooks/…): its channel gets nothing`,
  );
}

// ---- The outbox: from the request to the queue ----

type Outbox = { send: (job: DiscordJob) => Promise<unknown>; env: Env };
let outbox: Outbox | null = null;

/** Set by the queue when it starts (server/utils/queue.ts); null: nothing is announced (no base, tests). */
export function setOutbox(o: Outbox | null) {
  outbox = o;
}

/**
 * Announces an event: one job per channel whose webhook is set, nothing for the others. Never throws and is never
 * waited for by a request (`void announce(…)`): the job is written in the base in the background, Discord is called
 * by the queue's worker.
 */
export async function announce(event: Announcement): Promise<void> {
  const o = outbox;
  if (!o) return;
  for (const channel of channelsOf(event.kind)) {
    if (!webhookOf(o.env, channel)) continue;
    try {
      await o.send({ channel, event });
    } catch (e) {
      console.error(`[store] discord ${channel}: the announcement could not be queued:`, e instanceof Error ? e.message : e);
    }
  }
}

// ---- What hostile text may hold ----

const ZW = '\u200b';

/** Cuts a text to `max` characters (code points), with « … » when it was longer. */
export function cut(s: string, max: number): string {
  const chars = [...s];
  return chars.length <= max
    ? s
    : `${chars
        .slice(0, Math.max(0, max - 1))
        .join('')
        .trimEnd()}…`;
}

/**
 * A text of an author, made safe for a message: on one line, without control, bidirectional or invisible characters,
 * cut to `max`, its links broken (`https:​//`, no link is ever made of it) and every sign of Discord's Markdown
 * escaped (bold, links, code, quotes, spoilers, headings, mentions, emoji, timestamps).
 */
export function md(input: unknown, max: number): string {
  const s = String(input ?? '')
    .normalize('NFC')
    .replace(/[\u0000-\u001f\u007f-\u009f\u00ad\u061c\u200b-\u200f\u2028-\u202e\u2060-\u206f\ufeff]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  return cut(s, max)
    .replace(/:\/\//g, `:${ZW}//`)
    .replace(/[\\*_~`|>#\-[\]()<:!.@+=]/g, (c) => `\\${c}`);
}

/** A text of several lines (the review's public summary): each line made safe by `md`, 8 lines and `max` in all. */
export function mdLines(input: unknown, max: number): string {
  const lines = String(input ?? '')
    .split(/\r\n?|\n/)
    .map((l) => l.trim())
    .filter(Boolean)
    .slice(0, 8);
  let room = max;
  const out: string[] = [];
  for (const l of lines) {
    if (room <= 0) break;
    out.push(md(l, room));
    room -= [...l].length;
  }
  return out.join('\n');
}

/** An id or a version in a command or a code: its safe characters alone. */
const code = (s: string, max = 100) => cut(s.replace(/[^\w.+-]/g, ''), max);

/** A page of the store: the only links of the messages. */
function page(site: string, ...parts: string[]): string {
  const enc = (p: string) => encodeURIComponent(p).replace(/[()'!*]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`);
  return `${site}/${parts.map(enc).join('/')}`.replace(/\/+$/, '');
}

// ---- What a message says ----

export interface Embed {
  title: string;
  url?: string;
  description?: string;
  color: number;
  fields?: { name: string; value: string; inline?: boolean }[];
  footer?: { text: string };
}

export interface WebhookBody {
  embeds: Embed[];
  allowed_mentions: { parse: [] };
}

/** The store's colours (store.css): its brand, the muted text, the warning and the danger. */
export const COLORS = { brand: 0x35c1bd, muted: 0x8a8f98, warn: 0xe5b85c, danger: 0xe8776b } as const;

/** Discord's own limits, held whatever the parts say (a title 256, a description 4096, a field 256/1024, 6000 in all). */
export function bounded(e: Embed): Embed {
  const out: Embed = { ...e, title: cut(e.title, 256) };
  if (e.description) out.description = cut(e.description, 4096);
  if (e.fields) out.fields = e.fields.slice(0, 25).map((f) => ({ ...f, name: cut(f.name, 256), value: cut(f.value, 1024) }));
  if (e.footer) out.footer = { text: cut(e.footer.text, 2048) };
  const size = (x: Embed) =>
    [...x.title].length +
    [...(x.description ?? '')].length +
    (x.fields ?? []).reduce((n, f) => n + [...f.name].length + [...f.value].length, 0) +
    [...(x.footer?.text ?? '')].length;
  // Past 6000: the last fields go first, then the description is cut.
  while (size(out) > 6000 && out.fields?.length) out.fields = out.fields.slice(0, -1);
  if (size(out) > 6000 && out.description)
    out.description = cut(out.description, Math.max(1, [...out.description].length - (size(out) - 6000)));
  return out;
}

const body = (e: Embed): WebhookBody => ({ embeds: [bounded(e)], allowed_mentions: { parse: [] } });

/** What the job read of a version. */
export interface VersionFacts {
  id: string;
  name: Text;
  description: Text;
  version: string;
  author: string | null;
  verified: boolean;
}

/** What the job read of an event, ready to be written. */
export type Facts =
  | { kind: 'new-plugin' | 'new-version'; v: VersionFacts }
  | { kind: 'verified'; v: VersionFacts; summary: Text | null }
  | { kind: 'review'; v: VersionFacts; base: string | null; previous: string | null; grown: string[] }
  | { kind: 'report'; id: string; name: Text; version: string | null; reason: Reason; open: number; malicious: number }
  | { kind: 'held'; v: VersionFacts; close: string[] }
  | { kind: 'tag-moved'; v: VersionFacts; tag: string; was: string; now: string; published: boolean }
  | { kind: 'revoked'; id: string; name: Text; entry: RevocationEntry; serial: number; quiet: boolean }
  | { kind: 'removed'; id: string; name: Text; entry: RevocationEntry };

const W = {
  fr: {
    newPlugin: 'Nouveau plugin',
    newVersion: 'Nouvelle version',
    verified: 'Vérifié par Escouade',
    removed: 'Plugin retiré',
    author: 'Auteur',
    review: 'Revue',
    unverified: 'Non vérifié : pas encore relu par Escouade',
    verifiedLine: 'Vérifié par Escouade',
    reason: 'Raison',
    versions: (v: RevocationEntry['versions']) => (v === '*' ? 'toutes les versions' : v.map((x) => code(x, 40)).join(', ')),
    summary: 'Les autres nouveautés du jour',
    more: (login: string, n: number) => `${login} a publié ${n} autre${n > 1 ? 's' : ''} nouveauté${n > 1 ? 's' : ''} aujourd’hui :`,
    andMore: (n: number) => `… et ${n} autre${n > 1 ? 's' : ''}`,
    unknown: 'compte supprimé',
  },
  en: {
    newPlugin: 'New plugin',
    newVersion: 'New version',
    verified: 'Verified by Escouade',
    removed: 'Plugin taken off',
    author: 'Author',
    review: 'Review',
    unverified: 'Unverified: not reviewed by Escouade yet',
    verifiedLine: 'Verified by Escouade',
    reason: 'Reason',
    versions: (v: RevocationEntry['versions']) => (v === '*' ? 'every version' : v.map((x) => code(x, 40)).join(', ')),
    summary: 'More from today',
    more: (login: string, n: number) => `${login} published ${n} more release${n > 1 ? 's' : ''} today:`,
    andMore: (n: number) => `… and ${n} more`,
    unknown: 'deleted account',
  },
} as const;

/** The bounds of what an author wrote. */
const MAX = { name: 80, description: 280, version: 40, login: 39, reason: 400, summary: 400, tag: 60, line: 120 } as const;

const pluginPage = (site: string, lang: 'fr' | 'en', id: string) => page(site, ...(lang === 'en' ? ['en'] : []), 'plugins', id);
const authorLink = (site: string, lang: 'fr' | 'en', login: string | null) =>
  login ? `[${md(login, MAX.login)}](${page(site, ...(lang === 'en' ? ['en'] : []), 'authors', login)})` : W[lang].unknown;
const headline = (name: string, version: string | null, id?: string) =>
  `**${md(name, MAX.name)}**${version ? ` ${md(version, MAX.version)}` : ''}${id ? ` · \`${code(id)}\`` : ''}`;

/** The message of an event for a channel (the administrators' in French, the public's in its language). */
export function render(f: Facts, channel: Channel, site: string): WebhookBody {
  if (channel === 'admin') return renderAdmin(f, site);
  const lang = channel;
  const w = W[lang];
  switch (f.kind) {
    case 'new-plugin':
    case 'new-version': {
      const desc = f.v.description[lang] ? `\n${md(f.v.description[lang], MAX.description)}` : '';
      return body({
        title: f.kind === 'new-plugin' ? w.newPlugin : w.newVersion,
        url: pluginPage(site, lang, f.v.id),
        description: `${headline(f.v.name[lang], f.v.version)}${f.kind === 'new-plugin' ? desc : ''}`,
        color: f.kind === 'new-plugin' ? COLORS.brand : COLORS.muted,
        fields: [
          { name: w.author, value: authorLink(site, lang, f.v.author), inline: true },
          { name: w.review, value: f.v.verified ? `✓ ${w.verifiedLine}` : w.unverified, inline: true },
        ],
      });
    }
    case 'verified':
      return body({
        title: w.verified,
        url: pluginPage(site, lang, f.v.id),
        description: `${headline(f.v.name[lang], f.v.version)}${f.summary?.[lang] ? `\n${mdLines(f.summary[lang], MAX.summary)}` : ''}`,
        color: COLORS.brand,
        fields: [{ name: w.author, value: authorLink(site, lang, f.v.author), inline: true }],
      });
    case 'removed':
      return body({
        title: w.removed,
        url: pluginPage(site, lang, f.id),
        description: `${headline(f.name[lang], null)} · ${w.versions(f.entry.versions)}`,
        color: COLORS.danger,
        fields: [{ name: w.reason, value: md(f.entry.reason[lang], MAX.reason) || '—' }],
      });
    default:
      throw new Error(`no public message for ${f.kind}`);
  }
}

function renderAdmin(f: Facts, site: string): WebhookBody {
  const review = page(site, 'review');
  const queue = { name: 'File', value: `[${md('/review', 20)}](${review})`, inline: true };
  switch (f.kind) {
    case 'review': {
      const since = f.base && f.previous && f.base !== f.previous ? ` --since ${code(f.base)}` : '';
      const grown = f.grown.length ? f.grown.map((g) => `• ${md(g, MAX.line)}`).join('\n') : 'Rien de plus dans les permissions';
      return body({
        title: 'Version à revoir',
        url: review,
        description: headline(f.v.name.fr, f.v.version, f.v.id),
        color: COLORS.warn,
        fields: [
          { name: 'Auteur', value: authorLink(site, 'fr', f.v.author), inline: true },
          queue,
          {
            name: f.base
              ? `Depuis la dernière vérifiée (${code(f.base, 40)})`
              : f.previous
                ? `Depuis la ${code(f.previous, 40)} (aucune vérifiée)`
                : 'Première version',
            value: grown,
          },
          { name: 'Commande', value: `\`\`\`\nnpm run plugin-review -- ${code(f.v.id)}@${code(f.v.version, 40)}${since}\n\`\`\`` },
        ],
      });
    }
    case 'report':
      return body({
        title: 'Signalement',
        url: review,
        description: headline(f.name.fr, f.version, f.id),
        color: COLORS.danger,
        fields: [
          { name: 'Motif', value: REASON_WORDS[f.reason]?.fr ?? 'Autre', inline: true },
          { name: 'Ouverts sur ce plugin', value: `${f.open}${f.malicious ? ` (dont ${f.malicious} « malveillant »)` : ''}`, inline: true },
          queue,
        ],
      });
    case 'held':
      return body({
        title: 'Nom trop proche',
        url: review,
        description: headline(f.v.name.fr, f.v.version, f.v.id),
        color: COLORS.warn,
        fields: [
          { name: 'Auteur', value: authorLink(site, 'fr', f.v.author), inline: true },
          { name: 'Proche de', value: f.close.map((c) => `\`${code(c)}\``).join(', ') || '—', inline: true },
          queue,
        ],
      });
    case 'tag-moved':
      return body({
        title: 'Étiquette déplacée',
        url: f.published ? pluginPage(site, 'fr', f.v.id) : review,
        description: headline(f.v.name.fr, f.v.version, f.v.id),
        color: COLORS.warn,
        fields: [
          { name: 'Étiquette', value: md(f.tag, MAX.tag), inline: true },
          { name: 'Commit', value: `\`${code(f.was, 12).slice(0, 7)}\` → \`${code(f.now, 12).slice(0, 7)}\``, inline: true },
          { name: 'Version', value: f.published ? 'publiée, inchangée sur le store' : 'en attente', inline: true },
          { name: 'Auteur', value: authorLink(site, 'fr', f.v.author), inline: true },
        ],
      });
    case 'revoked':
      return body({
        title: f.entry.action === 'disable' ? 'Retrait d’urgence fait' : 'Badge retiré',
        url: pluginPage(site, 'fr', f.id),
        description: `${headline(f.name.fr, null, f.id)} · ${W.fr.versions(f.entry.versions)}`,
        color: COLORS.danger,
        fields: [
          { name: 'Raison publique', value: md(f.entry.reason.fr, MAX.reason) || '—' },
          { name: 'Liste de révocation', value: `n° ${f.serial} publiée`, inline: true },
          { name: 'Auteur et salon public', value: f.quiet ? 'non prévenus (--quiet)' : 'prévenus', inline: true },
        ],
      });
    default:
      throw new Error(`no message for the administrators for ${f.kind}`);
  }
}

/** The one message of an author's announcements past the limit of the day. */
export function renderSummary(channel: 'fr' | 'en', site: string, login: string, lines: string[]): WebhookBody {
  const w = W[channel];
  const shown = lines.slice(0, 10).map((l) => `• ${md(l, MAX.line)}`);
  if (lines.length > shown.length) shown.push(w.andMore(lines.length - shown.length));
  return body({
    title: w.summary,
    url: page(site, ...(channel === 'en' ? ['en'] : []), 'authors', login),
    description: `${w.more(md(login, MAX.login), lines.length)}\n${shown.join('\n')}`,
    color: COLORS.muted,
  });
}

// ---- What the job reads again ----

async function versionFacts(db: Db, id: number) {
  const [r] = await db
    .select({ v: versions, plugin: plugins, login: accounts.login })
    .from(versions)
    .leftJoin(plugins, eq(plugins.id, versions.pluginId))
    .leftJoin(accounts, eq(accounts.id, versions.accountId))
    .where(eq(versions.id, id));
  if (!r?.v.pluginId) return null;
  const m = r.v.manifest ?? {};
  const [sig] = r.v.version
    ? await db
        .select({ sha: signatures.versionSha256 })
        .from(signatures)
        .where(and(eq(signatures.pluginId, r.v.pluginId), eq(signatures.version, r.v.version)))
    : [];
  const facts: VersionFacts = {
    id: r.v.pluginId,
    name: nameOf(m, r.v.pluginId),
    description: localized(m.description),
    version: r.v.version ?? r.v.tag,
    author: r.login ?? null,
    verified: !!sig && sig.sha === r.v.versionSha256 && r.v.reviewState === 'verified',
  };
  return { facts, row: r.v, plugin: r.plugin };
}

function nameOf(m: Record<string, unknown>, id: string): Text {
  const n = localized(m.name);
  return { fr: n.fr || id, en: n.en || id };
}

/** The name of a plugin by its last published version, its id without one. */
async function pluginName(db: Db, id: string): Promise<Text> {
  const rows = await db
    .select({ version: versions.version, manifest: versions.manifest })
    .from(versions)
    .where(and(eq(versions.pluginId, id), eq(versions.state, 'published')));
  rows.sort((a, b) => compareVersions(b.version!, a.version!));
  return nameOf(rows[0]?.manifest ?? {}, id);
}

const permissionLine = (p: Permission) => `${p.text.fr}${p.mono ? ` : ${p.mono}` : ''}`;

/**
 * Reads what a message says, when its job runs: null when there is nothing to say any more (the version gone), or
 * when the public would read of a plugin its author took off the list or deleted.
 */
export async function loadFacts(db: Db, e: Announcement, channel: Channel): Promise<Facts | null> {
  const isPublic = channel !== 'admin';
  switch (e.kind) {
    case 'new-plugin':
    case 'new-version':
    case 'review':
    case 'held':
    case 'tag-moved': {
      const got = await versionFacts(db, e.version);
      if (!got) return null;
      if (isPublic && (!got.plugin || got.plugin.unlistedAt || got.plugin.deletedAt || got.row.state !== 'published')) return null;
      if (e.kind === 'new-plugin' || e.kind === 'new-version') return { kind: e.kind, v: got.facts };
      if (e.kind === 'held') {
        const others = await otherPlugins(db, got.row.accountId);
        const close = closeNames(got.facts.id, got.facts.name.en, others).map((c) => c.id);
        return { kind: 'held', v: got.facts, close };
      }
      if (e.kind === 'tag-moved')
        return {
          kind: 'tag-moved',
          v: got.facts,
          tag: got.row.tag,
          was: got.row.commit,
          now: got.row.tagMovedTo ?? '',
          published: got.row.state === 'published',
        };
      // A version to review: what it asks more than the last verified (else the one before it).
      const all = await db
        .select({ version: versions.version, sha: versions.versionSha256, manifest: versions.manifest, binaries: versions.binaries })
        .from(versions)
        .where(and(eq(versions.pluginId, got.facts.id), eq(versions.state, 'published')));
      const below = all.filter((x) => x.version && compareVersions(x.version, got.facts.version) < 0);
      below.sort((a, b) => compareVersions(b.version!, a.version!));
      const signed = await db.select().from(signatures).where(eq(signatures.pluginId, got.facts.id));
      const base = below.find((x) => signed.some((s) => s.version === x.version && s.versionSha256 === x.sha)) ?? null;
      const compared = base ?? below[0] ?? null;
      const had = new Set(compared ? permissionsOf(compared.manifest ?? {}, compared.binaries).map(permissionLine) : []);
      const grown = permissionsOf(got.row.manifest ?? {}, got.row.binaries)
        .map(permissionLine)
        .filter((p) => !had.has(p));
      return { kind: 'review', v: got.facts, base: base?.version ?? null, previous: below[0]?.version ?? null, grown };
    }
    case 'verified': {
      const [row] = await db
        .select({ id: versions.id })
        .from(versions)
        .where(and(eq(versions.pluginId, e.plugin), eq(versions.version, e.version), eq(versions.state, 'published')));
      const got = row ? await versionFacts(db, row.id) : null;
      if (!got || !got.facts.verified || !got.plugin || got.plugin.unlistedAt || got.plugin.deletedAt) return null;
      const [sig] = await db
        .select({ summary: signatures.summary })
        .from(signatures)
        .where(and(eq(signatures.pluginId, e.plugin), eq(signatures.version, e.version)));
      return { kind: 'verified', v: got.facts, summary: sig?.summary ?? null };
    }
    case 'report': {
      const [r] = await db.select().from(reports).where(eq(reports.id, e.report));
      if (!r) return null;
      const open = await db
        .select({ reason: reports.reason })
        .from(reports)
        .where(and(eq(reports.pluginId, r.pluginId), eq(reports.state, 'open')));
      return {
        kind: 'report',
        id: r.pluginId,
        name: await pluginName(db, r.pluginId),
        version: r.version,
        reason: r.reason as Reason,
        open: open.length,
        malicious: open.filter((x) => x.reason === 'malicious').length,
      };
    }
    case 'revoked':
      return { kind: 'revoked', id: e.entry.id, name: await pluginName(db, e.entry.id), entry: e.entry, serial: e.serial, quiet: e.quiet };
    case 'removed':
      return { kind: 'removed', id: e.entry.id, name: await pluginName(db, e.entry.id), entry: e.entry };
  }
}

// ---- The limit per author and per day ----

const COUNTED: readonly Kind[] = ['new-plugin', 'new-version'];

/**
 * Counts a public announcement of an author for the day: `send` within the limit; past it, the line is kept for the
 * summary and `first-over` says the summary must be planned (once), `over` that it already is.
 */
export async function takeQuota(
  db: Db,
  channel: Channel,
  login: string,
  day: string,
  line: string,
): Promise<'send' | 'first-over' | 'over'> {
  const [row] = await db
    .insert(discordQuota)
    .values({ channel, login, day, sent: 1 })
    .onConflictDoUpdate({
      target: [discordQuota.channel, discordQuota.login, discordQuota.day],
      set: { sent: sql`${discordQuota.sent} + 1` },
    })
    .returning({ sent: discordQuota.sent });
  const n = row!.sent;
  if (n <= PER_AUTHOR_DAY) return 'send';
  await db
    .update(discordQuota)
    .set({ skipped: sql`${discordQuota.skipped} || ${JSON.stringify([line])}::jsonb` })
    .where(and(eq(discordQuota.channel, channel), eq(discordQuota.login, login), eq(discordQuota.day, day)));
  return n === PER_AUTHOR_DAY + 1 ? 'first-over' : 'over';
}

// ---- The sending ----

export interface Deliver {
  env: Env;
  site: string;
  fetch?: typeof fetch;
  sleep?: (ms: number) => Promise<void>;
  now?: () => Date;
  /** Plans the summary of a day (store.discord-summary), after `at`. */
  later?: (job: SummaryJob, at: Date) => Promise<unknown>;
}

/** A failure Discord may get over: pg-boss tries the job again, later. Its message holds the host alone. */
export class DiscordRetry extends Error {}

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** The seconds a 429 asks to wait: `Retry-After`, else the body's `retry_after`, else one. */
async function retryAfter(res: Response): Promise<number> {
  const header = Number(res.headers.get('retry-after'));
  let fromBody = NaN;
  try {
    fromBody = Number(((await res.json()) as { retry_after?: unknown }).retry_after);
  } catch {
    // Not JSON: the header alone.
  }
  const s = Math.max(Number.isFinite(header) ? header : 0, Number.isFinite(fromBody) ? fromBody : 0);
  return s > 0 ? s : 1;
}

/**
 * Posts a message to a channel's webhook. A 429 is waited for (`Retry-After`) and tried again, up to three times in
 * the job, or failed for pg-boss when the wait is longer than a minute; a 5xx or a network failure fails the job (tried
 * again later, spaced); another 4xx (the webhook deleted, the message refused) is dropped and said in the log. Only the
 * webhook's host is ever written.
 */
export async function post(
  url: URL,
  channel: Channel,
  message: WebhookBody,
  how: Pick<Deliver, 'fetch' | 'sleep'> = {},
): Promise<'sent' | 'dropped'> {
  const doFetch = how.fetch ?? fetch;
  const sleep = how.sleep ?? wait;
  const where = `discord ${channel} (${url.host})`;
  for (let limited = 0; ; limited++) {
    let res: Response;
    try {
      res = await doFetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(message),
        redirect: 'error',
        signal: AbortSignal.timeout(15_000),
      });
    } catch (e) {
      const why = e instanceof Error ? ((e.cause as { code?: string } | undefined)?.code ?? e.name) : 'error';
      throw new DiscordRetry(`${where}: no answer (${why})`);
    }
    if (res.ok) return 'sent';
    if (res.status === 429) {
      const s = await retryAfter(res);
      if (limited >= RATE_LIMITED_MAX || s > RETRY_AFTER_MAX) throw new DiscordRetry(`${where}: rate limited, retry after ${s}s`);
      await sleep(Math.ceil(s * 1000));
      continue;
    }
    if (res.status >= 500) throw new DiscordRetry(`${where}: HTTP ${res.status}`);
    console.error(`[store] ${where}: HTTP ${res.status}, message dropped`);
    return 'dropped';
  }
}

/** The UTC day of a date, and the next midnight. */
const dayOf = (d: Date) => d.toISOString().slice(0, 10);
const nextMidnight = (d: Date) => new Date(Date.UTC(d.getUTCFullYear(), d.getUTCMonth(), d.getUTCDate() + 1));

/** Works a job of store.discord: reads the event again, holds the limit per author, writes and sends the message. */
export async function deliver(db: Db, job: DiscordJob, how: Deliver): Promise<'sent' | 'dropped' | 'off' | 'gone' | 'over'> {
  const url = webhookOf(how.env, job.channel);
  if (!url) return 'off';
  const facts = await loadFacts(db, job.event, job.channel);
  if (!facts) return 'gone';
  if (job.channel !== 'admin' && COUNTED.includes(facts.kind) && 'v' in facts) {
    const now = (how.now ?? (() => new Date()))();
    const login = facts.v.author ?? '';
    const line = `${facts.v.name[job.channel]} ${facts.v.version}`;
    const quota = await takeQuota(db, job.channel, login, dayOf(now), line);
    if (quota !== 'send') {
      if (quota === 'first-over' && login) await how.later?.({ channel: job.channel, login, day: dayOf(now) }, nextMidnight(now));
      return 'over';
    }
  }
  return post(url, job.channel, render(facts, job.channel, how.site), how);
}

/** Works a job of store.discord-summary: the lines past the limit of a day, in one message. */
export async function deliverSummary(db: Db, job: SummaryJob, how: Deliver): Promise<'sent' | 'dropped' | 'off' | 'gone'> {
  const url = webhookOf(how.env, job.channel);
  if (!url || job.channel === 'admin') return 'off';
  const [row] = await db
    .select({ skipped: discordQuota.skipped })
    .from(discordQuota)
    .where(and(eq(discordQuota.channel, job.channel), eq(discordQuota.login, job.login), eq(discordQuota.day, job.day)));
  if (!row?.skipped.length) return 'gone';
  return post(url, job.channel, renderSummary(job.channel, how.site, job.login, row.skipped), how);
}

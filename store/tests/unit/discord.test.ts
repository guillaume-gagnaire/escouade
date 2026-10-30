import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { missingLines } from '../../server/utils/config';
import {
  announce,
  bounded,
  channelsOf,
  DiscordRetry,
  md,
  mdLines,
  post,
  render,
  renderSummary,
  setOutbox,
  webhookOf,
  webhookProblems,
  type Channel,
  type DiscordJob,
  type Facts,
  type VersionFacts,
  type WebhookBody,
} from '../../server/utils/discord';
import { fakeDiscord, type FakeDiscord } from '../fake/discord.mjs';

// The announcements on Discord (docs/STORE.md §4.7, STORE-12), without a base: what each message says in each channel
// and language, what hostile text of an author becomes (no mention, Markdown escaped, links broken, lengths bounded,
// no link but the store's), the webhooks' addresses checked and never written, and the sending against a fake
// Discord (a 429 waited for after its Retry-After, a 5xx tried again by the queue, a 4xx dropped).

const SITE = 'https://store.escouade.dev';

const hostile = {
  name: '@everyone **gras** [clique](https://evil.example/x) <@123> <@&9> <#1> <t:1:R> `code` ||spoiler|| # titre ' + 'x'.repeat(500),
  description: 'Va sur https://evil.example ou discord.gg/abc, <https://evil.example/y> > cité\n# Titre\n- liste @here',
  login: 'evil[x](https://evil.example)',
  version: '1.0.0 @here',
};

const v = (o: Partial<VersionFacts> = {}): VersionFacts => ({
  id: 'io.github.sroux.ntfy',
  name: { fr: 'Notifications ntfy', en: 'ntfy notifications' },
  description: { fr: 'Envoie une notification.', en: 'Sends a notification.' },
  version: '1.2.0',
  author: 'sroux',
  verified: false,
  ...o,
});
const bad = v({
  name: { fr: hostile.name, en: hostile.name },
  description: { fr: hostile.description, en: hostile.description },
  author: hostile.login,
  version: hostile.version,
});
const entry = (
  action: 'disable' | 'unverify',
  reason = { fr: 'Envoie des secrets à un tiers.', en: 'Sends secrets to a third party.' },
) => ({
  id: 'io.github.sroux.ntfy',
  versions: ['1.2.0'] as string[],
  action,
  reason,
  date: '2026-10-29',
});

/** Every event, as the job reads it, with `x` for the version. */
const every = (x: VersionFacts): Facts[] => [
  { kind: 'new-plugin', v: x },
  { kind: 'new-version', v: x },
  { kind: 'verified', v: { ...x, verified: true }, summary: { fr: 'Relu en entier.', en: 'Read in full.' } },
  { kind: 'removed', id: x.id, name: x.name, entry: entry('disable') },
  { kind: 'review', v: x, base: '1.0.0', previous: '1.1.0', grown: ['Se connecte à : evil.example'] },
  { kind: 'report', id: x.id, name: x.name, version: x.version, reason: 'malicious', open: 3, malicious: 2 },
  { kind: 'held', v: x, close: ['io.github.other.ntfy'] },
  { kind: 'tag-moved', v: x, tag: 'v1.2.0', was: 'a'.repeat(40), now: 'b'.repeat(40), published: true },
  { kind: 'revoked', id: x.id, name: x.name, entry: entry('disable'), serial: 4, quiet: false },
];
const channelOf = (f: Facts): Channel[] => channelsOf(f.kind);
const text = (b: WebhookBody) => JSON.stringify(b);

describe('the channels', () => {
  it('send the review, the reports, the names too close, the tags moved and the revocations to the administrators alone', () => {
    for (const k of ['review', 'report', 'held', 'tag-moved', 'revoked'] as const) expect(channelsOf(k)).toEqual(['admin']);
  });
  it('send the new plugins, the new versions, the badge and the removals to the public, in French and in English', () => {
    for (const k of ['new-plugin', 'new-version', 'verified', 'removed'] as const) expect(channelsOf(k)).toEqual(['fr', 'en']);
  });
});

describe('the messages', () => {
  it('to the administrators are in French: the version to review with its link to /review and its command', () => {
    const b = render({ kind: 'review', v: v(), base: '1.0.0', previous: '1.1.0', grown: ['Se connecte à : api.ntfy.sh'] }, 'admin', SITE);
    const e = b.embeds[0]!;
    expect(e.title).toBe('Version à revoir');
    expect(e.url).toBe(`${SITE}/review`);
    expect(e.description).toContain('Notifications ntfy');
    const fields = Object.fromEntries(e.fields!.map((f) => [f.name, f.value]));
    expect(fields.Auteur).toBe(`[sroux](${SITE}/authors/sroux)`);
    expect(fields['Depuis la dernière vérifiée (1.0.0)']).toContain('Se connecte à ');
    expect(fields.Commande).toBe('```\nnpm run plugin-review -- io.github.sroux.ntfy@1.2.0 --since 1.0.0\n```');
    expect(fields.File).toContain(`(${SITE}/review)`);
    // A first version, and a version whose previous is the last verified: no --since.
    const first = render({ kind: 'review', v: v(), base: null, previous: null, grown: [] }, 'admin', SITE).embeds[0]!;
    expect(first.fields!.map((f) => f.name)).toContain('Première version');
    expect(first.fields!.find((f) => f.name === 'Commande')!.value).not.toContain('--since');
  });

  it('to the administrators: a report without its sender nor its text, a name too close, a tag moved, a removal', () => {
    const report = render(
      { kind: 'report', id: 'a.b', name: { fr: 'AB', en: 'AB' }, version: '1.0.0', reason: 'malicious', open: 3, malicious: 2 },
      'admin',
      SITE,
    ).embeds[0]!;
    expect(report.title).toBe('Signalement');
    expect(text({ embeds: [report], allowed_mentions: { parse: [] } })).toContain('Malveillant');
    expect(report.fields!.find((f) => f.name === 'Ouverts sur ce plugin')!.value).toBe('3 (dont 2 « malveillant »)');
    const held = render({ kind: 'held', v: v(), close: ['io.github.other.ntfy'] }, 'admin', SITE).embeds[0]!;
    expect(held.title).toBe('Nom trop proche');
    expect(held.fields!.find((f) => f.name === 'Proche de')!.value).toBe('`io.github.other.ntfy`');
    const moved = render(
      { kind: 'tag-moved', v: v(), tag: 'v1.2.0', was: 'a'.repeat(40), now: 'b'.repeat(40), published: true },
      'admin',
      SITE,
    ).embeds[0]!;
    expect(moved.title).toBe('Étiquette déplacée');
    expect(moved.fields!.find((f) => f.name === 'Commit')!.value).toBe('`aaaaaaa` → `bbbbbbb`');
    const out = render(
      { kind: 'revoked', id: 'a.b', name: { fr: 'AB', en: 'AB' }, entry: entry('disable'), serial: 4, quiet: true },
      'admin',
      SITE,
    ).embeds[0]!;
    expect(out.title).toBe('Retrait d’urgence fait');
    expect(out.fields!.map((f) => f.value)).toEqual(['Envoie des secrets à un tiers\\.', 'n° 4 publiée', 'non prévenu (--quiet)']);
    const unverify = render(
      { kind: 'revoked', id: 'a.b', name: { fr: 'AB', en: 'AB' }, entry: entry('unverify'), serial: 5, quiet: false },
      'admin',
      SITE,
    );
    expect(unverify.embeds[0]!.title).toBe('Badge retiré');
  });

  it('to the public, in its language: a new plugin marked unverified, a new version, the badge, a removal with its public reason', () => {
    const fr = render({ kind: 'new-plugin', v: v() }, 'fr', SITE).embeds[0]!;
    expect(fr).toMatchObject({ title: 'Nouveau plugin', url: `${SITE}/plugins/io.github.sroux.ntfy` });
    expect(fr.description).toBe('**Notifications ntfy** 1\\.2\\.0\nEnvoie une notification\\.');
    expect(fr.fields).toEqual([
      { name: 'Auteur', value: `[sroux](${SITE}/authors/sroux)`, inline: true },
      { name: 'Revue', value: 'Non vérifié : pas encore relu par Escouade', inline: true },
    ]);
    const en = render({ kind: 'new-plugin', v: v() }, 'en', SITE).embeds[0]!;
    expect(en).toMatchObject({ title: 'New plugin', url: `${SITE}/en/plugins/io.github.sroux.ntfy` });
    expect(en.description).toContain('ntfy notifications');
    expect(en.fields![1]!.value).toBe('Unverified: not reviewed by Escouade yet');
    expect(en.fields![0]!.value).toBe(`[sroux](${SITE}/en/authors/sroux)`);
    expect(render({ kind: 'new-version', v: v() }, 'fr', SITE).embeds[0]!.title).toBe('Nouvelle version');
    expect(render({ kind: 'new-version', v: v() }, 'en', SITE).embeds[0]!.title).toBe('New version');
    const badge = render({ kind: 'verified', v: v({ verified: true }), summary: { fr: 'Relu.', en: 'Read.' } }, 'en', SITE).embeds[0]!;
    expect(badge.title).toBe('Verified by Escouade');
    expect(badge.description).toContain('Read\\.');
    expect(render({ kind: 'verified', v: v({ verified: true }), summary: null }, 'fr', SITE).embeds[0]!.title).toBe('Vérifié par Escouade');
    const removed = render({ kind: 'removed', id: 'io.github.sroux.ntfy', name: v().name, entry: entry('disable') }, 'fr', SITE).embeds[0]!;
    expect(removed.title).toBe('Plugin retiré');
    expect(removed.fields).toEqual([{ name: 'Raison', value: 'Envoie des secrets à un tiers\\.' }]);
    expect(render({ kind: 'removed', id: 'x', name: v().name, entry: entry('disable') }, 'en', SITE).embeds[0]!.fields![0]!.value).toBe(
      'Sends secrets to a third party\\.',
    );
    // A badge withdrawn, with its public reason, in each language.
    const unverified = render({ kind: 'removed', id: 'io.github.sroux.ntfy', name: v().name, entry: entry('unverify') }, 'fr', SITE)
      .embeds[0]!;
    expect(unverified).toMatchObject({ title: 'Badge retiré', fields: [{ name: 'Raison', value: 'Envoie des secrets à un tiers\\.' }] });
    expect(render({ kind: 'removed', id: 'x', name: v().name, entry: entry('unverify') }, 'en', SITE).embeds[0]).toMatchObject({
      title: 'Badge withdrawn',
      fields: [{ name: 'Reason', value: 'Sends secrets to a third party\\.' }],
    });
  });

  it('never mention anyone, escape the Markdown, break the links, and bound every length, whatever the author wrote', () => {
    for (const f of every(bad))
      for (const channel of channelOf(f)) {
        const b = render(f, channel, SITE);
        expect(b.allowed_mentions).toEqual({ parse: [] });
        expect(Object.keys(b).sort()).toEqual(['allowed_mentions', 'embeds']);
        const s = text(b);
        // No link but the store's pages.
        for (const url of s.match(/https?:\/\/[^\s)\]>"\\]+/g) ?? []) expect(url.startsWith(`${SITE}/`) || url === SITE).toBe(true);
        expect(s).not.toContain('evil.example/x)');
        expect(s).not.toContain('discord.gg');
        // No mention, no channel, no role, no timestamp, no masked link: each sign escaped.
        const all = b.embeds
          .flatMap((e) => [e.title, e.description ?? '', ...(e.fields ?? []).flatMap((x) => [x.name, x.value])])
          .join('\n');
        expect(all).not.toMatch(/(^|[^\\])@(everyone|here)/);
        expect(all).not.toMatch(/(^|[^\\])<[@#t]/);
        expect(all).not.toMatch(/(^|[^\\])\*\*gras/);
        expect(all).not.toMatch(/(^|[^\\])\|\|spoiler/);
        for (const e of b.embeds) {
          expect([...e.title].length).toBeLessThanOrEqual(256);
          expect([...(e.description ?? '')].length).toBeLessThanOrEqual(4096);
          for (const x of e.fields ?? []) {
            expect([...x.name].length).toBeLessThanOrEqual(256);
            expect([...x.value].length).toBeLessThanOrEqual(1024);
          }
        }
        // The name is cut: 80 characters of it at most, before its escapes.
        expect(all).not.toContain('x'.repeat(81));
      }
  });

  it('sum up an author’s announcements past the limit in one message, linked to their page', () => {
    const lines = Array.from({ length: 13 }, (_, i) => `Plugin ${i} 1.0.${i}`);
    const fr = renderSummary('fr', SITE, 'sroux', lines).embeds[0]!;
    expect(fr.title).toBe('Les autres nouveautés du jour');
    expect(fr.url).toBe(`${SITE}/authors/sroux`);
    expect(fr.description).toContain('sroux a publié 13 autres nouveautés aujourd’hui :');
    expect(fr.description).toContain('… et 3 autres');
    const en = renderSummary('en', SITE, 'sroux', lines.slice(0, 1)).embeds[0]!;
    expect(en.description).toBe('sroux published 1 more release today:\n• Plugin 0 1\\.0\\.0');
    expect(en.url).toBe(`${SITE}/en/authors/sroux`);
  });
});

describe('a text of an author', () => {
  it.each([
    ['**gras** _it_ ~~barré~~', '\\*\\*gras\\*\\* \\_it\\_ \\~\\~barré\\~\\~'],
    ['[lien](https://evil.example)', '\\[lien\\]\\(https\\:\u200b//evil\\.example\\)'],
    ['@everyone @here <@123>', '\\@everyone \\@here \\<\\@123\\>'],
    ['`code` ||caché|| > cité', '\\`code\\` \\|\\|caché\\|\\| > cité'.replace('> ', '\\> ')],
    ['ligne 1\nligne 2\r\n# titre', 'ligne 1 ligne 2 \\# titre'],
    ['a\u202eb\u200bc\u0000d', 'a b c d'],
  ])('%j becomes %j', (input, out) => {
    expect(md(input, 200)).toBe(out);
  });

  it('keeps its lines when it is the review’s summary, each made safe, eight at most', () => {
    expect(mdLines('- Réseau vers hooks.slack.com\r\n\n- @everyone [x](https://evil.example)', 400)).toBe(
      '\\- Réseau vers hooks\\.slack\\.com\n\\- \\@everyone \\[x\\]\\(https\\:​//evil\\.example\\)',
    );
    expect(mdLines(Array.from({ length: 20 }, (_, i) => `l${i}`).join('\n'), 400).split('\n')).toHaveLength(8);
    expect([...mdLines('a'.repeat(300) + '\n' + 'b'.repeat(300), 400)].length).toBeLessThanOrEqual(401);
  });

  it('is cut to its bound, the cut said by « … », before its escapes', () => {
    expect(md('é'.repeat(50), 10)).toBe(`${'é'.repeat(9)}…`);
    expect(md('*'.repeat(50), 4)).toBe('\\*\\*\\*…');
  });

  it('and Discord’s own limits hold whatever the parts say', () => {
    const e = bounded({
      title: 't'.repeat(300),
      description: 'd'.repeat(5000),
      color: 0,
      fields: Array.from({ length: 30 }, () => ({ name: 'n'.repeat(300), value: 'v'.repeat(2000) })),
    });
    expect([...e.title].length).toBe(256);
    expect(e.fields![0]!.value.length).toBe(1024);
    const size = [...e.title].length + [...e.description!].length + e.fields!.reduce((n, f) => n + f.name.length + f.value.length, 0);
    expect(size).toBeLessThanOrEqual(6000);
    expect(e.fields!.length).toBeLessThanOrEqual(25);
  });
});

describe('the webhooks', () => {
  const real = 'https://discord.com/api/webhooks/123456789012345678/AbC-dEf_123';
  it('are Discord’s, over HTTPS, or the fake Discord on this machine', () => {
    expect(webhookOf({ STORE_DISCORD_ADMIN_WEBHOOK: real }, 'admin')?.host).toBe('discord.com');
    expect(webhookOf({ STORE_DISCORD_PUBLIC_WEBHOOK_FR: real.replace('discord.com', 'discordapp.com') }, 'fr')?.host).toBe(
      'discordapp.com',
    );
    expect(webhookOf({ STORE_DISCORD_PUBLIC_WEBHOOK_EN: 'http://127.0.0.1:9/api/webhooks/1/tok' }, 'en')?.host).toBe('127.0.0.1:9');
    for (const bad of [
      real.replace('https', 'http'),
      'https://evil.example/api/webhooks/1/tok',
      'https://discord.com/api/channels/1',
      'https://user:pw@discord.com/api/webhooks/1/tok',
      'not a url',
    ])
      expect(webhookOf({ STORE_DISCORD_ADMIN_WEBHOOK: bad }, 'admin')).toBeNull();
    expect(webhookOf({}, 'admin')).toBeNull();
  });

  it('missing, say so once at start, each channel its line; wrong, say so without the value', () => {
    const lines = missingLines({});
    expect(lines).toContain('[store] discord admin off (missing STORE_DISCORD_ADMIN_WEBHOOK): the administrators’ channel gets nothing');
    expect(lines).toContain('[store] discord fr off (missing STORE_DISCORD_PUBLIC_WEBHOOK_FR): the French public channel gets nothing');
    expect(lines).toContain('[store] discord en off (missing STORE_DISCORD_PUBLIC_WEBHOOK_EN): the English public channel gets nothing');
    const wrong = webhookProblems({ STORE_DISCORD_ADMIN_WEBHOOK: 'https://evil.example/api/webhooks/1/s3cr3t' });
    expect(wrong).toHaveLength(1);
    expect(wrong[0]).not.toContain('s3cr3t');
    expect(wrong[0]).not.toContain('evil.example');
  });
});

describe('the announcements', () => {
  afterEach(() => setOutbox(null));

  it('make one job per channel whose webhook is set, and none for a channel without one', async () => {
    const jobs: DiscordJob[] = [];
    setOutbox({ send: async (job) => jobs.push(job), env: { STORE_DISCORD_PUBLIC_WEBHOOK_FR: 'http://127.0.0.1:9/api/webhooks/1/fr' } });
    await announce({ kind: 'new-plugin', version: 1 });
    await announce({ kind: 'review', version: 1 });
    expect(jobs).toEqual([{ channel: 'fr', event: { kind: 'new-plugin', version: 1 } }]);
    // Nothing set: nothing at all.
    setOutbox({ send: async (job) => jobs.push(job), env: {} });
    await announce({ kind: 'report', report: 3 });
    expect(jobs).toHaveLength(1);
    // No queue (no base): nothing, and no error.
    setOutbox(null);
    await expect(announce({ kind: 'new-version', version: 2 })).resolves.toBeUndefined();
  });

  it('never fail their caller: a queue that refuses is said in the log', async () => {
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
    setOutbox({
      send: async () => {
        throw new Error('the base is down');
      },
      env: { STORE_DISCORD_ADMIN_WEBHOOK: 'http://127.0.0.1:9/api/webhooks/1/SECRET' },
    });
    await expect(announce({ kind: 'held', version: 1 })).resolves.toBeUndefined();
    expect(errors.mock.calls.flat().join(' ')).toContain('the base is down');
    expect(errors.mock.calls.flat().join(' ')).not.toContain('SECRET');
    errors.mockRestore();
  });
});

describe('the sending, against a fake Discord', () => {
  let discord: FakeDiscord;
  const logs: string[] = [];
  const message = render({ kind: 'new-plugin', v: v() }, 'fr', SITE);
  beforeAll(async () => {
    discord = await fakeDiscord();
    for (const level of ['error', 'warn', 'info', 'log'] as const)
      vi.spyOn(console, level).mockImplementation((...a: unknown[]) => void logs.push(a.map(String).join(' ')));
  });
  afterEach(() => discord.reset());
  afterAll(async () => {
    vi.restoreAllMocks();
    await discord.close();
    // The webhook's address is in no log: its host alone.
    expect(logs.length).toBeGreaterThan(0);
    for (const l of logs) {
      expect(l).not.toContain('tok3n');
      expect(l).not.toContain('/api/webhooks');
    }
  });

  it('posts the message as it is, once', async () => {
    expect(await post(new URL(discord.hook('fr')), 'fr', message)).toBe('sent');
    expect(discord.of('fr')).toEqual([message]);
  });

  it('waits for the Retry-After of a 429, then sends it again', async () => {
    discord.plan([{ status: 429, retryAfter: 1 }]);
    const t0 = Date.now();
    expect(await post(new URL(discord.hook('en')), 'en', message)).toBe('sent');
    expect(Date.now() - t0).toBeGreaterThanOrEqual(1000);
    expect(discord.of('en')).toHaveLength(1);
    // The wait asked, to the millisecond, a fraction of a second included.
    const waits: number[] = [];
    discord.plan([
      { status: 429, retryAfter: 2.5 },
      { status: 429, retryAfter: 0.2 },
    ]);
    expect(await post(new URL(discord.hook('en')), 'en', message, { sleep: async (ms) => void waits.push(ms) })).toBe('sent');
    expect(waits).toEqual([2500, 200]);
  });

  it('leaves to the queue a wait longer than a minute, a 429 that goes on, a 5xx and a Discord out of reach', async () => {
    const sleep = async () => {};
    discord.plan([{ status: 429, retryAfter: 120 }]);
    await expect(post(new URL(discord.hook('admin')), 'admin', message, { sleep })).rejects.toThrow(DiscordRetry);
    discord.plan([{ status: 429 }, { status: 429 }, { status: 429 }, { status: 429 }]);
    await expect(post(new URL(discord.hook('admin')), 'admin', message, { sleep })).rejects.toThrow(/rate limited/);
    discord.plan([{ status: 502 }]);
    const e = await post(new URL(discord.hook('admin')), 'admin', message).catch((x: Error) => x);
    expect(e).toBeInstanceOf(DiscordRetry);
    expect(String((e as Error).message)).toBe(`discord admin (${new URL(discord.base).host}): HTTP 502`);
    const gone = await post(new URL('http://127.0.0.1:1/api/webhooks/1/admin-tok3n-SECRET'), 'admin', message).catch((x: Error) => x);
    expect(gone).toBeInstanceOf(DiscordRetry);
    expect(String((gone as Error).message)).not.toContain('tok3n');
    expect(discord.of('admin')).toEqual([]);
  });

  it('drops a message Discord refuses (the webhook deleted), and says so with the host alone', async () => {
    discord.plan([{ status: 404 }]);
    expect(await post(new URL(discord.hook('admin')), 'admin', message)).toBe('dropped');
    expect(logs.at(-1)).toBe(`[store] discord admin (${new URL(discord.base).host}): HTTP 404, message dropped`);
  });
});

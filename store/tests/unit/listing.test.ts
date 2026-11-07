import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { BUILTINS, isBuiltinId, isOwnId, ownLogo } from '../../server/utils/builtin';
import { EMBEDDED_CATEGORIES } from '../../scripts/plugins-publish/publish';
import { CATEGORIES } from '../../server/utils/submissions';
import { asksOf, catalog, home, hueOf, localized, markOf, permissionsOf, search, type Entry } from '../../server/utils/listing';
import { fold, highlight } from '../../shared/search';

// The public pages' catalog without a database: the search made on the server (relevance, accents and case set
// aside), the words of the permissions, the home page's lists, and Escouade's own plugins.
const entry = (over: Partial<Entry>): Entry => ({
  id: 'dev.x.y',
  name: { fr: 'Y', en: 'Y' },
  description: { fr: '', en: '' },
  mark: 'Y',
  hue: 0,
  icon: null,
  logo: null,
  author: { login: 'sam', githubId: 1 },
  own: false,
  categories: ['notifications'],
  keywords: [],
  version: '1.0.0',
  trust: 'unverified',
  installs: 0,
  updatedAt: '2026-10-20T00:00:00Z',
  firstAt: '2026-10-20T00:00:00Z',
  asks: [{ kind: 'none' }],
  ...over,
});

const slack = entry({
  id: 'dev.lmartin.slack',
  name: { fr: 'Slack', en: 'Slack' },
  description: {
    fr: 'Les tickets à tester et les agents qui attendent, dans un salon Slack.',
    en: 'Tickets to test and agents waiting, in a Slack channel.',
  },
  author: { login: 'lea-martin', githubId: 2 },
  keywords: ['chat', 'équipe'],
  installs: 1284,
  asks: [{ kind: 'run' }, { kind: 'net', host: 'hooks.slack.com' }, { kind: 'secret' }],
});
const ntfy = entry({
  id: 'dev.sroux.ntfy',
  name: { fr: 'ntfy', en: 'ntfy' },
  description: { fr: 'Les agents qui attendent, sur ton téléphone.', en: 'Agents waiting, on your phone.' },
  installs: 402,
  updatedAt: '2026-10-27T00:00:00Z',
  firstAt: '2026-10-27T00:00:00Z',
  trust: 'review',
  asks: [{ kind: 'run' }, { kind: 'net', host: 'ntfy.sh' }],
});
const nord = entry({
  id: 'io.github.amoreau.nord',
  name: { fr: 'Nord', en: 'Nord' },
  description: { fr: 'La palette Nord, en clair et en sombre.', en: 'The Nord palette, light and dark.' },
  categories: ['themes'],
  installs: 1120,
  firstAt: '2026-10-12T00:00:00Z',
});
/** One of Escouade's own, a line of the catalogue like any (ESC-353), published before the others. */
const own = (id: string, name: string, categories: Entry['categories']) =>
  entry({
    id: `dev.escouade.${id}`,
    name: { fr: name, en: name },
    author: { login: 'guillaume-gagnaire', githubId: 9 },
    own: true,
    categories,
    trust: 'verified',
    firstAt: '2026-10-01T00:00:00Z',
    asks: [{ kind: 'run' }],
  });
const all = [
  own('claude-code', 'Claude Code', ['harness']),
  own('gemini-cli', 'Gemini CLI', ['harness']),
  own('opencode', 'OpenCode', ['harness']),
  own('bitbucket', 'Bitbucket', ['forges']),
  own('gitea', 'Gitea', ['forges']),
  own('jira', 'Jira', ['ticket-sources']),
  slack,
  ntfy,
  nord,
];

describe('the search', () => {
  it('ignores accents and case, and puts the name before the description', () => {
    const r = search(all, { q: 'ATTENDENT' }).results.map((e) => e.id);
    expect(r).toEqual(['dev.lmartin.slack', 'dev.sroux.ntfy']);
    expect(search(all, { q: 'Telephone' }).results.map((e) => e.id)).toEqual(['dev.sroux.ntfy']);
    expect(search(all, { q: 'equipe' }).results.map((e) => e.id)).toEqual(['dev.lmartin.slack']);
    // A name found whole comes before a word of a description.
    expect(search(all, { q: 'slack' }).results[0]!.id).toBe('dev.lmartin.slack');
  });

  it('wants every word somewhere, and finds Escouade’s own by their name, category or author', () => {
    expect(search(all, { q: 'slack téléphone' }).results).toEqual([]);
    expect(search(all, { q: 'gemini cli' }).results[0]!.id).toBe('dev.escouade.gemini-cli');
    expect(search(all, { q: 'gemini-cli' }).results[0]!.id).toBe('dev.escouade.gemini-cli');
    expect(search(all, { q: 'hébergeurs' }).results.map((e) => e.id)).toContain('dev.escouade.bitbucket');
    expect(search(all, { q: 'escouade', cat: 'themes' }).results).toEqual([]);
    expect(search(all, { q: 'lea' }).results.map((e) => e.id)).toEqual(['dev.lmartin.slack']);
  });

  it('filters by category, trust and what the plugin asks, and counts by category', () => {
    const n = search(all, { cat: 'notifications' });
    expect(n.results.map((e) => e.id)).toEqual(['dev.lmartin.slack', 'dev.sroux.ntfy']);
    expect(n.counts.themes).toBe(1);
    expect(n.counts.harness).toBe(3);
    expect(search(all, { cat: 'notifications', verified: true }).results).toEqual([]);
    expect(search(all, { verified: true }).results.every((e) => e.own)).toBe(true);
    expect(search(all, { noProgram: true }).results.map((e) => e.id)).toEqual(['io.github.amoreau.nord']);
    expect(search(all, { cat: 'notifications', noSecret: true }).results.map((e) => e.id)).toEqual(['dev.sroux.ntfy']);
    expect(search(all, { noNetwork: true, cat: 'harness' }).results).toHaveLength(3);
  });

  it('sorts by installations or by the last update when asked', () => {
    expect(search([slack, ntfy, nord], { sort: 'installs' }).results.map((e) => e.id)).toEqual([slack.id, nord.id, ntfy.id]);
    expect(search([slack, ntfy, nord], { sort: 'recent' }).results[0]!.id).toBe(ntfy.id);
  });

  it('marks the words it found, accents and case aside, in the original text', () => {
    expect(highlight('Les agents qui attendent', 'ATTEND')).toEqual(['Les agents qui ', 'attend', 'ent']);
    expect(highlight('Sur ton téléphone', 'telephone')).toEqual(['Sur ton ', 'téléphone']);
    expect(highlight('Slack', '')).toEqual(['Slack']);
    expect(fold('Équipe-Café')).toBe('equipe cafe');
    expect(highlight('Les agents qui attendent', 'agents  attend')).toEqual(['Les ', 'agents', ' qui ', 'attend', 'ent']);
  });
});

describe('a plugin in words', () => {
  it('says what it asks, the strongest first, with its hosts and secrets', () => {
    const m = {
      run: { command: 'bin/escouade-slack' },
      permissions: { decide: true, network: ['hooks.slack.com'], events: ['ticket.column', 'agent.waiting'], secrets: ['webhook'] },
      contributes: { secrets: [{ key: 'webhook', title: { fr: 'Adresse du webhook', en: 'Webhook address' } }] },
    };
    const p = permissionsOf(m, { 'linux-x64': { attested: true } });
    expect(p.map((x) => x.text.fr)).toEqual([
      'Lance un programme sur ton ordinateur, avec tes droits',
      'Décide des autorisations de tes agents',
      'Reçoit ces événements',
      'Se connecte à',
      'Garde un secret : Adresse du webhook',
    ]);
    expect(p[0]).toMatchObject({ tone: 'strong', mono: 'bin/escouade-slack', detail: { fr: 'compilé par la CI du dépôt, attesté' } });
    expect(p[1]!.tone).toBe('red');
    expect(p[2]!.mono).toBe('ticket.column · agent.waiting');
    expect(asksOf(m)).toEqual([{ kind: 'run' }, { kind: 'decide' }, { kind: 'net', host: 'hooks.slack.com' }, { kind: 'secret' }]);
    expect(permissionsOf({}, null).map((x) => x.text.en)).toEqual(['Asks for nothing: nothing runs, nothing leaves']);
    expect(asksOf({})).toEqual([{ kind: 'none' }]);
  });

  it('reads a text of the manifest in either language', () => {
    expect(localized('Ntfy')).toEqual({ fr: 'Ntfy', en: 'Ntfy' });
    expect(localized({ en: 'Rules' })).toEqual({ fr: 'Rules', en: 'Rules' });
    expect(localized({ en: 'Rules', fr: 'Règles' })).toEqual({ fr: 'Règles', en: 'Rules' });
    expect(markOf('ntfy')).toBe('nt');
    expect(markOf('“Été”')).toBe('Ét');
    expect(hueOf('dev.lmartin.slack') % 30).toBe(0);
  });
});

describe('the home page and Escouade’s own', () => {
  it('shows the verified (Escouade’s own, of every category), the new and the most installed', () => {
    const h = home(all);
    expect(h.verified).toHaveLength(6);
    expect(new Set(h.verified.map((e) => e.categories[0])).size).toBeGreaterThan(2);
    expect(h.fresh.map((e) => e.id).slice(0, 3)).toEqual([ntfy.id, slack.id, nord.id]);
    expect(h.top.map((e) => e.id)).toEqual([slack.id, nord.id, ntfy.id]);
    expect(h.counts['ticket-sources']).toBe(1);
  });

  it('keeps no list of its own: each is a plugin of the store, in the reserve dev.escouade.* (ESC-353)', async () => {
    expect(BUILTINS).toEqual([]);
    expect(await catalog(null)).toEqual([]);
    expect(isBuiltinId('dev.escouade.github')).toBe(false);
    expect(isOwnId('dev.escouade.github')).toBe(true);
    expect(isOwnId('io.github.me.github')).toBe(false);
  });

  it('gives each folder of plugins/ a category of the store to be first published in', () => {
    const root = fileURLToPath(new URL('../../../plugins/', import.meta.url));
    const published = ['discord', 'high-contrast', 'slack', 'team-rules'];
    for (const d of readdirSync(root).filter((x) => existsSync(join(root, x, 'escouade-plugin.json')))) {
      const m = JSON.parse(readFileSync(join(root, d, 'escouade-plugin.json'), 'utf8'));
      if (published.includes(d)) continue;
      // A module that reads its manifest embeds it: a key added there changes its fingerprint (docs/PLUGINS.md §8.29).
      // Its first version kept the tool's categories; a later one may carry a `store` key (its pages, ESC-366), as
      // it has a new fingerprint anyway.
      const code = existsSync(join(root, d, 'src/lib.rs'))
        ? readFileSync(join(root, d, 'src/lib.rs'), 'utf8').split('#[cfg(test)]')[0]!
        : '';
      const embeds = /\(MANIFEST\)/.test(code);
      if (!embeds) expect(m.store, d).toEqual(expect.anything());
      const categories: string[] = m.store?.categories ?? (embeds ? EMBEDDED_CATEGORIES[m.id]! : []);
      expect(categories?.length, d).toBeGreaterThan(0);
      for (const c of categories) expect(CATEGORIES, d).toContain(c);
    }
  });
});

describe('the logos of Escouade’s own (PLG-13)', () => {
  it('a brand’s mark only where its rules allow it without asking (read on 2026-10-29), else its category’s glyph', () => {
    expect(ownLogo('dev.escouade.github', ['ticket-sources', 'forges'])).toEqual({ brand: 'github' });
    expect(ownLogo('dev.escouade.gitea', ['forges'])).toEqual({ brand: 'gitea' });
    expect(ownLogo('dev.escouade.opencode', ['harness'])).toEqual({ brand: 'opencode' });
    expect(ownLogo('dev.escouade.gitlab', ['ticket-sources', 'forges'])).toEqual({ category: 'forge' });
    expect(ownLogo('dev.escouade.jira', ['ticket-sources'])).toEqual({ category: 'source' });
    expect(ownLogo('dev.escouade.claude-code', ['harness'])).toEqual({ category: 'harness' });
  });

  it('draws nothing for the others, nor for a category without a glyph', () => {
    expect(ownLogo('dev.escouade.high-contrast', ['themes'])).toBeNull();
    expect(ownLogo('dev.escouade.isola', ['tools'])).toBeNull();
    expect(ownLogo('io.github.me.github', ['forges'])).toBeNull();
  });
});

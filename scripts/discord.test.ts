// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { LIMIT, discordMessages, postMessages } from './discord.mjs';

const RELEASE = 'https://github.com/guillaume-gagnaire/escouade/releases/tag/v1.8.5';

describe('discord messages', () => {
  it('heads the notes with the version and a link to its release, in a single message when they fit', () => {
    const notes = '### Corrections\n\n- Les liens.';
    expect(discordMessages(notes, 'v1.8.5')).toEqual([
      `## Escouade 1.8.5\n-# [Release sur GitHub](<${RELEASE}>)\n\n### Corrections\n\n- Les liens.`,
    ]);
    expect(discordMessages(notes, '1.8.5')).toEqual(discordMessages(notes, 'v1.8.5'));
  });

  it('words the link in English for the English channel', () => {
    expect(discordMessages('### Fixes\n\n- The links.', 'v1.8.5', 'en')).toEqual([
      `## Escouade 1.8.5\n-# [Release on GitHub](<${RELEASE}>)\n\n### Fixes\n\n- The links.`,
    ]);
  });

  // ESC-586: a beta is never announced as the stable version.
  it('says a beta is one, after its version and before its notes, and how to go back to the stable version', () => {
    const fr = discordMessages('### Ajouts\n\n- Le serveur d’équipe.', 'v2.3.0-beta.1');
    expect(fr[0]).toMatch(
      /^## Escouade 2\.3\.0-beta\.1 \(bêta\)\n-# \[Release sur GitHub\]\(<https:\/\/github\.com\/guillaume-gagnaire\/escouade\/releases\/tag\/v2\.3\.0-beta\.1>\)\n\n> \*\*Version bêta\.\*\*/,
    );
    expect(fr[0]).toContain('**Revenir à la 2.2**');
    expect(fr[0]).toContain('« Recevoir les versions bêta »');
    expect(fr[0].indexOf('Revenir à la 2.2')).toBeLessThan(fr[0].indexOf('### Ajouts'));
    const en = discordMessages('### Added\n\n- The team server.', 'v2.3.0-beta.1', 'en');
    expect(en[0]).toMatch(/^## Escouade 2\.3\.0-beta\.1 \(beta\)\n/);
    expect(en[0]).toContain('**Going back to 2.2**');
    expect(en[0]).toContain('“Receive beta versions”');
    // A stable version says neither.
    expect(discordMessages('- x', 'v2.3.0').join('\n')).not.toMatch(/bêta|Revenir/);
  });

  it('splits long notes between lines, every message within Discord’s limit, nothing lost', () => {
    const items = Array.from({ length: 60 }, (_, i) => `- Entrée ${i} : ${'mot '.repeat(20).trim()}.`);
    const notes = `### Ajouts\n\n${items.join('\n')}`;
    const messages = discordMessages(notes, '1.8.0');
    expect(messages.length).toBeGreaterThan(1);
    for (const m of messages) {
      expect(m.length).toBeLessThanOrEqual(LIMIT);
      expect(m).toBe(m.trim());
    }
    for (const item of items) expect(messages.filter((m) => m.split('\n').includes(item))).toHaveLength(1);
  });

  it('does not leave a heading alone at the end of a message, it opens the next one', () => {
    // A last entry one character longer each time: one of them puts the cut right after « ### Corrections ».
    const filler = Array.from({ length: 18 }, (_, i) => `- Ajout ${i} : ${'mot '.repeat(22).trim()}.`).join('\n');
    for (let n = 1; n <= 150; n++) {
      const notes = `### Ajouts\n\n${filler}\n- ${'x'.repeat(n)}\n\n### Corrections\n\n- La correction.`;
      const messages = discordMessages(notes, '1.8.0');
      for (const m of messages) expect(m.split('\n').at(-1)).not.toMatch(/^#/);
      expect(messages.some((m) => m.includes('### Corrections\n\n- La correction.'))).toBe(true);
    }
  });

  it('cuts a line longer than the limit between words', () => {
    const words = Array.from({ length: 600 }, (_, i) => `mot${i}`);
    const messages = discordMessages(`- ${words.join(' ')}`, '1.8.0');
    for (const m of messages) expect(m.length).toBeLessThanOrEqual(LIMIT);
    expect(
      messages
        .join(' ')
        .split(/\s+/)
        .filter((w) => w.startsWith('mot')),
    ).toEqual(words);
  });
});

/** A fetch that records each request and answers with the next of `replies` (204 when they run out). */
function fakeFetch(replies: { status: number; body?: unknown }[] = []) {
  const calls: { url: string; body: { content: string; allowed_mentions: unknown } }[] = [];
  const fetch = async (url: string, init: { body: string }) => {
    calls.push({ url, body: JSON.parse(init.body) });
    const r = replies.shift() ?? { status: 200, body: {} };
    return new Response(JSON.stringify(r.body ?? {}), { status: r.status });
  };
  return { calls, fetch };
}

describe('posting to the webhook', () => {
  const WEBHOOK = 'https://discord.com/api/webhooks/1/secret';

  it('posts each message in order, waiting for Discord, and mentions no one', async () => {
    const { calls, fetch } = fakeFetch();
    await postMessages(WEBHOOK, ['un', 'deux'], { fetch, sleep: async () => {} });
    expect(calls.map((c) => c.body.content)).toEqual(['un', 'deux']);
    expect(calls.every((c) => c.url === `${WEBHOOK}?wait=true`)).toBe(true);
    expect(calls.every((c) => JSON.stringify(c.body.allowed_mentions) === '{"parse":[]}')).toBe(true);
  });

  it('keeps a query the webhook already has', async () => {
    const { calls, fetch } = fakeFetch();
    await postMessages(`${WEBHOOK}?thread_id=42`, ['un'], { fetch, sleep: async () => {} });
    expect(calls[0].url).toBe(`${WEBHOOK}?thread_id=42&wait=true`);
  });

  it('waits as long as Discord says when rate limited, then posts the same message again', async () => {
    const { calls, fetch } = fakeFetch([{ status: 429, body: { retry_after: 1.5 } }]);
    const waits: number[] = [];
    await postMessages(WEBHOOK, ['un', 'deux'], { fetch, sleep: async (ms: number) => void waits.push(ms) });
    expect(waits).toEqual([1500]);
    expect(calls.map((c) => c.body.content)).toEqual(['un', 'un', 'deux']);
  });

  it('stops at a refusal, saying Discord’s answer but never the webhook and its token', async () => {
    const { calls, fetch } = fakeFetch([{ status: 400, body: { message: 'Invalid Form Body' } }]);
    const error = await postMessages(WEBHOOK, ['un', 'deux'], { fetch, sleep: async () => {} }).catch((e) => e);
    expect(error).toBeInstanceOf(Error);
    expect(error.message).toContain('400');
    expect(error.message).toContain('Invalid Form Body');
    expect(error.message).not.toContain('secret');
    expect(calls).toHaveLength(1);
  });
});

// The two webhooks as Escouade sends them, checked on the manifest alone: which facts they answer, the message they
// post, and what they may not do. `node --test` in this folder (it reads its manifest there), no dependency; the bench of Escouade plays the rest.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const manifest = JSON.parse(readFileSync('escouade-plugin.json', 'utf8'));
const hooks = manifest.contributes.webhooks;

/** A value of the fact by its dotted path (`data.to`). */
const at = (fact, path) => path.split('.').reduce((v, k) => v?.[k], fact);
/** The `{{…}}` of a template filled from the fact, as Escouade fills them. */
const fill = (value, fact) =>
  typeof value === 'string'
    ? value.replace(/\{\{\s*([\w.]+)\s*\}\}/g, (_, p) => String(at(fact, p) ?? ''))
    : Array.isArray(value)
      ? value.map((v) => fill(v, fact))
      : value && typeof value === 'object'
        ? Object.fromEntries(Object.entries(value).map(([k, v]) => [k, fill(v, fact)]))
        : value;
/** The bodies the webhooks post for a fact. */
const posted = (fact) =>
  hooks
    .filter((h) => h.events.includes(fact.event) && Object.entries(h.when ?? {}).every(([p, v]) => at(fact, p) === v))
    .map((h) => fill(h.body, fact));

const project = { name: 'Banc' };

test('a ticket moved to test is posted, and nothing for another column', () => {
  const ticket = { key: 'APP-7', title: 'Le panier' };
  assert.deepEqual(posted({ event: 'ticket.column', project, ticket, data: { to: 'review' } }), [
    { content: '🧪 **APP-7** to test: Le panier (Banc)', allowed_mentions: { parse: [] } },
  ]);
  assert.deepEqual(posted({ event: 'ticket.column', project, ticket, data: { to: 'done' } }), []);
});

test('an agent that waits is posted, and nothing while it works', () => {
  const agent = { name: 'Mila' };
  assert.deepEqual(posted({ event: 'agent.status', project, agent, data: { status: 'waiting' } }), [
    { content: '✋ **Mila** waits for you (Banc)', allowed_mentions: { parse: [] } },
  ]);
  assert.deepEqual(posted({ event: 'agent.status', project, agent, data: { status: 'working' } }), []);
});

test('a title never pings the channel: mentions are off', () => {
  const ticket = { key: 'APP-8', title: '@everyone look' };
  const [body] = posted({ event: 'ticket.column', project, ticket, data: { to: 'review' } });
  assert.deepEqual(body.allowed_mentions, { parse: [] });
});

test('it asks only what it uses: its events, its secret, Discord', () => {
  assert.deepEqual([...new Set(hooks.flatMap((h) => h.events))].sort(), [...manifest.permissions.events].sort());
  for (const h of hooks) assert.equal(h.url, '{{secrets.webhook}}');
  assert.deepEqual(manifest.permissions.secrets, ['webhook']);
  assert.deepEqual(manifest.permissions.network, ['discord.com']);
});

// The plugin's handlers, driven by the SDK's fake Escouade, in memory, with a fake Slack. The bench plays the program.
import assert from 'node:assert/strict';
import { mock, test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { channelOf, slack } from '../src/plugin.ts';

const TOKEN = 'xoxb-1-test';

/** A fake Slack in place of fetch: what was posted, and its answer. */
function fakeSlack(answer: { ok: boolean; error?: string } = { ok: true }) {
  const posted: Array<{ url: string; auth: string; body: { channel: string; text: string } }> = [];
  mock.restoreAll();
  mock.method(globalThis, 'fetch', async (url: string, init: RequestInit) => {
    const headers = init.headers as Record<string, string>;
    posted.push({ url, auth: headers.authorization, body: JSON.parse(String(init.body)) });
    return new Response(JSON.stringify(answer));
  });
  return { posted };
}

test('a ticket to test is posted in its project’s channel, with the token from the keychain', async () => {
  const s = fakeSlack();
  const escouade = connect(slack({ channels: { Banc: '#banc-qa' } }), { secrets: { token: TOKEN } });
  await escouade.initialize();
  await escouade.fact('ticket.column', { ticket: { key: 'APP-7', title: 'Le panier' } });
  assert.deepEqual(s.posted, [
    {
      url: 'https://slack.com/api/chat.postMessage',
      auth: `Bearer ${TOKEN}`,
      body: { channel: 'banc-qa', text: ':test_tube: *APP-7* to test: Le panier' },
    },
  ]);
  assert.deepEqual(escouade.secretsAsked, ['token']);
  assert.deepEqual(escouade.logs, [{ level: 'info', message: 'posted in #banc-qa' }]);
});

test('an agent waiting goes to the channel named after its project', async () => {
  const s = fakeSlack();
  const escouade = connect(slack({ channels: {} }), { secrets: { token: TOKEN } });
  await escouade.initialize();
  await escouade.fact('agent.status', {
    project: { name: 'Café Société' },
    agent: { name: 'esc-7' },
    data: { status: 'waiting', was: 'running' },
  });
  assert.equal(s.posted[0].body.channel, 'escouade-cafe-societe');
  assert.equal(s.posted[0].body.text, ':raised_hand: *esc-7* waits for you');
});

test('the other moves and statuses say nothing', async () => {
  const s = fakeSlack();
  const escouade = connect(slack({ channels: {} }), { secrets: { token: TOKEN } });
  await escouade.initialize();
  await escouade.fact('ticket.column', { data: { from: 'todo', to: 'doing' } });
  await escouade.fact('agent.status', { data: { status: 'running', was: 'idle' } });
  assert.deepEqual(s.posted, []);
  assert.deepEqual(escouade.secretsAsked, []);
});

test('without a token, or with one that is not a bot’s, nothing is sent and the journal says why', async () => {
  for (const token of [null, 'xoxp-1-a-user-token']) {
    const s = fakeSlack();
    const escouade = connect(slack({ channels: {} }), { secrets: { token } });
    await escouade.initialize();
    await escouade.fact('ticket.column');
    assert.deepEqual(s.posted, []);
    assert.equal(escouade.logs[0].level, 'warn');
    assert.ok(!JSON.stringify(escouade.logs).includes('xoxp-1'), 'the secret is never logged');
  }
});

test('a refusal of Slack is in the journal, without the token', async () => {
  const s = fakeSlack({ ok: false, error: 'not_in_channel' });
  const escouade = connect(slack({ channels: {} }), { secrets: { token: TOKEN } });
  await escouade.initialize();
  await escouade.fact('ticket.column');
  assert.deepEqual(escouade.logs, [{ level: 'warn', message: 'Slack refused the message in #escouade-banc: not_in_channel' }]);
});

test('a channel is written as Slack names them', () => {
  assert.equal(channelOf('Mon Projet (v2)', {}), 'escouade-mon-projet-v2');
  assert.equal(channelOf('Escouade', { Escouade: '#escouade-dev' }), 'escouade-dev');
  assert.equal(channelOf('***', {}), 'escouade-project');
});

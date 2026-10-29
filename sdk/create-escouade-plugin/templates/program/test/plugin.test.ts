// The plugin's handlers, driven by the SDK's fake Escouade, in memory. `npm run bench` plays the real program.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { plugin } from '../src/plugin.ts';

test('it speaks version 1 of the protocol', async () => {
  const escouade = connect(plugin);
  const hello = await escouade.initialize();
  assert.equal(hello.protocolVersion, 1);
});

test('a ticket moved to test is said in the journal', async () => {
  const escouade = connect(plugin);
  await escouade.initialize();
  await escouade.fact('ticket.column', { ticket: { key: 'APP-7', column: 'test' } });
  assert.deepEqual(escouade.logs, [{ level: 'info', message: 'APP-7 is ready to test (Banc)' }]);
});

test('a force push is refused, the rest left to the user', async () => {
  const escouade = connect(plugin);
  await escouade.initialize();
  const push = await escouade.decide({ request: { input: { command: 'git push --force origin main' } } });
  assert.equal(push.decision, 'deny');
  assert.equal((await escouade.decide()).decision, 'ask');
});

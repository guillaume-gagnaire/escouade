// Prime Agent's manifest as the generic ACP plugin reads it: one harness, described over ACP (`prime-agent --mode acp
// --no-session`), found by its name, with the one mode it has, Bypass, said in clear; no read-only mode; its official
// installers and its update. `node --test` in this folder.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const manifest = JSON.parse(readFileSync('escouade-plugin.json', 'utf8'));
const [prime] = manifest.contributes.harnesses;

test('it installs only on an app that refuses its agents without Bypass allowed (2.2.2, ESC-489)', () => {
  assert.deepEqual(manifest.escouade, { api: '1', min: '2.2.2' });
});

test('it describes one harness over ACP, without a program of its own', () => {
  assert.equal(manifest.id, 'dev.escouade.prime-agent');
  assert.equal(manifest.run, undefined);
  assert.equal(manifest.permissions, undefined);
  assert.equal(manifest.contributes.harnesses.length, 1);
  assert.equal(prime.id, 'prime-agent');
  assert.deepEqual(prime.locate, { names: ['prime-agent'] });
});

test('its sessions belong to the connection: none is left in its daemon once the agent is gone', () => {
  // Without `--no-session`, a session of its ACP is saved and stays in its daemon (its worker, its Python kernel)
  // after the client left; with it, it ends with the connection (at once when the input closes, 30 s after a client
  // killed). Prime Agent 0.10.3 cannot load a session over ACP anyway (`loadSession: false`).
  assert.deepEqual(prime.acp.args, ['--mode', 'acp', '--no-session']);
});

test('its one mode is Bypass, and its meaning says in both languages that nothing asks and nothing is sandboxed', () => {
  assert.deepEqual(
    prime.caps.modes.map((m) => m.id),
    ['bypassPermissions'],
  );
  const { fr, en } = prime.caps.modes[0].meaning;
  assert.match(fr, /ne demande jamais/);
  assert.match(fr, /sans bac à sable/);
  assert.match(en, /never asks/);
  assert.match(en, /no sandbox/);
  // Prime Agent has no ACP mode (`session/set_mode` is « Method not found »): none is set, none is pretended.
  assert.equal(prime.acp.modes, undefined);
});

test('no discussion is open to it: it has no mode that changes nothing', () => {
  assert.notEqual(prime.caps.readOnly, true);
  assert.equal(prime.acp.readOnlyMode, undefined);
});

test('its files are its own: no private project opens to it', () => {
  // Offered `fs/read_text_file` and `fs/write_text_file`, it never used them: its Python writes the files itself.
  assert.equal(prime.acp.fs, false);
  assert.notEqual(prime.acp.fenceOnPermissions, true);
});

test('its official installers, as its README gives them, and its update', () => {
  assert.deepEqual(prime.install, {
    windows: 'irm https://app.primeintellect.ai/prime-agent/install.ps1 | iex',
    unix: 'curl -fsSL https://app.primeintellect.ai/prime-agent/install.sh | sh',
  });
  assert.equal(prime.updateCommand, 'prime-agent update');
});

test('its one tool, a cell of Python, is shown as Python; its model by its configuration option; its questions in text', () => {
  assert.deepEqual(prime.acp.tools, { 'Python cell': 'Python' });
  assert.equal(prime.acp.modelOption, 'model');
  assert.equal(prime.acp.questionsInText, true);
  // Its models are each user's own providers: none is declared, its default is the one it runs.
  assert.equal(prime.models, undefined);
  // Its reasoning effort (`thought_level`) is not set by Escouade's client: not claimed.
  assert.notEqual(prime.caps.effort, true);
});

test('one account: its daemon, shared by every session of the user, keeps the folder it started with', () => {
  // PRIME_AGENT_CODING_AGENT_DIR moves its folder, but a session opened on a daemon already running takes the daemon's
  // folder (tried): two accounts would mix. None is declared.
  assert.equal(prime.accountVar, undefined);
  assert.deepEqual(prime.tested, { oldest: '0.10.3', newest: '0.10.3' });
});

test('its effort goes by its option thought_level, there only for a model that reasons (ESC-491)', () => {
  assert.deepEqual(prime.efforts, ['low', 'medium', 'high']);
  assert.deepEqual(prime.acp.efforts, { low: 'low', medium: 'medium', high: 'high' });
  assert.equal(prime.acp.effortOption, undefined, 'the option of the category thought_level');
});

test('a turn it begins by itself ends by the marks of its stream; it loads no session, so none is resumed (ESC-491)', () => {
  assert.equal(prime.acp.turnEnd.length, 2);
  assert.deepEqual(prime.acp.turnEnd[0], { '/_meta/ai.primeintellect.prime-agent/phase': 'terminalQuiescence' });
  // --no-session: its sessions go with the connection, nothing to launch again with.
  assert.equal(prime.acp.resume, undefined);
  assert.equal(prime.acp.usage, undefined, 'it tells no tokens');
});

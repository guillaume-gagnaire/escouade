// Unharnessed's manifest as the generic ACP plugin reads it: one harness, described over ACP (`unharnessed --mode acp`),
// found by its name, with the one mode it has, Bypass, said in clear; no read-only mode, no installer, no path to run.
// `node --test` in this folder.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const manifest = JSON.parse(readFileSync('escouade-plugin.json', 'utf8'));
const [unharnessed] = manifest.contributes.harnesses;

test('it describes one harness over ACP, without a program of its own', () => {
  assert.equal(manifest.run, undefined);
  assert.equal(manifest.permissions, undefined);
  assert.equal(manifest.contributes.harnesses.length, 1);
  assert.equal(manifest.id, 'dev.escouade.unharnessed');
  assert.deepEqual(unharnessed.acp.args, ['--mode', 'acp']);
  assert.deepEqual(unharnessed.locate, { names: ['unharnessed'] });
});

test('its one mode is Bypass, and its meaning says in both languages that nothing asks and nothing is sandboxed', () => {
  assert.deepEqual(
    unharnessed.caps.modes.map((m) => m.id),
    ['bypassPermissions'],
  );
  const { fr, en } = unharnessed.caps.modes[0].meaning;
  assert.match(fr, /ne demande jamais/);
  assert.match(fr, /sans bac à sable/);
  assert.match(en, /never asks/);
  assert.match(en, /no sandbox/);
  // Unharnessed has no ACP mode: none is set, none is pretended.
  assert.equal(unharnessed.acp.modes, undefined);
});

test('no discussion is open to it: it has no mode that changes nothing', () => {
  assert.notEqual(unharnessed.caps.readOnly, true);
  assert.equal(unharnessed.acp.readOnlyMode, undefined);
});

test('its files are its own: no private project opens to it', () => {
  assert.equal(unharnessed.acp.fs, false);
  assert.notEqual(unharnessed.acp.fenceOnPermissions, true);
});

test('nothing installs it: it has no official installer, its documentation says how to get it', () => {
  assert.equal(unharnessed.install, undefined);
  assert.equal(unharnessed.updateCommand, undefined);
});

test('its one tool, a cell of Python, is shown as Python; its model by its configuration option; its questions in text', () => {
  assert.deepEqual(unharnessed.acp.tools, { 'Python cell': 'Python' });
  assert.equal(unharnessed.acp.modelOption, 'model');
  assert.equal(unharnessed.acp.questionsInText, true);
  // Its models are each user's own providers: none is declared, its default is the one it runs.
  assert.equal(unharnessed.models, undefined);
});

test('an account is a folder of its own, and the version tried is the one it holds', () => {
  assert.equal(unharnessed.accountVar, 'UNHARNESSED_CODING_AGENT_DIR');
  assert.deepEqual(unharnessed.tested, { oldest: '0.9.8', newest: '0.9.8' });
});

test('it installs only on an app that refuses its agents without Bypass allowed (2.2.2, ESC-489)', () => {
  assert.deepEqual(manifest.escouade, { api: '1', min: '2.2.2' });
});

test('a session is taken back by its process launched with --resume, under the id it saves it under (ESC-491)', () => {
  assert.deepEqual(unharnessed.acp.resume, {
    args: ['--resume', '{session}'],
    session: '/_meta/unharnessed/saved/sessionId',
  });
});

test('a turn it begins by itself (a wake) ends by the marks of its stream; its context is read where it tells it (ESC-491)', () => {
  assert.deepEqual(unharnessed.acp.turnEnd, [
    { '/_meta/unharnessed/phase': 'terminalQuiescence' },
    { '/_meta/unharnessed/phase': 'responseBoundary', '/_meta/unharnessed/terminalQuiescenceExpected': false },
  ]);
  assert.deepEqual(unharnessed.acp.usage, {
    context: '/_meta/unharnessed/usage/tokens',
    window: '/_meta/unharnessed/usage/window',
  });
  // Its effort was not tried: no table, nothing set.
  assert.equal(unharnessed.acp.efforts, undefined);
});

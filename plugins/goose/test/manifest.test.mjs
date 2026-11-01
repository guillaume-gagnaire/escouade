// Goose's manifest as the generic ACP plugin reads it: one harness, described over ACP, found by its name, its modes
// each with its ACP mode, read-only by its mode that calls no tool, no path to run. `node --test` in this folder.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const manifest = JSON.parse(readFileSync('escouade-plugin.json', 'utf8'));
const [goose] = manifest.contributes.harnesses;

test('it describes one harness over ACP, without a program of its own', () => {
  assert.equal(manifest.run, undefined);
  assert.equal(manifest.contributes.harnesses.length, 1);
  assert.deepEqual(goose.acp.args, ['acp']);
  assert.deepEqual(goose.locate, { names: ['goose'] });
});

test('each mode of Escouade has its mode of Goose and its meaning in both languages', () => {
  for (const mode of goose.caps.modes) {
    assert.ok(goose.acp.modes[mode.id], mode.id);
    assert.ok(mode.meaning.fr && mode.meaning.en, mode.id);
  }
  assert.equal(goose.caps.readOnly, true);
  assert.equal(goose.acp.readOnlyMode, 'chat');
});

test('its files are its own: no private project opens to it', () => {
  assert.equal(goose.acp.fs, false);
});

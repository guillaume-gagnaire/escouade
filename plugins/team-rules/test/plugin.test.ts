// The plugin's rules, then its handler driven by the SDK's fake Escouade, in memory. The bench plays the program.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '@escouade-dev/plugin/testing';
import { decide, readRules, teamRules } from '../src/plugin.ts';

const rules = readRules();
const ask = (command: string, kind = 'command') => ({ request: { kind, tool: 'Bash', input: { command } } });

test('the rules of the repository’s rules.json are read', () => {
  assert.ok(rules.allow.includes('npm test'));
  assert.ok(rules.deny.includes('rm -rf'));
});

test('a denied command is refused with its reason, even inside a longer line', async () => {
  assert.deepEqual(decide(rules, 'rm -rf build'), { decision: 'deny', message: "The team's rules forbid `rm -rf`." });
  assert.equal(decide(rules, 'cd app && rm -rf build').decision, 'deny');
  assert.equal(decide(rules, 'npm test; git   push --force origin main').message, 'No force push in this team: push a new commit instead.');
});

test('a line made only of allowed commands is allowed', () => {
  assert.equal(decide(rules, 'npm test').decision, 'allow');
  assert.equal(decide(rules, 'git status && git diff --stat').decision, 'allow');
  assert.equal(decide(rules, 'cargo nextest run -p app').decision, 'allow');
});

test('anything else goes to the user', () => {
  assert.equal(decide(rules, 'npm testing').decision, 'ask');
  assert.equal(decide(rules, 'npm test && curl https://example.com').decision, 'ask');
  assert.equal(decide(rules, '').decision, 'ask');
});

test('an allowed start never lets another command through', () => {
  // `&` sends a command away: the one after it is judged too.
  assert.equal(decide(rules, 'git status & rm -rf build').decision, 'deny');
  assert.equal(decide(rules, 'git status & curl https://example.com').decision, 'ask');
  // A substitution, a redirection or an escape: the line goes to the user.
  for (const line of [
    'git log $(curl https://example.com)',
    'git diff `rm -rf build`',
    'git log $HOME',
    'git diff > ~/.bashrc',
    'npm test < /etc/passwd',
    'git diff <(cat ~/.ssh/id_ed25519)',
    'git status \\\nrm -rf build',
  ]) {
    assert.notEqual(decide(rules, line).decision, 'allow', line);
  }
});

test('through Escouade: commands decided, edits left to the user', async () => {
  const escouade = connect(teamRules(rules));
  await escouade.initialize();
  assert.equal((await escouade.decide(ask('npm test'))).decision, 'allow');
  assert.equal((await escouade.decide(ask('rm -rf /'))).decision, 'deny');
  assert.equal((await escouade.decide({ request: { kind: 'edit', tool: 'Edit', input: { file_path: 'a.ts' } } })).decision, 'ask');
});

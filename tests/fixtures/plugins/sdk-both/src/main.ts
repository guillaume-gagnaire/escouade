// A plugin written for @escouade-dev/plugin as before ESC-321 (createPlugin, onEvent, decidePermission, secret,
// settings, log, start, readFileSync of a file of its folder, the standard fetch), plus p.file(). The runtime's tests
// play its committed bundle by Node (run.command) and in WASM (run.js), the same scenario on both transports
// (src-tauri/src/plugins_wasm_tests.rs); sdk/plugin/src/both.test.ts checks dist/plugin.mjs is the build of this file.
//
//   npx esbuild tests/fixtures/plugins/sdk-both/src/main.ts --bundle --platform=node --format=esm --target=node20 \
//     --outfile=tests/fixtures/plugins/sdk-both/dist/plugin.mjs
import { readFileSync } from 'node:fs';
import { createPlugin, RpcError } from '../../../../../sdk/plugin/src/index';

const rules = JSON.parse(readFileSync(new URL('../rules.json', import.meta.url), 'utf8')) as { endpoint: string; deny: string };
const plugin = createPlugin();

// A plugin in WASM reaches the folder of an agent and runs a declared program (ESC-333): the test gives it the agent.
plugin.handle('probe/host', async (params, p) => {
  const { agent, outside } = params as { agent: string; outside: string };
  const code = (e: unknown) => (e instanceof RpcError ? e.code : -1);
  const text = await p.fs.text({ agent, root: 'agent', path: 'notes.txt' });
  await p.fs.write({ agent, root: 'agent', path: 'written.txt', text: `${text}!` });
  const names = (await p.fs.list({ agent, root: 'agent', path: '.' })).entries.map((e) => e.name);
  const refused = await p.fs.read({ agent, path: outside }).then(() => 0, code);
  let out = '';
  const program = await p.runProgram({ command: 'git', args: ['--version'] }, (data) => (out += data));
  const exit = await program.exited;
  const denied = await p.runProgram({ command: 'git', args: ['status'] }).then(() => 0, code);
  return { text, names, refused, out, exit, denied };
});

plugin.onEvent('ticket.column', async (fact, p) => {
  const words = await p.file('data/words.txt');
  const refused = await p.file('../escouade-plugin.json').then(
    () => 0,
    (e: unknown) => (e instanceof RpcError ? e.code : -1),
  );
  const missing = await p.file('nope.txt').then(
    () => 0,
    (e: unknown) => (e instanceof RpcError ? e.code : -1),
  );
  const token = await p.secret('token');
  const res = await fetch(`${rules.endpoint}/hook`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', authorization: `Bearer ${token}` },
    body: JSON.stringify({ ticket: fact.ticket?.key, words: words.trim().split('\n'), refused, missing }),
  });
  const answer = (await res.json()) as { ok: boolean };
  p.log.info(`posted ${res.status} ok=${answer.ok}`);
});

plugin.decidePermission(({ request }) => {
  const command = typeof request.input?.command === 'string' ? request.input.command : '';
  return command.includes(rules.deny) ? { decision: 'deny', message: `no ${rules.deny} here` } : { decision: 'ask' };
});

void plugin.start();

#!/usr/bin/env node
// Stand-in of the team rules example of PLG-10 (ESC-182, the escouade-plugins repository): a command
// that starts with one of `deny` of rules.json is denied, one of `allow` allowed; anything else goes
// to the user (`ask`). Read once, at start: a decision has 500 ms.
import { readFileSync } from 'node:fs';
import process from 'node:process';

const rules = JSON.parse(readFileSync(new URL('./rules.json', import.meta.url), 'utf8'));
const send = (msg) => process.stdout.write(JSON.stringify(msg) + '\n');

function decide(params) {
  const command = String(params?.request?.input?.command ?? '').trim();
  const denied = rules.deny.find((r) => command.startsWith(r));
  if (denied) return { decision: 'deny', message: `The team's rules forbid \`${denied}\`.` };
  if (rules.allow.some((r) => command === r || command.startsWith(r + ' '))) return { decision: 'allow' };
  return { decision: 'ask' };
}

function handle(msg) {
  const { id, method, params } = msg;
  switch (method) {
    case 'initialize':
      return send({ jsonrpc: '2.0', id, result: { protocolVersion: 1, capabilities: {} } });
    case 'shutdown':
      send({ jsonrpc: '2.0', id, result: null });
      return setTimeout(() => process.exit(0), 10);
    case 'permissions/decide':
      return send({ jsonrpc: '2.0', id, result: decide(params) });
    default:
      if (id !== undefined && method !== undefined) {
        send({ jsonrpc: '2.0', id, error: { code: -32601, message: `unknown method ${method}` } });
      }
  }
}

let buffer = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (chunk) => {
  buffer += chunk;
  let nl;
  while ((nl = buffer.indexOf('\n')) >= 0) {
    const line = buffer.slice(0, nl).trim();
    buffer = buffer.slice(nl + 1);
    if (!line) continue;
    const msg = JSON.parse(line);
    for (const m of Array.isArray(msg) ? msg : [msg]) handle(m);
  }
});
process.stdin.on('end', () => process.exit(0));

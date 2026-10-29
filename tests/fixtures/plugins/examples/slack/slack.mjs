#!/usr/bin/env node
// Stand-in of the Slack example of PLG-10 (ESC-182, the escouade-plugins repository): a fact posts a
// line to the Slack webhook kept in the keychain (`secrets/get webhook`); without one, it says so in
// its journal and sends nothing.
import process from 'node:process';

const asked = new Map();
let next = 1;
const send = (msg) => process.stdout.write(JSON.stringify(msg) + '\n');
const log = (message) => send({ jsonrpc: '2.0', method: 'log', params: { level: 'info', message } });

function ask(method, params) {
  const id = `slack-${next++}`;
  return new Promise((resolve) => {
    asked.set(id, resolve);
    send({ jsonrpc: '2.0', id, method, params });
  });
}

const TEXT = {
  'ticket.column': (f) => `${f.ticket?.key ?? ''} ${f.ticket?.title ?? ''} → ${f.data?.to ?? ''}`,
  'agent.status': (f) => (f.data?.status === 'waiting' ? `${f.agent?.name ?? 'An agent'} waits for you (${f.project?.name ?? ''})` : null),
};

async function fact(f) {
  const text = TEXT[f.name]?.(f);
  if (!text) return;
  const { result: webhook } = await ask('secrets/get', { key: 'webhook' });
  if (typeof webhook !== 'string' || !webhook.startsWith('https://hooks.slack.com/')) {
    log('no Slack webhook kept: nothing sent');
    return;
  }
  try {
    const res = await fetch(webhook, { method: 'POST', body: JSON.stringify({ text }) });
    log(`posted to Slack: ${res.status}`);
  } catch (e) {
    process.stderr.write(`slack: ${e.message}\n`);
  }
}

function handle(msg) {
  if (msg.method === undefined) {
    asked.get(msg.id)?.(msg);
    asked.delete(msg.id);
    return;
  }
  const { id, method, params } = msg;
  switch (method) {
    case 'initialize':
      return send({ jsonrpc: '2.0', id, result: { protocolVersion: 1, capabilities: {} } });
    case 'shutdown':
      send({ jsonrpc: '2.0', id, result: null });
      return setTimeout(() => process.exit(0), 10);
    case 'events/fact':
      return void fact(params ?? {});
    default:
      if (id !== undefined) send({ jsonrpc: '2.0', id, error: { code: -32601, message: `unknown method ${method}` } });
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

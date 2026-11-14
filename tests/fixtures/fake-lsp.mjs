#!/usr/bin/env node
// A false language server (ESC-446), for the tests of `src-tauri/src/lsp`: the base protocol of LSP on stdin and
// stdout (`Content-Length`), and what the tests make it do.
//
// - `initialize` → its capabilities; `shutdown` → null; `exit` → ends (0).
// - `$/echo` (request) → its params as the result.
// - `$/crash` → ends at once, with the code 3.
// - `$/child` → starts a child that sleeps, and gives its pid (the Job ends it with the server).
// - `$/huge` → announces a message of 1 TB: the core refuses the stream.
// - `$/stderr` → writes its params' `text` on stderr.
//
// Its arguments: `--crash-at-start` ends at once (code 2), after a line on stderr; `--count <file>` appends a line to
// the file at each start (the restarts are counted).

import { appendFileSync } from 'node:fs';
import { spawn } from 'node:child_process';

const args = process.argv.slice(2);
const countAt = args.indexOf('--count');
if (countAt >= 0) appendFileSync(args[countAt + 1], `${process.pid}\n`);
process.stderr.write(`fake-lsp started (pid ${process.pid}) in ${process.cwd()}\n`);
if (args.includes('--crash-at-start')) {
  process.stderr.write('fake-lsp: crashing at start\n');
  process.exit(2);
}

function send(msg) {
  const body = Buffer.from(JSON.stringify(msg), 'utf8');
  process.stdout.write(`Content-Length: ${body.length}\r\n\r\n`);
  process.stdout.write(body);
}

function handle(msg) {
  const { id, method, params } = msg;
  switch (method) {
    case 'initialize':
      return send({ jsonrpc: '2.0', id, result: { capabilities: { hoverProvider: true }, serverInfo: { name: 'fake-lsp' } } });
    case 'shutdown':
      return send({ jsonrpc: '2.0', id, result: null });
    case 'exit':
      return process.exit(0);
    case '$/echo':
      return send({ jsonrpc: '2.0', id, result: params });
    case '$/crash':
      return process.exit(3);
    case '$/child': {
      const child = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 600000)'], { stdio: 'ignore' });
      return send({ jsonrpc: '2.0', id, result: { pid: child.pid } });
    }
    case '$/huge':
      return process.stdout.write('Content-Length: 1000000000000\r\n\r\n{');
    case '$/stderr':
      process.stderr.write(`${params.text}\n`);
      return send({ jsonrpc: '2.0', id, result: null });
    default:
      if (id !== undefined) send({ jsonrpc: '2.0', id, error: { code: -32601, message: `unknown method ${method}` } });
  }
}

let buf = Buffer.alloc(0);
process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  for (;;) {
    const end = buf.indexOf('\r\n\r\n');
    if (end < 0) return;
    const header = buf.subarray(0, end).toString('ascii');
    const m = /content-length:\s*(\d+)/i.exec(header);
    const len = m ? Number(m[1]) : 0;
    if (buf.length < end + 4 + len) return;
    const body = buf.subarray(end + 4, end + 4 + len).toString('utf8');
    buf = buf.subarray(end + 4 + len);
    handle(JSON.parse(body));
  }
});
process.stdin.on('end', () => process.exit(0));

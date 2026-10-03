#!/usr/bin/env node
// Test double of the GitHub CLI: `gh pr create … --body-file -` prints the address of PR #12.
// Each call appends {argv, cwd, body} to <tmp>/fake-gh-<cwd with non-alphanumerics replaced by _>.jsonl.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const argv = process.argv.slice(2);
let body = '';
process.stdin.setEncoding('utf8');
process.stdin.on('data', (d) => (body += d));
process.stdin.on('end', () => {
  const log = path.join(os.tmpdir(), `fake-gh-${process.cwd().replace(/[^a-zA-Z0-9]/g, '_')}.jsonl`);
  fs.appendFileSync(log, JSON.stringify({ argv, cwd: process.cwd(), body }) + '\n');
  if (argv[0] === 'pr' && argv[1] === 'create') {
    process.stdout.write('https://github.com/acme/demo/pull/12\n');
  } else {
    process.stderr.write(`unsupported: ${argv.join(' ')}\n`);
    process.exit(1);
  }
});

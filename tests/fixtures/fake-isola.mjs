#!/usr/bin/env node
// Test double of the isola CLI, run in a worktree. `up` marks it up (`.isola-up`), `ls --json` then
// lists its service `web` on the worktree's branch (`.isola-branch`, else git's), on the proxy at
// http://<branch>.demo.localhost:3000 and on its own at http://127.0.0.1:<.isola-port, else 9>,
// `down` and `destroy` mark it down. With `.isola-fail` in the folder, every call fails.
// Each call appends its arguments to <tmp>/fake-isola-<cwd with non-alphanumerics replaced by _>.log.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const argv = process.argv.slice(2);
const cwd = process.cwd();
const log = path.join(os.tmpdir(), `fake-isola-${cwd.replace(/[^a-zA-Z0-9]/g, '_')}.log`);
fs.appendFileSync(log, argv.join(' ') + '\n');

const here = (name) => path.join(cwd, name);
const read = (name) => (fs.existsSync(here(name)) ? fs.readFileSync(here(name), 'utf8').trim() : '');

if (fs.existsSync(here('.isola-fail'))) {
  process.stderr.write('Error: the fake isola fails\n');
  process.exit(1);
}

switch (argv[0]) {
  case 'up':
    fs.writeFileSync(here('.isola-up'), '');
    process.stdout.write('✓ 1 service started\n');
    break;
  case 'ls': {
    if (!fs.existsSync(here('.isola-up'))) {
      process.stdout.write('[]\n');
      break;
    }
    const branch = read('.isola-branch') || execFileSync('git', ['rev-parse', '--abbrev-ref', 'HEAD'], { encoding: 'utf8' }).trim();
    const port = Number(read('.isola-port') || 9);
    const url = `http://${branch.replace(/[^a-zA-Z0-9]+/g, '-')}.demo.localhost:3000`;
    const web = { worktree: branch, service: 'web', port, status: 'running', pid: 1, url, direct_url: `http://127.0.0.1:${port}` };
    process.stdout.write(JSON.stringify([web]) + '\n');
    break;
  }
  case 'down':
  case 'destroy':
    fs.rmSync(here('.isola-up'), { force: true });
    process.stdout.write(`✓ ${argv[0]}\n`);
    break;
  default:
    process.stderr.write(`unsupported: ${argv.join(' ')}\n`);
    process.exit(1);
}

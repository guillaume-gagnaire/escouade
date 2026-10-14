#!/usr/bin/env node
// Test double of the GitHub CLI: `gh pr create … --body-file -` prints the address of PR #12,
// `gh auth token` a token.
// Each call appends {argv, cwd, body} to <tmp>/fake-gh-<cwd with non-alphanumerics replaced by _>.jsonl.
// `gh pr list --head <branch> … --json …` and `gh pr view <url> --json …` answer from what a test wrote
// in <the same name>.pulls.json: an array of pull requests as `gh` gives them, each with its `headRefName`
// (and `was`, the address it had before its repository was renamed: asked there, it is found too).
// Without that file, a branch has no pull request and none is found at an address. With
// {"forbidChecks": true, "pulls": […]} instead of the array, asking for statusCheckRollup is refused, as
// GitHub refuses it to a token that may not read the checks.
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
  } else if (argv[0] === 'auth' && argv[1] === 'token') {
    process.stdout.write('gho_fake\n');
  } else if (argv[0] === 'pr' && (argv[1] === 'list' || argv[1] === 'view')) {
    let github = [];
    try {
      github = JSON.parse(fs.readFileSync(log.replace(/\.jsonl$/, '.pulls.json'), 'utf8'));
    } catch {
      // No file: GitHub has none.
    }
    const pulls = Array.isArray(github) ? github : github.pulls;
    if (github.forbidChecks && argv[argv.indexOf('--json') + 1].includes('statusCheckRollup')) {
      process.stderr.write('GraphQL: Resource not accessible by personal access token (repository.pullRequest.statusCheckRollup)\n');
      process.exit(1);
    }
    // Only what was asked for, as `gh` does.
    const fields = argv[argv.indexOf('--json') + 1].split(',');
    const asked = (p) => Object.fromEntries(fields.filter((f) => f in p).map((f) => [f, p[f]]));
    if (argv[1] === 'list') {
      const head = argv[argv.indexOf('--head') + 1];
      process.stdout.write(JSON.stringify(pulls.filter((p) => p.headRefName === head).map(asked)) + '\n');
    } else {
      const pr = pulls.find((p) => p.url === argv[2] || p.was === argv[2]);
      if (!pr) {
        process.stderr.write(`no pull requests found for ${argv[2]}\n`);
        process.exit(1);
      }
      process.stdout.write(JSON.stringify(asked(pr)) + '\n');
    }
  } else {
    process.stderr.write(`unsupported: ${argv.join(' ')}\n`);
    process.exit(1);
  }
});

// @vitest-environment node
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

// ESC-483: the workflows that only make sense in guillaume-gagnaire/escouade (they need its secrets, its
// npm trust, its main, or cost minutes nobody else should pay) check the repository on every job. A fork or a
// copy does not run them. ci.yml stays open, and macos-paste.yml only runs by hand. CONTRIBUTING.md says it.
const DIR = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', '.github', 'workflows');
const ORIGIN = "github.repository == 'guillaume-gagnaire/escouade'";

// The workflows whose every job is bounded, and the one that bounds only its nightly run.
const BOUNDED = ['release.yml', 'revocations.yml', 'sdk-publish.yml', 'discord.yml'];
const SCHEDULE_ONLY = ['advisories.yml'];

// The jobs of a workflow and their `if:` line, read as text (the repository has no YAML parser at its root).
function jobs(file: string): Map<string, string | undefined> {
  const lines = fs.readFileSync(path.join(DIR, file), 'utf8').split(/\r?\n/);
  const start = lines.findIndex((l) => l === 'jobs:');
  expect(start, `${file} has a jobs: section`).toBeGreaterThan(-1);
  const found = new Map<string, string | undefined>();
  let current: string | undefined;
  for (const line of lines.slice(start + 1)) {
    const job = /^ {2}([A-Za-z0-9_-]+):\s*$/.exec(line);
    if (job) {
      current = job[1];
      found.set(current, undefined);
      continue;
    }
    const cond = /^ {4}if:\s*(.+)$/.exec(line);
    if (cond && current) found.set(current, cond[1]);
  }
  return found;
}

describe('workflows bounded to the origin repository', () => {
  for (const file of BOUNDED) {
    it(`${file}: every job checks the repository`, () => {
      const all = jobs(file);
      expect(all.size).toBeGreaterThan(0);
      for (const [name, cond] of all) {
        expect(cond, `${file}: the job ${name} lost its \`if: ${ORIGIN}\``).toBeDefined();
        expect(cond, `${file}: the job ${name}`).toContain(ORIGIN);
      }
    });
  }

  for (const file of SCHEDULE_ONLY) {
    it(`${file}: the nightly run is only in the origin repository, the push and the pull requests stay open`, () => {
      const all = jobs(file);
      expect(all.size).toBeGreaterThan(0);
      for (const [name, cond] of all) {
        expect(cond, `${file}: the job ${name} lost its condition on the schedule`).toBeDefined();
        expect(cond, `${file}: the job ${name}`).toContain("github.event_name != 'schedule'");
        expect(cond, `${file}: the job ${name}`).toContain(ORIGIN);
      }
    });
  }

  it('ci.yml stays open to the forks and to the pull requests', () => {
    const text = fs.readFileSync(path.join(DIR, 'ci.yml'), 'utf8');
    expect(text).not.toContain('github.repository ==');
    expect(text).toMatch(/^ {2}pull_request:/m);
  });

  it('every schedule is in a workflow that is bounded', () => {
    for (const file of fs.readdirSync(DIR).filter((f) => f.endsWith('.yml'))) {
      const text = fs.readFileSync(path.join(DIR, file), 'utf8');
      if (/^ {2}schedule:/m.test(text)) {
        expect([...BOUNDED, ...SCHEDULE_ONLY], `${file} has a schedule`).toContain(file);
      }
    }
  });
});

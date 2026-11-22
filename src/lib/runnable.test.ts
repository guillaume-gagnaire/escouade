import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { opensOutside, RUNNABLE } from './runnable';

describe('runnable', () => {
  it('is the list of the core, no more, no less', () => {
    const rust = readFileSync(resolve(process.cwd(), 'src-tauri/src/sentfiles.rs'), 'utf8');
    const body = rust.slice(rust.indexOf('const RUNNABLE'), rust.indexOf('];', rust.indexOf('const RUNNABLE')));
    const core = [...body.matchAll(/\("([a-z0-9-]+)", Why::/g)].map((m) => m[1]);
    expect(core.length).toBeGreaterThan(80);
    expect([...RUNNABLE].sort()).toEqual([...core].sort());
  });

  it('refuses what runs, installs, mounts or follows, whatever its case, its final dots and its stream', () => {
    for (const name of [
      'a.msc',
      'A.TERMINAL',
      'a.appinstaller.',
      'a.iso ',
      'a.vhdx. .',
      'C:\\w\\a.library-ms',
      'a.msc::$DATA',
      'n.txt:a.lnk',
      'a.html',
    ]) {
      expect(opensOutside(name), name).toBe(true);
    }
    for (const name of ['a.pdf', 'a.docx', 'a.exe.pdf', 'msc', 'a.', '/w.app/notes.md']) {
      expect(opensOutside(name), name).toBe(false);
    }
  });
});

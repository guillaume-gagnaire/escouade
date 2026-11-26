// @vitest-environment node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { imageTags } from './image-tags.mjs';

// ESC-586: the tags the Release's job « server-image » puts on each server's image. `latest` is a stable version's
// alone, so a compose that pulls `latest` never gets a beta.
const SCRIPT = path.join(path.dirname(fileURLToPath(import.meta.url)), 'image-tags.mjs');
const IMAGE = 'ghcr.io/guillaume-gagnaire/escouade/escouade-team-server';

describe('the tags of a server’s image', () => {
  it('a stable version: its number, its minor one (the composes pull it, SR1-12) and latest', () => {
    expect(imageTags(IMAGE, 'v2.3.0')).toEqual([`${IMAGE}:2.3.0`, `${IMAGE}:2.3`, `${IMAGE}:latest`]);
    expect(imageTags(IMAGE, '2.2.5')).toEqual([`${IMAGE}:2.2.5`, `${IMAGE}:2.2`, `${IMAGE}:latest`]);
  });

  it('a beta: its number and beta, never latest', () => {
    expect(imageTags(IMAGE, 'v2.3.0-beta.1')).toEqual([`${IMAGE}:2.3.0-beta.1`, `${IMAGE}:beta`]);
    expect(imageTags(IMAGE, 'v2.3.0-beta.12')).toEqual([`${IMAGE}:2.3.0-beta.12`, `${IMAGE}:beta`]);
    // Any pre-release, not only a beta.
    expect(imageTags(IMAGE, 'v3.0.0-rc.1')).not.toContain(`${IMAGE}:latest`);
    expect(imageTags(IMAGE, 'v2.3.0-beta.1')).not.toContain(`${IMAGE}:2.3`);
  });

  it('refuses what is not a version’s tag, or not an image', () => {
    for (const tag of ['latest', 'v2.3', 'main', '', 'v2.3.0 ', 'v2.3.0-'])
      expect(() => imageTags(IMAGE, tag), tag).toThrow(/not a version/);
    expect(() => imageTags('', 'v2.3.0')).toThrow(/image/);
    expect(() => imageTags(`${IMAGE}:`, 'v2.3.0')).toThrow(/image/);
  });

  it('the script prints one tag a line, as the Release reads them, and fails without a version', () => {
    const ok = spawnSync(process.execPath, [SCRIPT, IMAGE, 'v2.3.0-beta.1'], { encoding: 'utf8' });
    expect(ok.status).toBe(0);
    expect(ok.stdout).toBe(`${IMAGE}:2.3.0-beta.1\n${IMAGE}:beta\n`);
    const bad = spawnSync(process.execPath, [SCRIPT, IMAGE, 'nightly'], { encoding: 'utf8' });
    expect(bad.status).toBe(1);
    expect(bad.stdout).toBe('');
  });
});

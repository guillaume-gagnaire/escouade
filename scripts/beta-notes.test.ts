// @vitest-environment node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { betaNotice, isBeta, stableBefore } from './beta-notes.mjs';

// ESC-586: what a beta says before its notes, on the Release and on Discord.
const SCRIPT = path.join(path.dirname(fileURLToPath(import.meta.url)), 'beta-notes.mjs');

describe('the notice of a beta', () => {
  it('tells a beta from a stable version', () => {
    expect(isBeta('v2.3.0-beta.1')).toBe(true);
    expect(isBeta('2.3.0-rc.2')).toBe(true);
    expect(isBeta('v2.3.0')).toBe(false);
    expect(isBeta('nightly')).toBe(false);
  });

  it('goes back to the stable line before it', () => {
    expect(stableBefore('v2.3.0-beta.1')).toBe('2.2');
    expect(stableBefore('v2.3.4-beta.1')).toBe('2.3');
    expect(stableBefore('v3.0.0-beta.1')).toBeNull();
  });

  it('says what a beta is, who gets it and how to go back, in French and in English', () => {
    const fr = betaNotice('v2.3.0-beta.1');
    expect(fr).toContain("**Version bêta.** Escouade 2.3.0-beta.1 est une version d'essai, pas la version stable");
    expect(fr).toContain('**Revenir à la 2.2**');
    expect(fr).toContain('réinstalle la dernière 2.2');
    expect(fr).toContain('Tes projets, tes tickets et tes réglages restent');
    expect(fr.split('\n').every((l) => l.startsWith('>'))).toBe(true);
    const en = betaNotice('v2.3.0-beta.1', 'en');
    expect(en).toContain('**Beta version.** Escouade 2.3.0-beta.1 is a trial version, not the stable one');
    expect(en).toContain('**Going back to 2.2**');
    expect(betaNotice('v3.0.0-beta.1')).toContain('**Revenir à la version stable**');
  });

  it('says nothing for a stable version', () => {
    expect(betaNotice('v2.3.0')).toBe('');
    const r = spawnSync(process.execPath, [SCRIPT, 'v2.3.0', '--lang', 'en'], { encoding: 'utf8' });
    expect(r.status).toBe(0);
    expect(r.stdout).toBe('');
  });

  it('the script prints it as the Release reads it, and refuses what is not a version', () => {
    const r = spawnSync(process.execPath, [SCRIPT, 'v2.3.0-beta.1', '--lang', 'en'], { encoding: 'utf8' });
    expect(r.status).toBe(0);
    expect(r.stdout).toBe(betaNotice('v2.3.0-beta.1', 'en') + '\n');
    expect(spawnSync(process.execPath, [SCRIPT, 'main'], { encoding: 'utf8' }).status).toBe(1);
  });
});

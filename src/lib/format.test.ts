import { describe, expect, it } from 'vitest';
import {
  basename,
  dirname,
  fAgo,
  fBytes,
  fCountdown,
  fDur,
  fTok,
  fUsd,
  fWhen,
  isAbsPath,
  joinPath,
  plural,
  relPath,
  tildify,
} from './format';

describe('fTok', () => {
  it.each([
    [0, '0'],
    [999, '999'],
    [1000, '1,0 k'],
    [182400, '182,4 k'],
    [1_000_000, '1,00 M'],
    [2_345_678, '2,35 M'],
    [3_200_000_000, '3,20 Md'],
  ])('%d → %s', (n, want) => expect(fTok(n)).toBe(want));
});

describe('fUsd', () => {
  const nbsp = (s: string) => s.replace(/\s/g, ' ');
  it('uses French decimals and a dollar suffix', () => expect(nbsp(fUsd(2.84))).toBe('2,84 $'));
  it('shows three decimals for small amounts so cents do not round to zero', () => expect(nbsp(fUsd(0.0062))).toBe('0,006 $'));
  it('keeps two decimals for zero', () => expect(nbsp(fUsd(0))).toBe('0,00 $'));
  it('groups thousands', () => expect(nbsp(fUsd(1284.6))).toBe('1 284,60 $'));
});

describe('fDur', () => {
  it.each([
    [0, '0m 00s'],
    [59_000, '0m 59s'],
    [312_000, '5m 12s'],
    [2_531_000, '42m 11s'],
    [3_920_000, '1h 05m'],
    [-5, '0m 00s'],
  ])('%d ms → %s', (ms, want) => expect(fDur(ms)).toBe(want));
});

describe('fCountdown', () => {
  const now = 1_000_000_000_000;
  it('formats hours and minutes', () => expect(fCountdown(now + (1 * 3600 + 48 * 60) * 1000, now)).toBe('1h48'));
  it('switches to days beyond 24h', () => expect(fCountdown(now + (2 * 86400 + 5 * 3600) * 1000, now)).toBe('2j 5h'));
  it('never goes negative', () => expect(fCountdown(now - 10_000, now)).toBe('0h00'));
  it('shows a dash when unknown', () => expect(fCountdown(null, now)).toBe('—'));
});

describe('paths', () => {
  it('relPath strips the base case-insensitively and normalizes slashes', () => {
    expect(relPath('C:\\Code\\App', 'c:\\code\\app\\src\\main.ts')).toBe('src/main.ts');
  });
  it('joinPath puts a path of a folder under it, with the folder’s separator', () => {
    expect(joinPath('C:\\code\\app', 'src/main.ts')).toBe('C:\\code\\app\\src\\main.ts');
    // Git spells a Windows folder with slashes: the path is given as Windows writes it.
    expect(joinPath('C:/code/app/', 'src/main.ts')).toBe('C:\\code\\app\\src\\main.ts');
    expect(joinPath('\\\\server\\share', 'a.ts')).toBe('\\\\server\\share\\a.ts');
    expect(joinPath('/home/me/app', 'a.ts')).toBe('/home/me/app/a.ts');
  });
  it('relPath leaves unrelated paths untouched', () => {
    expect(relPath('C:\\code\\app', 'C:\\code\\application\\x.ts')).toBe('C:/code/application/x.ts');
  });
  it('basename and dirname handle both separators', () => {
    expect(basename('src\\middleware\\auth.ts')).toBe('auth.ts');
    expect(dirname('src/middleware/auth.ts')).toBe('src/middleware');
    expect(dirname('README.md')).toBe('.');
  });
  it('isAbsPath tells drive, rooted and UNC paths from relative ones', () => {
    for (const p of ['C:\\code\\x.ts', 'c:/code/x.ts', '/tmp/x', '\\\\server\\share\\x']) expect(isAbsPath(p)).toBe(true);
    for (const p of ['src/x.ts', 'x.ts', '../x.ts', 'C:x.ts']) expect(isAbsPath(p)).toBe(false);
  });
  it('tildify shortens the home folder only', () => {
    expect(tildify('C:\\Users\\guill\\dev\\app')).toBe('~/dev/app');
    expect(tildify('D:\\work\\app')).toBe('D:\\work\\app');
  });
});

describe('plural', () => {
  it('agrees with the count', () => {
    expect(plural(1, 'fichier', 'fichiers')).toBe('1 fichier');
    expect(plural(3, 'fichier', 'fichiers')).toBe('3 fichiers');
  });
});

describe('fAgo', () => {
  const now = Date.UTC(2026, 8, 28, 12, 0, 0);
  const ago = (s: number) => fAgo(now / 1000 - s, now);

  it('says how long ago a commit was made', () => {
    expect(ago(20)).toBe('à l’instant');
    expect(ago(5 * 60)).toBe('il y a 5 min');
    expect(ago(3 * 3600)).toBe('il y a 3 h');
    expect(ago(30 * 3600)).toBe('hier');
    expect(ago(4 * 86400)).toBe('il y a 4 j');
  });

  it('gives the date beyond a month', () => {
    expect(ago(40 * 86400)).toBe(new Date(now - 40 * 86400e3).toLocaleDateString('fr-FR'));
  });
});

describe('fBytes', () => {
  const MB = 1024 * 1024;
  it('says megabytes, then gigabytes with a decimal', () => {
    expect(fBytes(0)).toBe('0 Mo');
    expect(fBytes(312.4 * MB)).toBe('312 Mo');
    expect(fBytes(1023 * MB)).toBe('1023 Mo');
    expect(fBytes(1024 * MB)).toBe('1 Go');
    expect(fBytes(1.26 * 1024 * MB)).toBe('1,3 Go');
  });
});

describe('fWhen', () => {
  it('gives the time alone today, and the day before it otherwise', () => {
    const now = new Date(2026, 8, 30, 12, 0).getTime();
    expect(fWhen(new Date(2026, 8, 30, 15, 0).getTime(), now)).toBe('à 15:00');
    expect(fWhen(new Date(2026, 9, 2, 9, 30).getTime(), now)).toBe('le vendredi 2 octobre à 09:30');
  });
});

import { beforeEach, describe, expect, it } from 'vitest';
import {
  basename,
  dirname,
  fAgo,
  fBytes,
  fCountdown,
  fDate,
  fDateTime,
  fDayMonth,
  fDur,
  fInt,
  fList,
  fNum,
  fPct,
  fSince,
  fTime,
  fTok,
  fUsd,
  fWhen,
  isAbsPath,
  joinPath,
  plural,
  relPath,
  tildify,
} from './format';
import { setLang } from './i18n';

/** Any space as a plain one: `Intl` puts non-breaking ones between digits, and before « AM » in some versions. */
const spaces = (s: string) => s.replace(/\s/g, ' ');

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
  // In a narrow card, « $ » never goes to a line of its own.
  it('keeps the dollar sign on the line of its amount', () => expect(fUsd(2.84)).toBe('2,84\u00a0$'));
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
  it('tildify shortens a home folder of macOS on macOS', () => {
    expect(tildify('/Users/ada/.escouade/claude/pro', true)).toBe('~/.escouade/claude/pro');
    expect(tildify('/Users/ada', true)).toBe('~');
    expect(tildify('/Volumes/work/app', true)).toBe('/Volumes/work/app');
    expect(tildify('C:\\Users\\ada\\app', true)).toBe('C:\\Users\\ada\\app');
    // A folder of the drive's root on Windows.
    expect(tildify('/Users/ada/app', false)).toBe('/Users/ada/app');
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

describe('fSince', () => {
  const now = Date.UTC(2026, 9, 10, 12, 0, 0);
  const since = (s: number) => fSince(now - s * 1000, now);

  it('says for how long, to follow « depuis »', () => {
    expect(since(20)).toBe('< 1 min');
    expect(since(3 * 60 + 40)).toBe('3 min');
    expect(since(2 * 3600 + 59 * 60)).toBe('2 h');
    expect(since(3 * 86400)).toBe('3 j');
    // A clock set back is no negative time.
    expect(since(-30)).toBe('< 1 min');
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

describe('fPct, fInt, fNum, fDate, fTime, fDateTime', () => {
  const day = new Date(2026, 9, 2, 9, 30).getTime();
  it('write a percentage, an integer, a date and a time the French way', () => {
    expect(fPct(41.6)).toBe('42 %');
    expect(spaces(fInt(1_234_567.4))).toBe('1 234 567');
    expect(fDate(day)).toBe('2 octobre 2026');
    expect(fTime(day)).toBe('09:30');
    expect(fDateTime(day)).toBe('2 octobre 2026 à 09:30');
  });

  it('write a day and a month in figures, the day first in French, the month first in English', () => {
    const day = new Date(2026, 9, 2, 9, 30).getTime();
    expect(fDayMonth(day)).toBe('02/10');
    expect(fDayMonth(new Date(2026, 9, 10, 18, 0).getTime())).toBe('10/10');
    setLang('en');
    expect(fDayMonth(day)).toBe('10/02');
  });

  it('write a number with as many decimals as asked, at most', () => {
    expect(fNum(1.26, 1)).toBe('1,3');
    expect(fNum(1.8, 1)).toBe('1,8');
    expect(fNum(2, 1)).toBe('2');
    expect(spaces(fNum(1234.5))).toBe('1 235');
  });
});

describe('fBytes with decimals', () => {
  const MB = 1024 * 1024;
  it('gives the megabytes to a decimal when asked, as a file size', () => {
    expect(fBytes(1.26 * MB, 1)).toBe('1,3 Mo');
    expect(fBytes(1023 * MB, 1)).toBe('1023 Mo');
    expect(fBytes(1.26 * 1024 * MB, 1)).toBe('1,3 Go');
  });
});

describe('fSince over days', () => {
  const now = Date.UTC(2026, 9, 10, 12, 0, 0);
  it('says « j » for one day as for several', () => {
    expect(fSince(now - 86400e3, now)).toBe('1 j');
    expect(fSince(now - 3 * 86400e3, now)).toBe('3 j');
  });
});

describe('in English', () => {
  beforeEach(() => setLang('en'));
  const now = Date.UTC(2026, 8, 28, 12, 0, 0);

  it('writes tokens with a dot and a unit stuck to the number', () => {
    expect([999, 1000, 182400, 2_345_678, 3_200_000_000].map(fTok)).toEqual(['999', '1.0k', '182.4k', '2.35M', '3.20B']);
  });

  it('puts the dollar sign before the amount', () => {
    expect(fUsd(0.1)).toBe('$0.10');
    expect(fUsd(0.0062)).toBe('$0.006');
    expect(fUsd(1284.6)).toBe('$1,284.60');
  });

  it('writes a percentage without a space, and an integer with commas', () => {
    expect(fPct(41.6)).toBe('42%');
    expect(fInt(1_234_567.4)).toBe('1,234,567');
  });

  it('writes durations and countdowns with d for days', () => {
    expect(fDur(312_000)).toBe('5m 12s');
    expect(fDur(3_920_000)).toBe('1h 05m');
    expect(fCountdown(now + (1 * 3600 + 48 * 60) * 1000, now)).toBe('1h48');
    expect(fCountdown(now + (4 * 86400 + 12 * 3600) * 1000, now)).toBe('4d 12h');
    expect(fCountdown(null, now)).toBe('—');
  });

  it('says how long ago, its units spaced from the number as « 5 min ago »', () => {
    const ago = (s: number) => fAgo(now / 1000 - s, now);
    expect([20, 5 * 60, 3 * 3600, 30 * 3600, 4 * 86400].map(ago)).toEqual(['just now', '5 min ago', '3 h ago', 'yesterday', '4 days ago']);
    expect(ago(40 * 86400)).toBe(new Date(now - 40 * 86400e3).toLocaleDateString('en-US'));
  });

  it('says for how long, days written out with their plural', () => {
    const since = (s: number) => fSince(now - s * 1000, now);
    expect([20, 3 * 60 + 40, 2 * 3600 + 59 * 60, 86400, 3 * 86400].map(since)).toEqual(['< 1 min', '3 min', '2 h', '1 day', '3 days']);
  });

  it('writes sizes in MB and GB', () => {
    const MB = 1024 * 1024;
    expect([0, 312.4 * MB, 1024 * MB, 1.26 * 1024 * MB].map((n) => fBytes(n))).toEqual(['0 MB', '312 MB', '1 GB', '1.3 GB']);
    expect(fBytes(1.26 * MB, 1)).toBe('1.3 MB');
  });

  it('writes numbers with a dot and commas', () => {
    expect(fNum(1.26, 1)).toBe('1.3');
    expect(fNum(1234.5)).toBe('1,235');
  });

  it('writes dates and times the American way', () => {
    const day = new Date(2026, 9, 2, 9, 30).getTime();
    expect(fDate(day)).toBe('October 2, 2026');
    expect(spaces(fTime(day))).toBe('9:30 AM');
    expect(spaces(fDateTime(day))).toBe('October 2, 2026 at 9:30 AM');
  });

  it('joins a list with « and » and the serial comma', () => {
    expect(fList(['a', 'b', 'c'])).toBe('a, b, and c');
    setLang('fr');
    // Switched back: the list follows at once.
    expect(fList(['a', 'b', 'c'])).toBe('a, b et c');
  });

  it('says when something happens', () => {
    const today = new Date(2026, 8, 30, 12, 0).getTime();
    expect(spaces(fWhen(new Date(2026, 8, 30, 15, 0).getTime(), today))).toBe('at 3:00 PM');
    expect(spaces(fWhen(new Date(2026, 9, 2, 9, 30).getTime(), today))).toBe('on Friday, October 2 at 9:30 AM');
  });
});

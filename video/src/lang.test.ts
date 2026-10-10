import { describe, expect, it } from 'vitest';
import { fmtOf, trOf } from './lang';

describe('tr', () => {
  it('picks the text of the language', () => {
    expect(trOf('fr')('Nouvel agent', 'New agent')).toBe('Nouvel agent');
    expect(trOf('en')('Nouvel agent', 'New agent')).toBe('New agent');
  });
});

describe('numbers and amounts', () => {
  const fr = fmtOf('fr');
  const en = fmtOf('en');

  it('pick a value, or some markup, by language: the French keeps its own text nodes', () => {
    expect(fr.pick(304, 256)).toBe(304);
    expect(en.pick(304, 256)).toBe(256);
    expect(fr.trx('un', 'one')).toBe('un');
    expect(en.trx('un', 'one')).toBe('one');
  });

  it('write tokens as the app does in each language', () => {
    expect(fr.tok(184, 0)).toBe('184 k');
    expect(fr.tok(24.8, 1)).toBe('24,8 k');
    expect(en.tok(184, 0)).toBe('184k');
    expect(en.tok(24.8, 1)).toBe('24.8k');
  });

  it('write amounts: « 2,41 $ » in French, « $2.41 » in English', () => {
    expect(fr.usd(2.41)).toBe('2,41 $');
    expect(en.usd(2.41)).toBe('$2.41');
    expect(fr.usd(0.04)).toBe('0,04 $');
    expect(en.usd(0)).toBe('$0.00');
  });

  it('write percentages, millions of tokens and counts', () => {
    expect(fr.pct(22)).toBe('22 %');
    expect(en.pct(22)).toBe('22%');
    expect(fr.million(28.1)).toBe('28,10 M');
    expect(en.million(28.1)).toBe('28.10M');
    expect(fr.int(1234)).toBe('1 234');
    expect(en.int(1234)).toBe('1,234');
  });

  it('agree a noun with its count: zero is singular in French, plural in English', () => {
    expect(fr.plural(0, 'fichier', 'fichiers', 'file', 'files')).toBe('fichier');
    expect(fr.plural(1, 'fichier', 'fichiers', 'file', 'files')).toBe('fichier');
    expect(fr.plural(2, 'fichier', 'fichiers', 'file', 'files')).toBe('fichiers');
    expect(en.plural(0, 'fichier', 'fichiers', 'file', 'files')).toBe('files');
    expect(en.plural(1, 'fichier', 'fichiers', 'file', 'files')).toBe('file');
    expect(en.plural(2, 'fichier', 'fichiers', 'file', 'files')).toBe('files');
  });
});

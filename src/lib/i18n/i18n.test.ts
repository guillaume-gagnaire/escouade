import { render, screen } from '@testing-library/svelte';
import { createRawSnippet, flushSync } from 'svelte';
import { describe, expect, it } from 'vitest';
import { fList } from '../format';
import { locale, setLang, t, tIn, tRich } from '.';
import LangProbe from './LangProbe.test.svelte';
import Rich from './Rich.svelte';

describe('t', () => {
  it('gives the text of a key in the language of the interface', () => {
    expect(t('common.cancel')).toBe('Annuler');
    setLang('en');
    expect(t('common.cancel')).toBe('Cancel');
  });

  it('starts each test in French', () => {
    // The test above switched to English: the setup puts French back before each test.
    expect(locale.ui).toBe('fr');
    expect(t('common.close')).toBe('Fermer');
  });

  it('fills in its parameters', () => {
    expect(t('format.ago.minutes', { n: 5 })).toBe('il y a 5 min');
    expect(t('format.when.day', { date: 'vendredi 2 octobre', time: '09:30' })).toBe('le vendredi 2 octobre à 09:30');
  });

  it('gives a text in a language of its own with tIn, whatever the interface’s', () => {
    expect(tIn('en', 'format.ago.minutes', { n: 5 })).toBe('5 min ago');
    expect(tIn('fr', 'common.cancel')).toBe('Annuler');
    expect(locale.ui).toBe('fr');
  });
});

describe('plurals', () => {
  const files = (lang: 'fr' | 'en') => [0, 1, 2].map((count) => tIn(lang, 'common.count.files', { count }));

  it('puts 0 and 1 in the singular in French', () => expect(files('fr')).toEqual(['0 fichier', '1 fichier', '2 fichiers']));

  it('puts only 1 in the singular in English', () => expect(files('en')).toEqual(['0 files', '1 file', '2 files']));

  it('uses « other » for a category the leaf does not write', () => {
    // French has a « many » form for a million: the leaf has only « one » and « other ».
    expect(tIn('fr', 'common.count.files', { count: 1_000_000 })).toBe('1000000 fichiers');
  });

  it('follows the language of the interface', () => {
    setLang('en');
    expect(t('common.count.files', { count: 1 })).toBe('1 file');
  });
});

describe('setLang', () => {
  it('writes the language on the document, for the spell checker and the screen readers', () => {
    setLang('en');
    expect(document.documentElement.lang).toBe('en');
    setLang('fr');
    expect(document.documentElement.lang).toBe('fr');
  });

  it('switches a rendered component to the new language at once, in its markup and in its $derived', () => {
    render(LangProbe, { props: { count: 2 } });
    expect(screen.getByRole('button')).toHaveTextContent('Annuler');
    expect(screen.getByTestId('derived')).toHaveTextContent('2 fichiers');

    setLang('en');
    flushSync();
    expect(screen.getByRole('button')).toHaveTextContent('Cancel');
    expect(screen.getByTestId('derived')).toHaveTextContent('2 files');
  });
});

describe('Rich', () => {
  const kbd = createRawSnippet(() => ({ render: () => '<kbd>Ctrl+Entrée</kbd>' }));

  it('renders the snippet a placeholder names, and a text parameter', () => {
    const { container } = render(Rich, { props: { text: 'Appuie sur {key} pour {action}.', key: kbd, action: 'envoyer' } });
    expect(container.querySelector('kbd')).toHaveTextContent('Ctrl+Entrée');
    // Exactly: no space added around the pieces.
    expect(container.textContent).toBe('Appuie sur Ctrl+Entrée pour envoyer.');
  });

  it('writes a text parameter as text, never as markup', () => {
    const { container } = render(Rich, { props: { text: 'Fichier {name}', name: '<b>x</b>' } });
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toBe('Fichier <b>x</b>');
  });

  it('leaves a placeholder it is given nothing for as written', () => {
    const { container } = render(Rich, { props: { text: 'à {time}' } });
    expect(container.textContent).toBe('à {time}');
  });

  it('takes its text from tRich, which keeps the placeholders left to the snippets', () => {
    expect(tRich('format.when.today')).toBe('à {time}');
    expect(tRich('format.when.day', { date: 'demain' })).toBe('le demain à {time}');
    expect(tRich('common.count.files', { count: 2 })).toBe('2 fichiers');
    setLang('en');
    const { container } = render(Rich, { props: { text: tRich('format.when.today'), time: kbd } });
    expect(container).toHaveTextContent('at Ctrl+Entrée');
  });
});

describe('fList', () => {
  it('joins with a conjunction, as each language does', () => {
    expect(fList(['a', 'b', 'c'])).toBe('a, b et c');
    expect(fList(['a', 'b'])).toBe('a et b');
    expect(fList(['a'])).toBe('a');
    expect(fList([])).toBe('');
    setLang('en');
    expect(fList(['a', 'b', 'c'])).toBe('a, b, and c');
    expect(fList(['a', 'b'])).toBe('a and b');
  });
});

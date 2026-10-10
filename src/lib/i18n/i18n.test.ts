import { render, screen } from '@testing-library/svelte';
import { createRawSnippet, flushSync, type Component } from 'svelte';
import { describe, expect, it } from 'vitest';
import { fList } from '../format';
import { locale, setLang, t, tIn, tRaw, type Key, type RichProps } from '.';
import { langOfTag } from './locale.svelte';
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
    expect(screen.getByTestId('rich').textContent).toBe('le vendredi à 09:30');

    setLang('en');
    flushSync();
    expect(screen.getByRole('button')).toHaveTextContent('Cancel');
    expect(screen.getByTestId('derived')).toHaveTextContent('2 files');
    expect(screen.getByTestId('rich').textContent).toBe('on vendredi at 09:30');
  });
});

describe('langOfTag', () => {
  // The language the window starts in, before the backend says the one of the settings: the browser's, which is the
  // system's (the default setting). The same rule as the backend's.
  it('reads French when the language subtag is, English otherwise', () => {
    expect(['fr-FR', 'fr', 'FR', 'fr-CA'].map(langOfTag)).toEqual(['fr', 'fr', 'fr', 'fr']);
    expect(['en-US', 'en', 'de-DE', 'frr', ''].map(langOfTag)).toEqual(['en', 'en', 'en', 'en', 'en']);
  });
});

describe('Rich', () => {
  const kbd = createRawSnippet(() => ({ render: () => '<kbd>09:30</kbd>' }));
  // Rendered from a script, a generic component is not inferred from its props: its key is, here.
  const rich = <K extends Key>(props: RichProps<K>) => render(Rich as unknown as Component<object>, { props });

  it('renders the snippet a placeholder names, and a text parameter', () => {
    const { container } = rich({ k: 'format.when.day', date: 'vendredi', time: kbd });
    expect(container.querySelector('kbd')).toHaveTextContent('09:30');
    // Exactly: no space added around the pieces.
    expect(container.textContent).toBe('le vendredi à 09:30');
  });

  it('writes a text parameter as text, never as markup', () => {
    const { container } = rich({ k: 'format.when.today', time: '<b>x</b>' });
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toBe('à <b>x</b>');
  });

  it('chooses the form of a plural by its count, and writes the count', () => {
    const { container } = rich({ k: 'common.count.files', count: 2 });
    expect(container.textContent).toBe('2 fichiers');
  });

  it('leaves a placeholder it is given nothing for as written (the types require them all)', () => {
    const { container } = rich({ k: 'format.when.today' } as never);
    expect(container.textContent).toBe('à {time}');
  });

  it('follows the language of the interface', () => {
    const { container } = rich({ k: 'format.when.today', time: kbd });
    expect(container.textContent).toBe('à 09:30');
    setLang('en');
    flushSync();
    expect(container.textContent).toBe('at 09:30');
  });
});

describe('tRaw', () => {
  it('gives the text of a key with its placeholders as written, its plural chosen by the count', () => {
    expect(tRaw('format.when.day')).toBe('le {date} à {time}');
    expect(tRaw('common.count.files', 1)).toBe('1 fichier');
    setLang('en');
    expect(tRaw('format.when.today')).toBe('at {time}');
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

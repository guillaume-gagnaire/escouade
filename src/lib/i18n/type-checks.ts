// Checked by `npm run check`, never run: each `@ts-expect-error` below must meet the error it announces
// (an unused one fails the check), so the types of `t` and of the English catalog keep refusing these mistakes.
import { en } from './en';
import { t, tIn, tRich } from '.';
import { catalogChecker, defineCatalog } from './types';

// --- t: the keys and their parameters --------------------------------------------------------------------------

t('common.cancel');
t('format.ago.minutes', { n: 5 });
t('common.count.files', { count: 2 });
tIn('en', 'format.when.day', { date: 'Friday, October 2', time: '9:30 AM' });

// @ts-expect-error a key the French catalog does not have
t('common.nope');
// @ts-expect-error a group is no text
t('format.ago');
// @ts-expect-error a text with a placeholder needs its parameter
t('format.ago.minutes');
// @ts-expect-error the parameters of a text are all needed
t('format.when.day', { date: 'demain' });
// @ts-expect-error a parameter the text does not have
t('format.ago.minutes', { n: 5, unit: 'min' });
// @ts-expect-error a text without placeholders takes no parameters
t('common.cancel', { n: 1 });
// @ts-expect-error a plural needs its count
t('common.count.files');
// @ts-expect-error a count is a number
t('common.count.files', { count: '2' });
// @ts-expect-error the language is French or English
tIn('de', 'common.cancel');

// tRich leaves to its snippets the placeholders it is not given, but still knows the key and its parameters.
tRich('format.when.day');
tRich('format.when.day', { date: 'demain' });
// @ts-expect-error a parameter the text does not have
tRich('format.when.day', { place: 'ici' });
// @ts-expect-error a plural still needs its count
tRich('common.count.files');

// --- the English catalog: the same keys, the same placeholders as the French one ----------------------------------

const frSample = {
  a: 'A',
  g: { hi: 'Bonjour {name}', bye: 'Au revoir' },
  files: { one: '{count} fichier dans {dir}', other: '{count} fichiers dans {dir}' },
} as const;
const defineSample = catalogChecker<typeof frSample>();
const files = { one: 'one file in {dir}', other: '{count} files in {dir}' } as const;

defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye' }, files });
// @ts-expect-error a key is missing
defineSample({ a: 'A', g: { hi: 'Hello {name}' }, files });
// @ts-expect-error a key the French catalog does not have
defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye', extra: 'More' }, files });
// @ts-expect-error another placeholder than the French one
defineSample({ a: 'A', g: { hi: 'Hello {who}', bye: 'Bye' }, files });
// @ts-expect-error a placeholder the French text does not have
defineSample({ a: 'A {x}', g: { hi: 'Hello {name}', bye: 'Bye' }, files });
// @ts-expect-error a placeholder dropped
defineSample({ a: 'A', g: { hi: 'Hello', bye: 'Bye' }, files });
// @ts-expect-error a plural with another placeholder
defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye' }, files: { one: 'one file', other: '{count} files in {d}' } });
// @ts-expect-error a plural without its « one » form
defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye' }, files: { other: '{count} files in {dir}' } });
// @ts-expect-error a plural with a form French does not write
defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye' }, files: { ...files, zero: 'no files in {dir}' } });
// @ts-expect-error a plural written as a single text
defineSample({ a: 'A', g: { hi: 'Hello {name}', bye: 'Bye' }, files: '{count} files in {dir}' });

// The real catalog, with a key less and with a placeholder renamed.
const { cancel: _cancel, ...commonWithoutCancel } = en.common;
// @ts-expect-error « common.cancel » is missing
defineCatalog({ ...en, common: commonWithoutCancel });
// @ts-expect-error « {n} » renamed
defineCatalog({ ...en, format: { ...en.format, ago: { ...en.format.ago, minutes: '{x} min ago' } } });

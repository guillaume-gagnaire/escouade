// The texts the guard of `hardcoded.ts` lets through: written the same in every language (proper nouns, symbols,
// names of keys). Each says why; the test fails on one that no longer lets anything through.
import type { Allowed } from './hardcoded';

export const ALLOWED: Allowed[] = [
  { file: '*', text: 'Escouade', why: 'le nom de l’app, le même dans toutes les langues' },
  { file: '*', text: 'Kanban', why: 'nom propre : Kanban reste Kanban (GLOSSARY.md)' },
];

// The texts the guard of `hardcoded.ts` lets through: written the same in every language (proper nouns, symbols,
// names of keys). Each says why; the test fails on one that no longer lets anything through. An exception is no way
// to skip an extraction: a text a person reads in one language is translated.
// Each task adds its own in its block below (lots run side by side: never in another's block).
import type { Allowed } from './hardcoded';

export const ALLOWED: Allowed[] = [
  // L1 — shared
  { file: '*', text: 'Escouade', why: 'le nom de l’app, le même dans toutes les langues' },
  { file: '*', text: 'Kanban', why: 'nom propre : Kanban reste Kanban (GLOSSARY.md)' },

  // L3 — editor

  // L4 — settings

  // L5 — integrations, boardSettings

  // L6 — board, stats

  // L7 — nav, git

  // L8 — conv, composer

  // L9 — runs, shell

  // L10 — errors, backend

  // K — accounts

  // M — mcp

  // G — branches
];

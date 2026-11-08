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
  { file: 'src/components/editor/EditorView.svelte', text: 'UTF-8', why: 'le nom de l’encodage, écrit pareil dans toutes les langues' },
  { file: 'src/components/editor/EditorView.svelte', text: 'CRLF', why: 'la fin de ligne Windows, écrite pareil dans toutes les langues' },
  { file: 'src/components/editor/EditorView.svelte', text: 'LF', why: 'la fin de ligne Unix, écrite pareil dans toutes les langues' },

  // L4 — settings
  {
    file: 'src/components/settings/GitPage.svelte',
    text: 'Git',
    why: 'nom propre : Git reste Git, titre du groupe des réglages git du projet',
  },
  {
    file: 'src/components/settings/GitPage.svelte',
    text: '.env*',
    why: 'un motif de fichiers donné en exemple (le champ dit « un motif par ligne »), le même dans toutes les langues',
  },
  {
    file: 'src/components/wizard/WorktreesStep.svelte',
    text: '.env*',
    why: 'le même motif d’exemple que dans Réglages › Projets, pour le même champ de l’assistant',
  },
  {
    file: 'src/components/settings/AdvancedPage.svelte',
    text: '~/.escouade',
    why: 'un chemin de dossier, pas un texte (le temps que le vrai soit lu)',
  },

  // L5 — integrations, boardSettings
  {
    file: 'src/components/settings/ServicesPage.svelte',
    text: 'atlas.atlassian.net',
    why: 'un exemple de site Jira, le même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ServicesPage.svelte',
    text: 'ada@atlas.dev',
    why: 'un exemple d’adresse, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ServicesPage.svelte',
    text: 'gh auth login',
    why: 'la commande de GitHub CLI, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ServicesPage.svelte',
    text: 'claude-ready',
    why: 'un exemple d’étiquette, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ForgesSection.svelte',
    text: 'git.example.com',
    why: 'un exemple d’instance, le même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ForgesSection.svelte',
    text: 'ada@atlas.dev',
    why: 'un exemple d’adresse, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/ForgesSection.svelte',
    text: '${…} auth login',
    why: 'la commande de la CLI de l’hébergeur (gh, glab), la même dans toutes les langues',
  },

  // L6 — board, stats

  // L7 — nav, git
  {
    file: 'src/components/TitleBar.svelte',
    text: 'Δ',
    why: 'le symbole de la différence, avant le nombre de modifications git : le même dans toutes les langues',
  },

  // L8 — conv, composer

  // L9 — runs, shell

  // L10 — errors, backend

  // K — accounts

  // M — mcp

  // G — branches

  // ESC-195 — Réglages › Projets › Kanban › Workflow
  {
    file: 'src/components/settings/WorkflowSection.svelte',
    text: 'PROMPT.md',
    why: 'le nom d’un fichier, le même dans toutes les langues',
  },
  {
    file: 'src/components/settings/WorkflowSection.svelte',
    text: '.escouade/workflows/',
    why: 'le nom d’un fichier, le même dans toutes les langues',
  },
];

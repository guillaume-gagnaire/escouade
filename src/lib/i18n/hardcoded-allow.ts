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
  { file: 'src/components/editor/EditorView.svelte', text: '.claude/worktrees/${…}', why: 'un chemin de dossier, pas un texte' },
  { file: 'src/components/editor/SourcePicker.svelte', text: '.claude/worktrees/', why: 'un chemin de dossier, pas un texte' },

  // L4 — settings

  // L5 — integrations, boardSettings
  {
    file: 'src/components/modals/ImportModal.svelte',
    text: 'Tâche',
    why: 'le type de ticket que Jira nomme ainsi : une donnée d’un service externe, comparée telle quelle, pas un texte de l’interface',
  },
  {
    file: 'src/components/settings/IntegrationsTab.svelte',
    text: 'atlas.atlassian.net',
    why: 'un exemple de site Jira, le même dans toutes les langues',
  },
  {
    file: 'src/components/settings/IntegrationsTab.svelte',
    text: 'ada@atlas.dev',
    why: 'un exemple d’adresse, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/IntegrationsTab.svelte',
    text: 'gh auth login',
    why: 'la commande de GitHub CLI, la même dans toutes les langues',
  },
  {
    file: 'src/components/settings/IntegrationsTab.svelte',
    text: 'claude-ready',
    why: 'un exemple d’étiquette, la même dans toutes les langues',
  },

  // L6 — board, stats

  // L7 — nav, git
  {
    file: 'src/components/TitleBar.svelte',
    text: 'Δ',
    why: 'le symbole de la différence, avant le nombre de modifications git : le même dans toutes les langues',
  },

  // L8 — conv, composer
  { file: 'src/components/Conversation.svelte', text: 'C', why: 'l’initiale de Claude dans l’avatar de ses messages : un nom propre' },

  // L9 — runs, shell

  // K, M, G — accounts, mcp, branches
];

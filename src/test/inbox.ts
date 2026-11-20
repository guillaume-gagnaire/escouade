// Test utilities of the box « À toi » (ESC-510): requests as the core gives them to the window, and its file command.

import type { AskFile, WindowAsk } from '../lib/types';

/** A 1 × 1 PNG, in base64. */
export const PNG_1PX = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';

export function askFile(over: Partial<AskFile> = {}): AskFile {
  return {
    file: `u${Math.random().toString(36).slice(2, 8)}/${over.name ?? 'a.png'}`,
    name: 'a.png',
    kind: 'image',
    mediaType: 'image/png',
    size: 70,
    option: null,
    byUser: false,
    ...over,
  };
}

export function windowAsk(over: Partial<WindowAsk> = {}): WindowAsk {
  const now = Date.now();
  return {
    id: 'a-1',
    projectId: 'p1',
    ticketId: null,
    author: { agent: 'a1', name: 'design-tableau', chef: false },
    kind: 'choice',
    title: 'Choisis la direction du tableau de bord',
    body: 'J’ai rendu trois directions.',
    options: [
      { id: 'a', label: 'Colonnes de chiffres', description: 'Une barre à gauche.' },
      { id: 'b', label: 'Cartes empilées', description: 'Les cartes du mobile.' },
      { id: 'c', label: 'Tableau dense', description: 'Tout en lignes.' },
    ],
    multi: false,
    steps: [],
    recommend: { option: 'b', text: null, why: 'Les adhérents arrivent du mobile.' },
    unblocks: 'DEM-13 et DEM-14 démarrent sur la direction choisie',
    tickets: [],
    urgency: 'normal',
    expiresAt: now + 14 * 86_400_000,
    decision: false,
    wake: true,
    yesLabel: '',
    noLabel: '',
    gate: false,
    files: [],
    state: 'open',
    createdAt: now - 40 * 60_000,
    updatedAt: now - 40 * 60_000,
    snoozedUntil: null,
    woke: false,
    delegatedTo: null,
    answer: null,
    readAt: null,
    closedAt: null,
    reason: null,
    filesErasedAt: null,
    seen: true,
    history: [{ at: now - 40 * 60_000, what: 'deposited', detail: 'design-tableau' }],
    projectName: 'demo-api',
    private: false,
    ticketKey: null,
    delegatedName: null,
    delegates: [],
    ...over,
  };
}

/** The renders of a choice of three options, dark and light. */
export const renders = () =>
  ['a', 'b', 'c'].flatMap((o) => [
    askFile({ file: `${o}1/${o}.png`, name: `${o}.png`, option: o }),
    askFile({ file: `${o}2/${o}-clair.png`, name: `${o}-clair.png`, option: o }),
  ]);

/** `inbox_file`'s answer: the PNG of one pixel for an image, some bytes for anything else. */
export function inboxFileAnswer(a: { id: string; file: string; offset?: number }) {
  const png = a.file.endsWith('.png');
  const data = png ? PNG_1PX : btoa(a.file.endsWith('.md') ? '# Notes\n\n- un point' : '%PDF-1.7 fake');
  return {
    name: a.file.split('/').pop(),
    kind: png ? 'image' : 'other',
    mediaType: png ? 'image/png' : 'application/octet-stream',
    size: atob(data).length,
    offset: a.offset ?? 0,
    data: (a.offset ?? 0) > 0 ? '' : data,
  };
}

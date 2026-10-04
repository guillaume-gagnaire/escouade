// The demo board of demo-api, shared by the board scenes.

import type { FC, ReactNode } from 'react';
import { AGENTS } from '../data';
import { BoardColumn, BoardHeader, COLUMNS, TicketCard, type Column, type Ticket } from '../ui/Board';
import type { AgentInfo } from '../ui/Sidebar';

export const CRITERIA = [
  'Au-delà de 5 échecs en 15 min, la connexion est refusée',
  'Le message d’erreur dit quand réessayer',
  'Tests verts',
];
export const PROGRESS = [
  'Compteur d’échecs par IP et par compte',
  'Limiteur branché sur /login',
  'Message « Réessaie dans 15 min »',
  'Tests du limiteur',
];
export const TITLE = 'Limiter les tentatives de connexion';
export const DESCRIPTION = 'Bloquer les attaques par force brute sur /login, sans gêner les vrais utilisateurs.';

export const DONE: Ticket[] = [
  {
    key: 'DEM-3',
    title: 'Pagination de /users',
    column: 'done',
    outcome: '⇡ Poussé sur ticket/dem-3',
    doneMeta: '2 boucles · 0,61 $',
    agent: { name: 'dem-3-pagination-de-users', status: 'done' },
  },
  {
    key: 'DEM-2',
    title: 'Webhooks Stripe',
    column: 'done',
    outcome: '⇡ PR #41 → main',
    doneMeta: '3 boucles · 1,12 $',
    agent: { name: 'dem-2-webhooks-stripe', status: 'done' },
  },
  {
    key: 'DEM-1',
    title: 'Réinitialisation du mot de passe',
    column: 'done',
    outcome: '⤵ Mergé dans main · squash',
    doneMeta: '1 boucle · 0,48 $',
    agent: { name: 'dem-1-reinitialisation-du-mot-de-passe', status: 'done' },
  },
];

export const CSV: Ticket = {
  key: 'DEM-5',
  title: 'Export CSV des factures',
  column: 'todo',
  loop: [1, 5],
  criteria: [
    { text: 'Le CSV reprend les filtres de la liste', ok: false },
    { text: 'Montants au format français', ok: false },
    { text: 'Tests verts', ok: false },
  ],
};

export const LOGIN: Ticket = {
  key: 'DEM-6',
  title: TITLE,
  column: 'todo',
  loop: [1, 5],
  criteria: CRITERIA.map((text) => ({ text, ok: false })),
};

export const SEARCH: Ticket = {
  key: 'DEM-4',
  title: 'Recherche plein texte des clients',
  column: 'review',
  partial: true,
  criteria: [
    { text: 'Recherche par nom, email et société', ok: true },
    { text: 'Résultats en moins de 200 ms', ok: false },
    { text: 'Tests verts', ok: true },
  ],
  progress: ['Index plein texte sur les clients', 'Route GET /customers/search', 'Tests de la recherche'],
  agent: { name: 'dem-4-recherche-plein-texte-des-clients', status: 'done' },
};

export const AGENT_CSV = 'dem-5-export-csv-des-factures';
export const AGENT_LOGIN = 'dem-6-limiter-les-tentatives-de';

/** The sidebar's agents on the board scenes: three of the project's, then the tickets' own. */
export const boardAgents = (tickets: { name: string; tag: string; status?: AgentInfo['status']; enter?: number }[]): AgentInfo[] => [
  ...AGENTS.slice(0, 2),
  ...tickets.map((t): AgentInfo => ({
    name: t.name,
    status: t.status ?? 'running',
    model: 'Opus 5.5',
    time: '2m 10s',
    tokens: '38 k',
    cost: '≈ 0,41 $',
    files: 4,
    ticket: t.tag,
    branch: `ticket/${t.name.slice(0, 5)}`,
  })),
];

export const BoardView: FC<{
  tickets: Ticket[];
  places: string;
  autopilot: boolean;
  summary?: string;
  glow?: Partial<Record<Column, number>>;
  form?: ReactNode;
  plusPressed?: number;
  pressedCfg?: number;
  pressedImport?: number;
}> = ({ tickets, places, autopilot, summary = 'merge squash → main', glow = {}, form, plusPressed, pressedCfg, pressedImport }) => (
  <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
    <BoardHeader
      project="demo-api"
      count={tickets.length}
      looping={tickets.filter((t) => t.column === 'doing').length}
      places={places}
      summary={summary}
      autopilot={autopilot}
      pressedCfg={pressedCfg}
      pressedImport={pressedImport}
    />
    <div
      style={{
        flex: 1,
        minHeight: 0,
        display: 'grid',
        gridTemplateColumns: 'repeat(4, minmax(0, 1fr))',
        gap: 12,
        padding: '16px 18px 18px',
      }}
    >
      {COLUMNS.map((c) => {
        const mine = tickets.filter((t) => t.column === c.id);
        return (
          <BoardColumn key={c.id} column={c.id} count={mine.length} glow={glow[c.id]} plusPressed={c.id === 'todo' ? plusPressed : 0}>
            {c.id === 'todo' ? form : null}
            {mine.length ? mine.map((t) => <TicketCard key={t.key} t={t} />) : form && c.id === 'todo' ? null : undefined}
          </BoardColumn>
        );
      })}
    </div>
  </div>
);

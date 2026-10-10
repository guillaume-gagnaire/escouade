// The demo board of demo-api, shared by the board scenes.

import type { FC, ReactNode } from 'react';
import { demoOf } from '../data';
import { useFmt, type Fmt, type Tr } from '../lang';
import { BoardColumn, BoardHeader, columnsOf, TicketCard, type Column, type Ticket } from '../ui/Board';
import type { AgentInfo } from '../ui/Sidebar';

/** The name of a ticket's agent, as the app makes it: its key and its title, cut at 40 characters on a word. */
export function ticketAgent(key: string, title: string): string {
  const slug = `${key}-${title}`
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  if (slug.length <= 40) return slug;
  const cut = slug.slice(0, 40);
  return slug[40] === '-' ? cut : cut.slice(0, cut.lastIndexOf('-'));
}

/** What the board's header says of the places left for agents. */
export const placesText = (tr: Tr, free: number) =>
  free <= 0
    ? tr('Toutes les places sont prises', 'All slots taken')
    : free === 1
      ? tr('1 place libre', '1 free slot')
      : tr(`${free} places libres`, `${free} free slots`);

/** The board's tickets and texts, in a language. */
export interface BoardData {
  CRITERIA: string[];
  PROGRESS: string[];
  TITLE: string;
  DESCRIPTION: string;
  DONE: Ticket[];
  CSV: Ticket;
  LOGIN: Ticket;
  SEARCH: Ticket;
  AGENT_CSV: string;
  AGENT_LOGIN: string;
  /** The sidebar's agents on the board scenes: three of the project's, then the tickets' own. */
  boardAgents: (tickets: { name: string; tag: string; status?: AgentInfo['status']; enter?: number }[]) => AgentInfo[];
}

export function boardOf(fmt: Fmt): BoardData {
  const { tr, tok, usd, lang } = fmt;
  const CRITERIA = [
    tr('Au-delà de 5 échecs en 15 min, la connexion est refusée', 'After 5 failures in 15 min, login is refused'),
    tr('Le message d’erreur dit quand réessayer', 'The error message says when to retry'),
    tr('Tests verts', 'Tests pass'),
  ];
  const PROGRESS = [
    tr('Compteur d’échecs par IP et par compte', 'Failure counter per IP and per account'),
    tr('Limiteur branché sur /login', 'Limiter hooked to /login'),
    tr('Message « Réessaie dans 15 min »', 'Message “Try again in 15 min”'),
    tr('Tests du limiteur', 'Limiter tests'),
  ];
  const TITLE = tr('Limiter les tentatives de connexion', 'Limit login attempts');
  const DESCRIPTION = tr(
    'Bloquer les attaques par force brute sur /login, sans gêner les vrais utilisateurs.',
    'Block brute-force attacks on /login without bothering real users.',
  );
  const done = (key: string, title: string, outcome: string, loops: [string, string], cost: number): Ticket => ({
    key,
    title,
    column: 'done',
    outcome,
    doneMeta: tr(`${loops[0]} · ${usd(cost)}`, `${loops[1]} · ${usd(cost)}`),
    agent: { name: ticketAgent(key, title), status: 'done' },
  });
  const DONE: Ticket[] = [
    done(
      'DEM-3',
      tr('Pagination de /users', 'Pagination for /users'),
      tr('⇡ Poussé sur ticket/dem-3', '⇡ Pushed to ticket/dem-3'),
      ['2 boucles', '2 loops'],
      0.61,
    ),
    done('DEM-2', 'Webhooks Stripe', '⇡ PR #41 → main', ['3 boucles', '3 loops'], 1.12),
    done(
      'DEM-1',
      tr('Réinitialisation du mot de passe', 'Password reset'),
      tr('⤵ Mergé dans main · squash', '⤵ Merged into main · squash'),
      ['1 boucle', '1 loop'],
      0.48,
    ),
  ];
  const csvTitle = tr('Export CSV des factures', 'Invoice CSV export');
  const CSV: Ticket = {
    key: 'DEM-5',
    title: csvTitle,
    column: 'todo',
    loop: [1, 5],
    criteria: [
      { text: tr('Le CSV reprend les filtres de la liste', 'The CSV uses the list’s filters'), ok: false },
      { text: tr('Montants au format français', 'Amounts in euros, two decimals'), ok: false },
      { text: tr('Tests verts', 'Tests pass'), ok: false },
    ],
  };
  const LOGIN: Ticket = {
    key: 'DEM-6',
    title: TITLE,
    column: 'todo',
    loop: [1, 5],
    criteria: CRITERIA.map((text) => ({ text, ok: false })),
  };
  const searchTitle = tr('Recherche plein texte des clients', 'Full-text customer search');
  const SEARCH: Ticket = {
    key: 'DEM-4',
    title: searchTitle,
    column: 'review',
    partial: true,
    criteria: [
      { text: tr('Recherche par nom, email et société', 'Search by name, email and company'), ok: true },
      { text: tr('Résultats en moins de 200 ms', 'Results in under 200 ms'), ok: false },
      { text: tr('Tests verts', 'Tests pass'), ok: true },
    ],
    progress: [
      tr('Index plein texte sur les clients', 'Full-text index on customers'),
      'Route GET /customers/search',
      tr('Tests de la recherche', 'Search tests'),
    ],
    agent: { name: ticketAgent('DEM-4', searchTitle), status: 'done' },
  };
  const agents = demoOf(lang).agents;
  return {
    CRITERIA,
    PROGRESS,
    TITLE,
    DESCRIPTION,
    DONE,
    CSV,
    LOGIN,
    SEARCH,
    AGENT_CSV: ticketAgent('DEM-5', csvTitle),
    AGENT_LOGIN: ticketAgent('DEM-6', TITLE),
    boardAgents: (tickets) => [
      ...agents.slice(0, 2),
      ...tickets.map((t): AgentInfo => ({
        name: t.name,
        status: t.status ?? 'running',
        model: 'Opus 5.5',
        time: '2m 10s',
        tokens: tok(38),
        cost: `≈ ${usd(0.41)}`,
        files: 4,
        ticket: t.tag,
        branch: `ticket/${t.name.slice(0, 5)}`,
      })),
    ],
  };
}

/** The board of the picture being drawn: `const { CSV, LOGIN, boardAgents } = useBoard()`. */
export const useBoard = (): BoardData => boardOf(useFmt());

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
}> = ({ tickets, places, autopilot, summary, glow = {}, form, plusPressed, pressedCfg, pressedImport }) => {
  const { tr } = useFmt();
  return (
    <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
      <BoardHeader
        project="demo-api"
        count={tickets.length}
        looping={tickets.filter((t) => t.column === 'doing').length}
        places={places}
        summary={summary ?? tr('merge squash → main', 'squash merge → main')}
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
        {columnsOf(tr).map((c) => {
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
};

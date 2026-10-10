import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { TicketForm, type Ticket } from '../ui/Board';
import { Conversation, ConvHeader, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { ContextMenu } from '../ui/Modal';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Spotlight, Stage, Title } from '../ui/Stage';
import { BoardView, useBoard } from './boardCommon';
import { RefactoConv } from './common';

/** From the agents to the board: its columns, a new ticket, then the autopilot starting two agents. */
export const BoardScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd, pick } = useFmt();
  const { status: STATUS, refactor } = useDemo();
  const { AGENT_CSV, AGENT_LOGIN, boardAgents, CRITERIA, CSV, DESCRIPTION, DONE, LOGIN, TITLE } = useBoard();
  const open = c.word('switch', 'kanban') + 2;
  const glow = (word: string) => ramp(frame, c.word('switch', word) - 3, 6) - ramp(frame, c.word('switch', word) + 22, 10);
  const plus = c.at('ticket') - 4;
  const t = (word: string, nth = 0) => c.word('ticket', word, nth);
  const add = c.end('ticket') + 6;
  const formOpen = frame >= plus + 4 && frame < add + 2;
  const menuAt = c.at('manage');
  const m = (word: string) => c.word('manage', word);
  const byHand = c.word('auto', 'main');
  const auto = c.word('auto', 'pilote');
  const startCsv = c.word('auto', 'seuls');
  const startLogin = c.word('auto', 'ordre');
  const full = c.word('auto', 'parallèle');
  const created = frame >= add + 2;
  const autopilot = frame >= auto;
  const ticket = (base: Ticket, at: number, activity: string, agent: string): Ticket =>
    frame >= at
      ? { ...base, column: 'doing', enter: pop(frame, fps, at, 16), activity, agent: { name: agent, status: 'running' } }
      : {
          ...base,
          wait: autopilot
            ? tr("Pris dès qu'une place se libère", 'Picked up as soon as a slot frees up')
            : tr('Pilote auto désactivé', 'Autopilot off'),
          canStart: !autopilot,
        };
  const tickets: Ticket[] = [
    ticket(CSV, startCsv, tr('Lit src/billing/invoices.ts', 'Reads src/billing/invoices.ts'), AGENT_CSV),
    ...(created ? [ticket({ ...LOGIN, enter: pop(frame, fps, add + 2, 16) }, startLogin, tr('Réfléchit', 'Thinking'), AGENT_LOGIN)] : []),
    ...DONE,
  ];
  const criteria = [...CRITERIA.slice(0, Math.max(0, Math.floor(ramp(frame, t('critères'), 70) * 3.99)))];
  const board = frame >= open;
  const free = 2 - [startCsv, startLogin].filter((s) => frame >= s).length;
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={{ ...STATUS, active: 3 + (2 - free) }}
          sidebar={
            board ? (
              <AgentsSidebar
                board
                agents={boardAgents([
                  ...(frame >= startCsv ? [{ name: AGENT_CSV, tag: tr('DEM-5 · boucle 1/5', 'DEM-5 · loop 1/5') }] : []),
                  ...(frame >= startLogin ? [{ name: AGENT_LOGIN, tag: tr('DEM-6 · boucle 1/5', 'DEM-6 · loop 1/5') }] : []),
                ])}
                enters={[1, 1, pop(frame, fps, startCsv), pop(frame, fps, startLogin)]}
              />
            ) : (
              <AgentsSidebar agents={boardAgents([])} selected={refactor} />
            )
          }
          overlay={
            <>
              <ContextMenu
                x={480}
                y={372}
                enter={frame >= menuAt + 4 && frame < c.end('manage') + 8 ? pop(frame, fps, menuAt + 4, 20) : 0}
                hover={frame >= m('supprime') - 4 ? 3 : frame >= m('tête') - 4 ? 1 : frame >= m('modifie') - 4 ? 0 : undefined}
                items={[
                  tr('Modifier', 'Edit'),
                  tr('Passer en tête', 'Move to the top'),
                  null,
                  { label: tr('Supprimer', 'Delete'), danger: true },
                ]}
              />
              <Spotlight x={510} y={268} w={76} h={34} on={ramp(frame, byHand - 4, 8) - ramp(frame, auto - 10, 8)} />
              <PointerPath
                keys={[
                  [open - 30, 300, 300],
                  [open - 2, 218, 81, true],
                  [plus - 16, 450, 300],
                  [plus, 576, 154, true],
                  [t('boucles') - 12, pick(420, 408), pick(520, 484)],
                  [t('boucles') + 4, pick(434, 422), pick(494, 458), true],
                  [add - 4, pick(545, 556), pick(528, 476), true],
                  [menuAt - 16, 520, 450],
                  [menuAt + 2, 470, 362, true],
                  [m('modifie') - 4, 540, 395],
                  [m('tête') - 4, 560, 432],
                  [m('supprime') - 4, 540, 490],
                  [byHand - 6, 546, 290],
                  [auto - 20, 1300, 200],
                  [auto, 1450, 80, true],
                ]}
              />
            </>
          }
        >
          {board ? (
            <BoardView
              tickets={tickets}
              autopilot={autopilot}
              places={
                frame >= full - 6 && free === 0
                  ? tr('Toutes les places sont prises', 'All slots taken')
                  : free === 1
                    ? tr('1 place libre', '1 free slot')
                    : tr(`${free} places libres`, `${free} free slots`)
              }
              glow={{ todo: glow('faire'), doing: glow('cours'), review: glow('tester'), done: glow('terminé') }}
              plusPressed={ramp(frame, plus - 3, 3) - ramp(frame, plus + 3, 4)}
              form={
                formOpen ? (
                  <TicketForm
                    title={typed(TITLE, frame, fps, t('titre') - 4, 40)}
                    description={typed(DESCRIPTION, frame, fps, t('description') - 2, 60)}
                    criteria={criteria}
                    loops={5}
                    focus={frame < t('description') - 2 ? 'title' : frame < t('critères') ? 'description' : 'criteria'}
                    pressed={ramp(frame, add - 7, 3) - ramp(frame, add - 1, 3)}
                  />
                ) : null
              }
            />
          ) : (
            <>
              <ConvHeader
                name={refactor}
                status="running"
                sub="demo-api / main"
                metrics={{ model: 'Opus 5.5', tokens: tok(184), cost: usd(2.41), files: 6, duration: '12m 40s' }}
              />
              <Conversation>
                <RefactoConv />
                <Working />
              </Conversation>
              <Composer placeholder={tr(`Envoyer un message à ${refactor}…`, `Send a message to ${refactor}…`)} busy />
            </>
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { STATUS, TABS } from '../data';
import { C, MONO } from '../theme';
import { BoardSettings, type Ticket } from '../ui/Board';
import { PointerPath } from '../ui/Cursor';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Stage, Title } from '../ui/Stage';
import { AGENT_CSV, AGENT_LOGIN, boardAgents, BoardView, CRITERIA, CSV, DONE, LOGIN, PROGRESS, SEARCH } from './boardCommon';

const REASON = 'Le séparateur doit être un point-virgule, pour Excel.';

/** « Valider et merger »: tests, generated commit, merge; the board's settings; a ticket sent back; the finished ones. */
export const Validate: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const o = (w: string) => c.word('ok', w);
  const approve = o('valide') + 2;
  const merged = o('fusionne') + 16;
  const cfgClick = c.at('settings') - 2;
  const s = (w: string) => c.word('settings', w);
  const closeCfg = c.end('settings') + 8;
  const csvReview = c.at('reject') - 20;
  const rejectClick = c.word('reject', 'renvoie') + 2;
  const sendBack = c.word('reject', 'si') - 8;
  const failClick = c.word('reject', 'test');
  const backAt = c.word('reject', 'repart');
  const step =
    frame < approve + 4
      ? undefined
      : frame < o('commite') - 2
        ? 'Tests…'
        : frame < o('fusionne')
          ? 'Commit…'
          : frame < merged
            ? 'Merge…'
            : undefined;
  const login: Ticket =
    frame >= merged
      ? {
          ...LOGIN,
          column: 'done',
          enter: pop(frame, fps, merged, 16),
          outcome: '⤵ Mergé dans main · squash',
          doneMeta: '3 boucles · 1,86 $',
          agent: { name: AGENT_LOGIN, status: 'done' },
        }
      : {
          ...LOGIN,
          column: 'review',
          criteria: CRITERIA.map((text) => ({ text, ok: true })),
          progress: PROGRESS,
          test: true,
          step,
          agent: { name: AGENT_LOGIN, status: 'done' },
          pressed: { approve: ramp(frame, approve - 3, 3) - ramp(frame, approve + 3, 4) },
        };
  const csvInReview = frame >= csvReview && frame < sendBack;
  const csv: Ticket = csvInReview
    ? {
        ...CSV,
        column: 'review',
        enter: pop(frame, fps, csvReview, 16),
        criteria: CSV.criteria!.map((x) => ({ ...x, ok: true })),
        progress: ['Route GET /invoices.csv', 'Montants en 1 234,56 €', 'Filtres de la liste repris'],
        agent: { name: AGENT_CSV, status: 'done' },
        rejecting: frame >= rejectClick + 4 ? typed(REASON, frame, fps, rejectClick + 8, 40) : undefined,
        pressed: {
          reject:
            frame >= rejectClick + 4
              ? ramp(frame, sendBack - 6, 3) - ramp(frame, sendBack, 3)
              : ramp(frame, rejectClick - 3, 3) - ramp(frame, rejectClick + 3, 4),
        },
      }
    : {
        ...CSV,
        column: 'doing',
        enter: frame >= sendBack ? pop(frame, fps, sendBack, 16) : 1,
        loop: [frame >= sendBack ? 1 : 2, 5],
        criteria: CSV.criteria!.map((x, i) => ({ ...x, ok: frame >= sendBack || i < 2 })),
        progress: ['Route GET /invoices.csv', 'Montants en 1 234,56 €'],
        activity: frame >= sendBack ? 'Réfléchit' : 'Lance npm test',
        agent: { name: AGENT_CSV, status: 'running' },
      };
  const search: Ticket =
    frame < backAt
      ? {
          ...SEARCH,
          step: frame >= failClick + 4 ? 'Tests…' : undefined,
          pressed: { approve: ramp(frame, failClick - 3, 3) - ramp(frame, failClick + 3, 4) },
        }
      : {
          ...SEARCH,
          column: 'doing',
          partial: false,
          loop: [1, 5],
          enter: pop(frame, fps, backAt, 16),
          activity: 'Réfléchit',
          agent: { name: SEARCH.agent!.name, status: 'running' },
        };
  const tickets = [csv, login, search, ...DONE];
  const doing = tickets.filter((t) => t.column === 'doing').length;
  const review = tickets.filter((t) => t.column === 'review').length;
  const doneGlow = ramp(frame, c.word('done', 'terminés') - 4, 8) - ramp(frame, c.end('done') + 10, 10);
  const parallel = frame >= s('parallèle') + 6 ? 3 : 2;
  const free = Math.max(0, parallel - doing);
  const commitMsg = ramp(frame, o('message') - 6, 8) - ramp(frame, o('fusionne') + 4, 8);
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              board
              badge={review || undefined}
              archived={frame >= merged ? 1 : undefined}
              agents={boardAgents([
                {
                  name: AGENT_CSV,
                  tag: csvInReview ? 'DEM-5 · à tester' : `DEM-5 · boucle ${frame >= sendBack ? 1 : 2}/5`,
                  status: csvInReview ? 'done' : 'running',
                },
                ...(frame < merged ? [{ name: AGENT_LOGIN, tag: 'DEM-6 · à tester', status: 'done' as const }] : []),
                {
                  name: SEARCH.agent!.name,
                  tag: search.column === 'review' ? 'DEM-4 · à tester' : 'DEM-4 · boucle 1/5',
                  status: search.column === 'review' ? 'done' : 'running',
                },
              ])}
            />
          }
          overlay={
            <>
              {commitMsg > 0 ? (
                <div
                  style={{
                    position: 'absolute',
                    left: 620,
                    top: 640,
                    padding: '12px 16px',
                    borderRadius: 10,
                    background: C.elev,
                    border: `1px solid ${C.spark}`,
                    boxShadow: '0 20px 60px rgba(0,0,0,0.55)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 6,
                    opacity: commitMsg,
                    zIndex: 30,
                  }}
                >
                  <span style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: C.muted }}>
                    Message de commit généré
                  </span>
                  <span style={{ fontFamily: MONO, fontSize: 15 }}>feat(auth): limiter les tentatives de connexion [DEM-6]</span>
                </div>
              ) : null}
              <BoardSettings
                enter={frame >= closeCfg ? 0 : pop(frame, fps, cfgClick + 4, 18)}
                action={0}
                hover={frame >= s('pousser') ? 2 : frame >= s('pull') ? 1 : undefined}
                glow={frame >= s('parallèle') - 4 ? 'agents' : frame >= s('conflits') - 4 ? 'conflicts' : undefined}
                parallel={parallel}
              />
              <PointerPath
                keys={[
                  [approve - 24, 1000, 400],
                  [approve, 990, 555, true],
                  [cfgClick - 24, 1100, 300],
                  [cfgClick, 1175, 81, true],
                  [rejectClick - 24, 1000, 400],
                  [rejectClick, 1112, 447, true],
                  [sendBack - 6, 1126, 501, true],
                  [failClick - 16, 1040, 520],
                  [failClick, 990, 447, true],
                ]}
              />
            </>
          }
        >
          <BoardView
            tickets={tickets}
            autopilot
            places={free ? `${free} ${free > 1 ? 'places libres' : 'place libre'}` : 'Toutes les places sont prises'}
            glow={{ done: doneGlow }}
            pressedCfg={ramp(frame, cfgClick - 3, 3) - ramp(frame, cfgClick + 3, 4)}
          />
        </Shell>
      </AppWindow>
    </Stage>
  );
};

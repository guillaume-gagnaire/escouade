import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { AGENTS, STATUS, TABS } from '../data';
import { C } from '../theme';
import type { Ticket } from '../ui/Board';
import { Conversation, ConvHeader, UserMsg, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { RunsSection, LogView } from '../ui/Runs';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Sfx, Spotlight, Stage, Title } from '../ui/Stage';
import { Browser, TestLaunchModal, type Step, type StepState } from '../ui/TestLaunch';
import { WinToast } from '../ui/Toast';
import { AGENT_CSV, AGENT_LOGIN, boardAgents, BoardView, CRITERIA, CSV, DONE, LOGIN, PROGRESS, SEARCH } from './boardCommon';

/** The ticket's worktree, as the app shortens it. */
const WORKTREE = '~\\dev\\demo-api\\.claude\\worktrees\\dem-6';
const LOG = [
  '$ npm run dev -- --port 4100',
  '',
  '> demo-api@2.4.0 dev',
  '> tsx watch src/server.ts',
  '',
  '  API listening on http://localhost:4100',
  '  GET /health 200 3 ms',
  '  POST /login 401 41 ms',
  '  POST /login 429 2 ms  (limite : 5 échecs / 15 min)',
];

/** The ticket goes to test; « ▶ Tester » prepares, starts, waits, opens the feature; the logs; preparing any worktree's test. */
export const TestScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const toReview = c.word('ready', 'tester') - 6;
  const notified = c.word('ready', 'prévenu');
  const testClick = c.at('run') + 4;
  const modal = testClick + 6;
  const prep = c.word('run', 'prépare');
  const servers = c.word('run', 'serveurs');
  const answered = c.word('run', 'répondent');
  const opened = c.at('open') + 4;
  const logsClick = c.at('logs') + 2;
  const logs = frame >= logsClick + 6;
  const prepare = c.word('prepare', 'préparer') + 4;
  const prepareView = frame >= c.at('prepare') - 10;
  const reviewed = frame >= toReview;
  const partialAt = c.word('ready', 'objectif') - 4;
  const state = (from: number, done: number): StepState => (frame < from ? 'waiting' : frame < done ? 'running' : 'ready');
  const process = (from: number, done: number, port: number, seconds: string) =>
    frame >= done ? `prêt · ${seconds} s` : frame >= from + 10 ? `en attente de localhost:${port}…` : 'démarrage…';
  const steps: Step[] = [
    { label: 'Préparation : npm install', detail: frame >= servers - 6 ? 'terminée' : 'en cours…', state: state(prep, servers - 6) },
    { label: 'api', detail: process(servers, answered, 4100, '2,4'), state: state(servers, answered) },
    { label: 'web', detail: process(servers, answered + 8, 4101, '2,7'), state: state(servers, answered + 8) },
  ].filter((l) => l.state !== 'waiting');
  const login: Ticket = reviewed
    ? {
        ...LOGIN,
        column: 'review',
        enter: pop(frame, fps, toReview, 16),
        criteria: CRITERIA.map((text) => ({ text, ok: true })),
        progress: PROGRESS,
        test: true,
        testing: frame >= servers,
        agent: { name: AGENT_LOGIN, status: 'done' },
        pressed: { test: ramp(frame, testClick - 3, 3) - ramp(frame, testClick + 3, 4) },
      }
    : {
        ...LOGIN,
        column: 'doing',
        loop: [3, 5],
        criteria: CRITERIA.map((text, i) => ({ text, ok: i < 2 || frame >= c.word('ready', 'critères') })),
        progress: PROGRESS,
        activity: 'Lance npm test',
        agent: { name: AGENT_LOGIN, status: 'running' },
      };
  const csv: Ticket = {
    ...CSV,
    column: 'doing',
    loop: [2, 5],
    criteria: CSV.criteria!.map((x, i) => ({ ...x, ok: i < 2 })),
    progress: ['Route GET /invoices.csv', 'Montants en 1 234,56 €'],
    activity: 'Lance npm test',
    agent: { name: AGENT_CSV, status: 'running' },
  };
  const browserX = 1920 - 1100 * Math.min(1, pop(frame, fps, opened + 4, 18)) + 1100 * ramp(frame, c.at('logs') - 8, 14);
  const testing = frame >= servers;
  const groups =
    testing && !prepareView
      ? [
          {
            agent: AGENT_LOGIN,
            runs: [
              { name: 'Préparation 1', status: 'done' as const },
              { name: 'api', status: 'running' as const },
              { name: 'web', status: 'running' as const },
            ],
          },
        ]
      : [];
  return (
    <Stage>
      <Title />
      <Sfx at={notified} name="chime" />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              board={!logs}
              badge={reviewed ? 1 : undefined}
              selected={prepareView ? 'pagination-users' : logs ? undefined : undefined}
              agents={
                prepareView
                  ? AGENTS.slice(0, 3)
                  : boardAgents([
                      { name: AGENT_CSV, tag: 'DEM-5 · boucle 2/5' },
                      {
                        name: AGENT_LOGIN,
                        tag: reviewed ? 'DEM-6 · à tester' : 'DEM-6 · boucle 3/5',
                        status: reviewed ? 'done' : 'running',
                      },
                    ])
              }
              foot={false}
              after={
                <RunsSection
                  runs={[
                    { name: 'Front', status: 'ready' },
                    { name: 'API', status: 'ready' },
                  ]}
                  groups={groups}
                  selected={logs && !prepareView ? `${AGENT_LOGIN}/api` : undefined}
                />
              }
            />
          }
          overlay={
            <>
              <TestLaunchModal
                title="Tester DEM-6"
                steps={steps}
                opened={frame >= opened ? 'http://localhost:4101/login' : undefined}
                enter={frame >= logsClick + 4 ? 0 : pop(frame, fps, modal, 18)}
                pressedLogs={ramp(frame, logsClick - 3, 3) - ramp(frame, logsClick + 3, 3)}
              />
              <Spotlight
                x={4}
                y={592}
                w={292}
                h={210}
                on={ramp(frame, c.word('logs', 'lancement') - 6, 8) - ramp(frame, c.end('logs') + 10, 8)}
              />
              <Spotlight x={736} y={62} w={172} h={40} on={ramp(frame, prepare - 18, 8) - ramp(frame, c.end('prepare') + 6, 8)} />
              <PointerPath
                keys={[
                  [testClick - 24, 1000, 560],
                  [testClick, 952, 494, true],
                  [logsClick - 24, 900, 600],
                  [logsClick, 806, 512, true],
                  [prepare - 22, 1000, 300],
                  [prepare, 822, 81, true],
                ]}
              />
            </>
          }
        >
          {prepareView ? (
            <>
              <ConvHeader
                name="pagination-users"
                status={frame >= prepare + 6 ? 'running' : 'idle'}
                sub="demo-api / escouade/pagination-users"
                test="prepare"
                metrics={{ model: 'Opus 5.5', tokens: '24,8 k', cost: '0,27 $', files: 3, duration: '4m 02s' }}
                pressed={{ test: ramp(frame, prepare - 3, 3) - ramp(frame, prepare + 3, 4) }}
              />
              <Conversation>
                {frame >= prepare + 6 ? (
                  <>
                    <UserMsg
                      text="Prépare le lancement de test de ce worktree. Ports réservés : 4110 à 4119 (ESCOUADE_PORT_BASE et ESCOUADE_PORT_END dans le lancement)…"
                      enter={pop(frame, fps, prepare + 6)}
                    />
                    <Working />
                  </>
                ) : null}
              </Conversation>
              <Composer placeholder="Envoyer un message à pagination-users…" model="Opus 5.5" busy={frame >= prepare + 6} />
            </>
          ) : logs ? (
            <LogView
              title="api"
              status="en cours"
              state="running"
              statusColor={C.ok}
              lines={LOG}
              command="npm run dev -- --port 4100"
              where={WORKTREE}
            />
          ) : (
            <BoardView
              tickets={[
                csv,
                login,
                ...(frame >= partialAt ? [{ ...SEARCH, test: true, enter: pop(frame, fps, partialAt, 16) }] : []),
                ...DONE,
              ]}
              autopilot
              places={reviewed ? '1 place libre' : 'Toutes les places sont prises'}
            />
          )}
        </Shell>
      </AppWindow>
      <Browser x={browserX} url="localhost:4101/login" tries={frame >= opened + 40 ? 6 : 5} />
      <WinToast title="demo-api" body="DEM-6 prêt à tester" enter={pop(frame, fps, notified, 16) - ramp(frame, c.at('run') + 10, 14)} />
    </Stage>
  );
};

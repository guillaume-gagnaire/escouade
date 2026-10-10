import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C, MONO } from '../theme';
import type { Ticket } from '../ui/Board';
import { AssistantMsg, Conversation, ConvHeader, CriteriaReport, Diffstat, ToolCall, UserMsg, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, camPath, Spotlight, Stage, Title } from '../ui/Stage';
import { WinToast } from '../ui/Toast';
import { BoardView, placesText, useBoard } from './boardCommon';

/** The ticket's own worktree and ports; its agent loops on its criteria; a blocked ticket resumes; its report in the conversation. */
export const Loop: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd } = useFmt();
  const { status: STATUS } = useDemo();
  const { AGENT_CSV, AGENT_LOGIN, boardAgents, CRITERIA, CSV, DONE, LOGIN, PROGRESS } = useBoard();
  const BLOCKED = tr("Erreur : Claude Code s'est arrêté (code 1)", 'Error: Claude Code stopped (code 1)');
  const FACTS: [string, string][] = [
    ['agent', AGENT_LOGIN],
    [tr('branche', 'branch'), 'ticket/dem-6'],
    ['worktree', '.claude/worktrees/dem-6'],
    [tr('copiés', 'copied'), '.env  .env.local'],
    ['ports', '4100 → 4109'],
  ];
  const factAt = [
    c.word('setup', 'agent'),
    c.word('setup', 'branche'),
    c.word('setup', 'worktree'),
    c.word('setup', 'fichiers'),
    c.word('setup', 'ports'),
  ];
  const facts = ramp(frame, factAt[0] - 6, 8) - ramp(frame, c.end('setup') + 10, 10);
  const firstOk = c.word('loop', 'bilan') + 10;
  const loop2 = c.word('loop', 'repart');
  const secondOk = c.word('card', 'critères');
  const blockedAt = c.at('blocked') - 6;
  const resume = c.word('blocked', 'clic') + 4;
  const open = c.at('report') - 8;
  const report = frame >= open;
  const progress = PROGRESS.slice(0, frame < loop2 ? 1 : frame < c.word('card', 'place') ? 2 : 4);
  const activity =
    frame < c.at('loop')
      ? tr('Lit src/auth/login.ts', 'Reads src/auth/login.ts')
      : frame < loop2
        ? tr('Lance npm test', 'Runs npm test')
        : frame < c.word('card', 'moment')
          ? tr('Modifie src/auth/limiter.ts', 'Edits src/auth/limiter.ts')
          : tr('Lance npm test -- limiter', 'Runs npm test -- limiter');
  const login: Ticket = {
    ...LOGIN,
    column: 'doing',
    loop: [frame < loop2 ? 1 : 2, 5],
    criteria: CRITERIA.map((text, i) => ({ text, ok: (i === 0 && frame >= firstOk) || (i === 1 && frame >= secondOk) })),
    progress,
    activity,
    agent: { name: AGENT_LOGIN, status: 'running' },
  };
  const blocked = frame >= blockedAt && frame < resume;
  const csv: Ticket = {
    ...CSV,
    column: 'doing',
    loop: [1, 5],
    criteria: CSV.criteria!.map((x, i) => ({ ...x, ok: i === 0 })),
    progress: ['Route GET /invoices.csv'],
    activity: frame >= resume ? tr('Lance npm test', 'Runs npm test') : tr('Lit src/billing/invoices.ts', 'Reads src/billing/invoices.ts'),
    blocked: blocked ? BLOCKED : undefined,
    agent: { name: AGENT_CSV, status: blocked ? 'error' : 'running' },
    pressed: { resume: ramp(frame, resume - 3, 3) - ramp(frame, resume + 3, 4) },
  };
  const free = blocked && frame >= c.word('blocked', 'place') - 4 ? 1 : 0;
  const cam = camPath(frame, [
    [c.at('card') - 6, 24, { x: 760, y: 470, zoom: 1.65 }],
    [c.end('card') + 2, 20, { x: 750, y: 422, zoom: 1 }],
  ]);
  return (
    <Stage>
      <Title out={cam.zoom > 1 ? Math.min(1, (cam.zoom - 1) * 4) : 0} />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          status={{ ...STATUS, active: 5 }}
          sidebar={
            <AgentsSidebar
              board={!report}
              badge={undefined}
              selected={report ? AGENT_LOGIN : undefined}
              agents={boardAgents([
                { name: AGENT_CSV, tag: tr('DEM-5 · boucle 1/5', 'DEM-5 · loop 1/5'), status: blocked ? 'error' : 'running' },
                {
                  name: AGENT_LOGIN,
                  tag: tr(
                    `DEM-6 · boucle ${frame >= c.word('report', 'latérale') ? 3 : frame < loop2 ? 1 : 2}/5`,
                    `DEM-6 · loop ${frame >= c.word('report', 'latérale') ? 3 : frame < loop2 ? 1 : 2}/5`,
                  ),
                },
              ])}
            />
          }
          overlay={
            <>
              {facts > 0 ? (
                <div
                  style={{
                    position: 'absolute',
                    left: 908,
                    top: 300,
                    width: 470,
                    padding: '14px 18px',
                    borderRadius: 12,
                    background: C.elev,
                    border: `1px solid ${C.spark}`,
                    boxShadow: '0 20px 60px rgba(0,0,0,0.55)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 8,
                    opacity: facts,
                    zIndex: 30,
                  }}
                >
                  {FACTS.map(([k, v], i) => (
                    <span
                      key={k}
                      style={{
                        display: 'flex',
                        gap: 14,
                        fontFamily: MONO,
                        fontSize: 15,
                        opacity: Math.min(1, pop(frame, fps, factAt[i] - 4)),
                      }}
                    >
                      <span style={{ width: 90, color: C.dim }}>{k}</span>
                      <span style={{ color: i === 4 ? C.spark : C.text }}>{v}</span>
                    </span>
                  ))}
                </div>
              ) : null}
              <Spotlight
                x={12}
                y={466}
                w={278}
                h={118}
                on={ramp(frame, c.word('report', 'latérale') - 6, 8) - ramp(frame, c.end('report') + 14, 8)}
              />
              <PointerPath
                keys={[
                  [resume - 26, 700, 560],
                  [resume, 686, 406, true],
                  [open - 22, 700, 600],
                  [open, 748, 520, true],
                ]}
              />
            </>
          }
        >
          {report ? (
            <>
              <ConvHeader
                name={AGENT_LOGIN}
                status="running"
                sub="demo-api / ticket/dem-6"
                metrics={{ model: 'Opus 5.5', tokens: tok(61), cost: `≈ ${usd(0.84)}`, files: 4, duration: '6m 12s' }}
              />
              <Conversation>
                <ToolCall tool="Edit" target="src/auth/limiter.ts" meta={<Diffstat add={64} del={0} />} />
                <ToolCall tool="Bash" target="npm test -- limiter" meta={<span style={{ fontSize: 13, color: C.muted }}>9 tests</span>} />
                <AssistantMsg
                  text={tr(
                    'Le limiteur bloque bien au sixième essai, avec un message qui dit quand réessayer.',
                    'The limiter does block on the sixth try, with a message that says when to retry.',
                  )}
                >
                  <CriteriaReport
                    criteria={[
                      { text: CRITERIA[0], ok: true, note: tr('vérifié par tests/limiter.test.ts', 'verified by tests/limiter.test.ts') },
                      { text: CRITERIA[1], ok: true, note: tr('« Réessaie dans 15 min »', '“Try again in 15 min”') },
                      { text: CRITERIA[2], ok: false, note: tr('un test d’intégration échoue encore', 'an integration test still fails') },
                    ]}
                    progress={PROGRESS}
                    enter={pop(frame, fps, c.word('report', 'bilan') - 6)}
                  />
                </AssistantMsg>
                <UserMsg
                  text={tr(
                    "Boucle 3/5. Critères non atteints : 3 (un test d'intégration échoue encore). Continue jusqu'à les atteindre, puis termine par le bilan.",
                    'Loop 3/5. Criteria not met: 3 (an integration test still fails). Keep going until you meet them, then finish with the report.',
                  )}
                  enter={pop(frame, fps, c.word('report', 'latérale'))}
                />
                <Working />
              </Conversation>
              <Composer placeholder={tr(`Envoyer un message à ${AGENT_LOGIN}…`, `Send a message to ${AGENT_LOGIN}…`)} busy />
            </>
          ) : (
            <BoardView tickets={[csv, login, ...DONE]} autopilot places={placesText(tr, free)} />
          )}
        </Shell>
      </AppWindow>
      <WinToast
        title="demo-api"
        body={tr(`DEM-5 bloqué : ${BLOCKED}`, `DEM-5 blocked: ${BLOCKED}`)}
        enter={pop(frame, fps, c.word('blocked', 'prévient'), 16) - ramp(frame, resume, 10)}
      />
    </Stage>
  );
};

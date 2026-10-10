import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import { AssistantMsg, Conversation, ConvHeader, QuestionCard, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { OtherWindow, Taskbar } from '../ui/Desktop';
import { KeyPress } from '../ui/Keys';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { PointerPath } from '../ui/Cursor';
import { AppWindow, Sfx, Spotlight, Stage, Title } from '../ui/Stage';
import { WinToast } from '../ui/Toast';
import { RefactoConv } from './common';

/** An agent waits: its card and the tabs blink, the chime, a Windows notification, Ctrl+J; closing keeps everything going. */
export const Notify: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd } = useFmt();
  const { agents: AGENTS, status: STATUS, refactor } = useDemo();
  const asks = c.at('ask') + 10;
  const away = c.word('toast', 'arrière-plan') - 6;
  const toast = c.word('toast', 'notification');
  // Back in front during the breath before Ctrl+J: the other window is gone when the keys show.
  const back = c.end('toast') + 2;
  const jumped = c.word('jump', 'contrôle') + 8;
  const answered = c.end('jump') + 6;
  const closeAt = c.word('tray', 'fermes');
  const reopen = c.word('tray', 'redémarrage') - 4;
  const waiting = frame >= asks && frame < answered;
  const finished = frame >= asks + 40;
  const agents = AGENTS.map((a) => (a.name === 'tests-e2e' && waiting ? { ...a, status: 'waiting' as const } : a));
  const tabs = TABS.map((t, i) =>
    i === 0 && waiting ? { ...t, waiting: 1 } : i === 2 && finished && frame < reopen ? { ...t, running: false, alert: C.ok } : t,
  );
  // The window: in front, behind another app, back, closed to the tray, back after a restart.
  const behind = ramp(frame, away, 14) - ramp(frame, back, 14);
  const closed = ramp(frame, closeAt, 16) - ramp(frame, reopen, 18);
  const taskbar = Math.max(behind, closed, frame >= away && frame < reopen + 30 ? 1 : 0);
  return (
    <Stage>
      <Title />
      <Sfx at={c.word('ask', 'carillon')} name="chime" />
      <Sfx at={toast} name="chime" />
      <div
        style={{
          opacity: 1 - 0.45 * behind - closed,
          transform: `translate(${closed * 760}px, ${closed * 420}px) scale(${1 - 0.06 * behind - 0.9 * closed})`,
          transformOrigin: '50% 50%',
        }}
      >
        <AppWindow>
          <Shell
            tabs={tabs}
            status={{ ...STATUS, waiting: waiting ? 1 : 0, active: waiting ? 2 : 3 }}
            sidebar={
              <AgentsSidebar
                agents={agents}
                selected={frame >= jumped ? 'tests-e2e' : refactor}
                alerts={waiting && frame < jumped ? { 'tests-e2e': C.wait } : undefined}
              />
            }
            overlay={
              <Spotlight
                x={1398}
                y={812}
                w={56}
                h={28}
                on={ramp(frame, c.word('ask', 'couper') - 6, 8) - ramp(frame, c.end('ask') + 12, 8)}
              />
            }
          >
            {frame >= jumped ? (
              <>
                <ConvHeader
                  name="tests-e2e"
                  status={waiting ? 'waiting' : 'running'}
                  sub="demo-api / main"
                  metrics={{ model: 'Sonnet 5.5', tokens: tok(96), cost: usd(0.73), files: 3, duration: '8m 02s' }}
                />
                <Conversation>
                  <AssistantMsg text={tr('Les 18 scénarios passent sur Chrome.', 'All 18 scenarios pass on Chrome.')} />
                  <QuestionCard
                    question={tr('Je lance aussi les tests sur Firefox ?', 'Shall I run the tests on Firefox too?')}
                    options={[tr('Oui', 'Yes'), tr('Non, Chrome suffit', 'No, Chrome is enough')]}
                    picked={frame >= answered ? 0 : null}
                    cursor={
                      frame >= answered - 26 && frame < answered + 14
                        ? { target: 0, t: ramp(frame, answered - 26, 20), click: ramp(frame, answered, 14) }
                        : undefined
                    }
                  />
                  {frame >= answered ? (
                    <AssistantMsg
                      text={typed(
                        tr('Je lance les 18 scénarios sur Firefox.', 'Running the 18 scenarios on Firefox.'),
                        frame,
                        fps,
                        answered + 8,
                      )}
                    />
                  ) : null}
                  {frame >= answered ? <Working /> : null}
                </Conversation>
                <Composer
                  placeholder={
                    frame >= answered
                      ? tr('Envoyer un message à tests-e2e…', 'Send a message to tests-e2e…')
                      : tr('Réponds à la question ou écris une réponse libre…', 'Answer the question or write a free reply…')
                  }
                  model="Sonnet 5.5"
                  waiting={waiting}
                  busy={!waiting}
                />
              </>
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
      </div>
      <OtherWindow enter={behind} />
      <Taskbar enter={taskbar} flash={frame >= c.word('toast', 'barre') - 4 && frame < back} badge={waiting ? 1 : undefined} active />
      {closed > 0.6 ? (
        <span
          style={{
            position: 'absolute',
            left: 1756,
            top: 1033,
            width: 40,
            height: 40,
            borderRadius: 10,
            border: `2px solid ${C.spark}`,
            boxShadow: `0 0 24px ${C.spark}`,
            opacity: (closed - 0.6) / 0.4,
            zIndex: 70,
          }}
        />
      ) : null}
      <PointerPath
        keys={[
          [closeAt - 22, 1500, 420],
          [closeAt - 2, 1683, 215, true],
        ]}
      />
      <WinToast
        title="demo-api · tests-e2e"
        body={tr('Claude attend ta réponse', 'Claude is waiting for your answer')}
        enter={pop(frame, fps, toast, 16) - ramp(frame, back, 14)}
        bottom={72}
      />
      <KeyPress keys={['Ctrl', 'J']} at={jumped - 8} />
    </Stage>
  );
};

import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { AGENTS, STATUS, TABS } from '../data';
import { C } from '../theme';
import { AssistantMsg, Conversation, ConvHeader, UserMsg, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { ContextMenu } from '../ui/Modal';
import { AppToast } from '../ui/Toast';
import { Phone } from '../ui/Phone';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Stage, Title } from '../ui/Stage';

const ASK = "J'ai fini la refacto de l'auth. Je lance les tests ?";
const REPLY = 'Oui, et ajoute les tests Firefox';

/** Remote control from the agent's menu; the same session on a phone; a message sent from it shows up in the app. */
export const Remote: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const menuAt = c.at('enable') + 30;
  const enabled = c.word('enable', 'active') + 6;
  const slideAt = c.word('enable', 'session');
  const slide = pop(frame, fps, slideAt, 18);
  const sent = c.word('reply', 'envoies');
  const arrived = c.word('reply', 'affiche');
  const remote = frame >= enabled;
  return (
    <Stage>
      <Title />
      <AppWindow x={-190 * Math.min(1, slide)} scale={1 - 0.1 * Math.min(1, slide)}>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.slice(0, 4).map((a) =>
                a.name === 'refacto-auth'
                  ? {
                      ...a,
                      status: frame >= arrived ? ('running' as const) : ('done' as const),
                      remote: remote ? frame >= enabled + 20 : undefined,
                    }
                  : a,
              )}
              selected="refacto-auth"
            />
          }
          overlay={
            <>
              <ContextMenu
                x={170}
                y={200}
                enter={frame >= menuAt && frame < enabled ? 1 : 0}
                hover={frame >= enabled - 14 ? 4 : undefined}
                items={[
                  'Renommer',
                  { label: 'Archiver', hint: 'garde la conversation' },
                  'Ouvrir dans l’éditeur',
                  null,
                  { label: 'Activer le remote control', hint: 'claude.ai, mobile' },
                  null,
                  { label: 'Supprimer…', danger: true },
                ]}
              />
              <AppToast
                text="refacto-auth est accessible depuis claude.ai et l’app Claude"
                tone="ok"
                enter={pop(frame, fps, enabled + 4, 14) - ramp(frame, enabled + 4 * fps, 12)}
              />
              <PointerPath
                keys={[
                  [menuAt - 18, 400, 300],
                  [menuAt, 150, 190, true],
                  [enabled - 14, 260, 360],
                  [enabled - 4, 262, 362, true],
                ]}
              />
            </>
          }
        >
          <ConvHeader
            name="refacto-auth"
            status={frame >= arrived ? 'running' : 'done'}
            sub="demo-api / main"
            metrics={{ model: 'Opus 5.5', tokens: '214 k', cost: '2,86 $', files: 6, duration: '15m 02s' }}
          />
          <Conversation>
            <AssistantMsg text="La refacto est terminée : JWT en place, période de transition pour les sessions existantes." />
            <AssistantMsg text={ASK} avatar={false} />
            <UserMsg text={REPLY} tag="depuis claude.ai" enter={pop(frame, fps, arrived)} />
            {frame >= arrived ? <Working /> : null}
          </Conversation>
          <Composer placeholder="Envoyer un message à refacto-auth…" busy={frame >= arrived} />
        </Shell>
      </AppWindow>
      {frame >= sent && frame < arrived ? (
        <div
          style={{
            position: 'absolute',
            left: 1480 - 280 * ramp(frame, sent, arrived - sent),
            top: 560,
            width: 16,
            height: 16,
            borderRadius: 8,
            background: C.spark,
            boxShadow: `0 0 24px ${C.spark}`,
          }}
        />
      ) : null}
      <Phone
        x={1920 - 480 * Math.min(1, pop(frame, fps, slideAt + 10, 18))}
        messages={[{ from: 'claude', text: ASK }, ...(frame >= sent ? [{ from: 'me' as const, text: REPLY }] : [])]}
        draft={frame >= sent ? '' : typed(REPLY, frame, fps, c.word('enable', 'téléphone'), 22)}
      />
    </Stage>
  );
};

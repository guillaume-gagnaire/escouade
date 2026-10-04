import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { count, pop, ramp } from '../anim';
import { useCues } from '../cues';
import { AGENTS, STATUS, TABS } from '../data';
import { C, MONO, soft } from '../theme';
import { AssistantMsg, Conversation, ConvHeader, enterStyle, TurnSep, Working } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { Shell } from '../ui/Shell';
import { AgentsSidebar, Button } from '../ui/Sidebar';
import { AppWindow, camPath, Spotlight, Stage, Title } from '../ui/Stage';
import { StatsView } from '../ui/Stats';
import { RefactoConv } from './common';

/** The quotas and today's cost in the status bar, the resume after the usage limit, then the stats page. */
export const StatsScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const q = (w: string) => c.word('quota', w);
  const limited = frame >= c.at('resume') - 4 && frame < c.word('resume', 'revient') + 6;
  const back = frame >= c.word('resume', 'revient') + 6;
  const statsClick = c.word('page', 'statistiques');
  const statsOn = frame >= statsClick + 4;
  const p = (w: string) => c.word('page', w);
  const cam = camPath(frame, [
    [4, 26, { x: 500, y: 572, zoom: 1.55 }],
    [q('session') - 12, 22, { x: 765, y: 580, zoom: 1.6 }],
    [c.end('cost') + 4, 22, { x: 750, y: 422, zoom: 1 }],
  ]);
  const spot = (from: number, to: number) => ramp(frame, from - 4, 8) - ramp(frame, to - 4, 8);
  const session = limited ? 100 : count(frame, 20, 200, 64, 71);
  return (
    <Stage>
      <Title out={cam.zoom > 1 ? Math.min(1, (cam.zoom - 1) * 4) : 0} />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          view={statsOn ? 'stats' : 'project'}
          status={{
            ...STATUS,
            session,
            sessionReset: limited ? '0 h 03' : '2 h 14',
            weekly: count(frame, 20, 200, 36, 38),
            cost: count(frame, c.at('cost'), 150, 3.48, 4.12),
            estimated: frame >= c.at('cost') && frame < c.end('cost') + 30,
          }}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.map((a) =>
                a.name === 'refacto-auth' && limited ? { ...a, status: 'error' as const, resume: 'Reprise à 14:05' } : a,
              )}
              selected="refacto-auth"
            />
          }
          overlay={
            <>
              <Spotlight x={8} y={812} w={304} h={30} on={spot(q('actifs'), q('quotas'))} />
              <Spotlight x={540} y={812} w={292} h={30} on={spot(q('session'), q('semaine'))} />
              <Spotlight x={540} y={812} w={512} h={30} on={spot(q('semaine'), c.at('cost'))} />
              <Spotlight x={1070} y={812} w={142} h={30} on={spot(c.word('cost', 'coût'), c.word('cost', 'mémoire'))} />
              <Spotlight x={318} y={812} w={206} h={30} on={spot(c.word('cost', 'mémoire'), c.end('cost') + 4)} />
              <PointerPath
                keys={[
                  [statsClick - 22, 1200, 300],
                  [statsClick, 1290, 28, true],
                ]}
              />
            </>
          }
        >
          {statsOn ? (
            <StatsView
              grow={ramp(frame, statsClick + 4, 40)}
              count={ramp(frame, statsClick + 4, 40)}
              range={frame >= p('mois') - 2 ? 2 : frame >= p('semaine') - 2 ? 1 : 0}
              glow={{
                series: frame >= p('sortie') - 2 ? 3 : frame >= p('cache') - 2 ? 2 : frame >= p('entrée') - 2 ? 1 : undefined,
                range: spot(p('jour'), p('projet') - 4),
                projects: spot(p('projet'), p('modèle')),
                models: spot(p('modèle'), c.length),
              }}
            />
          ) : (
            <>
              <ConvHeader
                name="refacto-auth"
                status={limited ? 'idle' : 'running'}
                sub="demo-api / main"
                metrics={{ model: 'Opus 5.5', tokens: '184 k', cost: '2,41 $', files: 6, duration: '12m 40s' }}
              />
              <Conversation>
                <RefactoConv />
                {limited || back ? (
                  <div
                    style={{
                      marginLeft: 36,
                      display: 'flex',
                      flexDirection: 'column',
                      gap: 10,
                      padding: '14px 18px',
                      borderRadius: 10,
                      border: `1px solid ${soft(C.del, 55)}`,
                      background: C.panel,
                      ...enterStyle(pop(frame, fps, c.at('resume'))),
                    }}
                  >
                    <span style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 14, fontWeight: 700 }}>
                      <span style={{ width: 8, height: 8, borderRadius: 4, background: C.del }} />
                      Le tour s'est terminé en erreur
                    </span>
                    <span style={{ fontFamily: MONO, fontSize: 13, color: C.muted }}>
                      Limite d'usage atteinte · réinitialisation à 14:05
                    </span>
                    {limited ? (
                      <span style={{ display: 'flex', alignItems: 'center', gap: 12, fontSize: 14, color: C.wait }}>
                        Reprise automatique à 14:05
                        <Button small>Annuler la reprise</Button>
                      </span>
                    ) : null}
                  </div>
                ) : null}
                {back ? (
                  <>
                    <TurnSep text="↻ Quota revenu · reprise automatique" enter={pop(frame, fps, c.word('resume', 'revient') + 6)} />
                    <AssistantMsg
                      text="Je reprends : il reste la période de transition des sessions."
                      enter={pop(frame, fps, c.word('resume', 'revient') + 16)}
                    />
                    <Working />
                  </>
                ) : !limited ? (
                  <Working />
                ) : null}
              </Conversation>
              <Composer placeholder="Envoyer un message à refacto-auth…" busy={!limited} />
            </>
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

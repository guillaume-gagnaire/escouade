import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { count, pop, ramp } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
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
  const { tr, tok, usd, pick } = useFmt();
  const { agents: AGENTS, status: STATUS, refactor } = useDemo();
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
            sessionReset: limited ? tr('0 h 03', '0h03') : tr('2 h 14', '2h14'),
            weekly: count(frame, 20, 200, 36, 38),
            cost: count(frame, c.at('cost'), 150, 3.48, 4.12),
            estimated: frame >= c.at('cost') && frame < c.end('cost') + 30,
          }}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.map((a) =>
                a.name === refactor && limited
                  ? { ...a, status: 'error' as const, resume: tr('Reprise à 14:05', 'Resumes at 2:05 PM') }
                  : a,
              )}
              selected={refactor}
            />
          }
          overlay={
            <>
              <Spotlight x={8} y={812} w={pick(304, 256)} h={30} on={spot(q('actifs'), q('quotas'))} />
              <Spotlight x={pick(540, 490)} y={812} w={pick(292, 256)} h={30} on={spot(q('session'), q('semaine'))} />
              <Spotlight x={pick(540, 490)} y={812} w={pick(512, 469)} h={30} on={spot(q('semaine'), c.at('cost'))} />
              <Spotlight
                x={pick(1070, 974)}
                y={812}
                w={pick(142, 96)}
                h={30}
                on={spot(c.word('cost', 'coût'), c.word('cost', 'mémoire'))}
              />
              <Spotlight x={pick(318, 275)} y={812} w={pick(206, 198)} h={30} on={spot(c.word('cost', 'mémoire'), c.end('cost') + 4)} />
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
                name={refactor}
                status={limited ? 'idle' : 'running'}
                sub="demo-api / main"
                metrics={{ model: 'Opus 5.5', tokens: tok(184), cost: usd(2.41), files: 6, duration: '12m 40s' }}
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
                      {tr("Le tour s'est terminé en erreur", 'The turn ended with an error')}
                    </span>
                    <span style={{ fontFamily: MONO, fontSize: 13, color: C.muted }}>
                      {tr("Limite d'usage atteinte · réinitialisation à 14:05", 'Usage limit reached · resets at 2:05 PM')}
                    </span>
                    {limited ? (
                      <span style={{ display: 'flex', alignItems: 'center', gap: 12, fontSize: 14, color: C.wait }}>
                        {tr('Reprise automatique à 14:05', 'Auto-resume at 2:05 PM')}
                        <Button small>{tr('Annuler la reprise', 'Cancel resume')}</Button>
                      </span>
                    ) : null}
                  </div>
                ) : null}
                {back ? (
                  <>
                    <TurnSep
                      text={tr('↻ Quota revenu · reprise automatique', '↻ Quota is back · auto-resume')}
                      enter={pop(frame, fps, c.word('resume', 'revient') + 6)}
                    />
                    <AssistantMsg
                      text={tr(
                        'Je reprends : il reste la période de transition des sessions.',
                        'Picking up: the sessions’ transition period remains.',
                      )}
                      enter={pop(frame, fps, c.word('resume', 'revient') + 16)}
                    />
                    <Working />
                  </>
                ) : !limited ? (
                  <Working />
                ) : null}
              </Conversation>
              <Composer placeholder={tr(`Envoyer un message à ${refactor}…`, `Send a message to ${refactor}…`)} busy={!limited} />
            </>
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

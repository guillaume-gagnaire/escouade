import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { AGENTS, onSonnet, STATUS, TABS } from '../data';
import { C } from '../theme';
import { Composer } from '../ui/Composer';
import { Conversation, ConvHeader } from '../ui/Chat';
import { PointerPath } from '../ui/Cursor';
import { ContextMenu } from '../ui/Modal';
import { COLORS, NewProjectModal } from '../ui/NewProject';
import { Shell } from '../ui/Shell';
import { AgentsSidebar, type AgentInfo } from '../ui/Sidebar';
import { AppWindow, Spotlight, Stage, Title } from '../ui/Stage';
import { EmptyConv, RefactoConv } from './common';

const PATH = 'C:\\dev\\boutique';
const FIRST: AgentInfo = { name: 'agent-1', status: 'idle', model: 'Opus 5.5', time: '0m 00s', tokens: '0', cost: '0,00 $', files: 0 };

/** Project tabs, then a new project from its modal. */
export const Projects: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const open = c.word('add', 'nouveau') + 14;
  const create = c.end('options') + 6;
  const created = frame >= create + 8;
  // « infra » dragged next to « demo-api », then the tab's context menu.
  const dragFrom = c.at('menu') + 2;
  const drop = c.word('menu', 'ranger') + 4;
  const menuAt = c.word('menu', 'clic');
  const w = (word: string) => c.word('menu', word);
  const order = frame >= drop ? [0, 3, 1, 2] : [0, 1, 2, 3];
  const tabs = [
    ...order
      .map((k) => TABS[k])
      .map((t, i) => ({ ...t, enter: pop(frame, fps, 4 + i * 10), delta: Math.round((t.delta ?? 0) * ramp(frame, 40, 50)) })),
    { name: 'boutique', hue: 325, enter: created ? pop(frame, fps, create + 8) : 0 },
  ];
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={tabs}
          active={created ? 4 : 0}
          status={created ? { ...STATUS, sync: { branch: 'main', behind: 0, ahead: 0 } } : STATUS}
          sidebar={
            created ? (
              <AgentsSidebar agents={[FIRST]} selected="agent-1" foot={{ path: PATH, counts: [0, 0, 0] }} />
            ) : (
              <AgentsSidebar agents={onSonnet(AGENTS)} selected="refacto-auth" enters={AGENTS.map((_, i) => pop(frame, fps, 30 + i * 8))} />
            )
          }
          overlay={
            <>
              <Spotlight
                x={50}
                y={8}
                w={190}
                h={42}
                on={ramp(frame, c.word('tabs', 'point') - 6, 10) - ramp(frame, c.at('menu') - 10, 10)}
              />
              {frame >= dragFrom + 8 && frame < drop ? (
                <span
                  style={{
                    position: 'absolute',
                    left: 236,
                    top: 10,
                    width: 3,
                    height: 38,
                    borderRadius: 2,
                    background: C.spark,
                    zIndex: 50,
                  }}
                />
              ) : null}
              <ContextMenu
                x={70}
                y={48}
                enter={frame >= menuAt + 2 && frame < c.end('menu') + 8 ? pop(frame, fps, menuAt + 2, 20) : 0}
                hover={frame >= w('ferme') - 4 ? 7 : frame >= w('recolore') - 4 ? 1 : frame >= w('renomme') - 4 ? 0 : undefined}
                items={[
                  'Renommer…',
                  { label: 'Couleur', colors: COLORS, selected: 0 },
                  null,
                  { label: 'Désactiver le worktree par agent', hint: 'nouveaux agents' },
                  'Commandes de lancement…',
                  'Ouvrir le dossier',
                  null,
                  { label: 'Fermer le projet…', danger: true },
                ]}
              />
              <NewProjectModal
                enter={pop(frame, fps, open + 6, 16) - (frame >= create ? 1 : 0)}
                path={typed(PATH, frame, fps, c.word('add', 'choisis'), 26)}
                git={ramp(frame, c.word('add', 'détecte'), 10)}
                name={typed('boutique', frame, fps, c.at('options'), 20)}
                color={frame >= c.word('options', 'couleur') ? 12 : 0}
                firstAgent={frame >= c.word('options', 'premier')}
                worktrees={frame >= c.word('options', 'worktree')}
                pressed={ramp(frame, create - 4, 3) - ramp(frame, create, 3)}
              />
              <PointerPath
                keys={[
                  [dragFrom - 16, 640, 150],
                  [dragFrom, 650, 28],
                  [drop, 238, 28, true],
                  [menuAt - 12, 200, 60],
                  [menuAt, 140, 28, true],
                  [w('renomme') - 4, 160, 72],
                  [w('recolore') - 4, 160, 112],
                  [w('ferme') - 4, 170, 322],
                  [open - 26, 640, 300],
                  [open, 742, 28, true],
                  [c.word('options', 'couleur') - 14, 830, 380],
                  [c.word('options', 'couleur'), 925, 433, true],
                  [c.word('options', 'premier'), 1050, 560, true],
                  [c.word('options', 'worktree'), 1050, 618, true],
                  [create - 4, 990, 678, true],
                ]}
              />
            </>
          }
        >
          {created ? (
            <>
              <ConvHeader name="agent-1" status="idle" sub="boutique / main" />
              <EmptyConv cwd={PATH} />
              <Composer placeholder="Décris la tâche à confier à Claude…" />
            </>
          ) : (
            <>
              <ConvHeader name="refacto-auth" status="running" sub="demo-api / main" />
              <Conversation>
                <RefactoConv />
              </Conversation>
              <Composer placeholder="Envoyer un message à refacto-auth…" busy />
            </>
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

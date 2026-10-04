import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { AGENTS, onSonnet, STATUS, TABS } from '../data';
import { C, soft } from '../theme';
import { Composer } from '../ui/Composer';
import { Conversation, ConvHeader, UserMsg, Working } from '../ui/Chat';
import { PointerPath } from '../ui/Cursor';
import { ConfirmModal, ContextMenu } from '../ui/Modal';
import { Shell } from '../ui/Shell';
import { AgentsSidebar, type AgentInfo } from '../ui/Sidebar';
import { AppWindow, camPath, Spotlight, Stage, Title } from '../ui/Stage';
import { EmptyConv, RefactoConv } from './common';

const ASK = 'Ajoute un export CSV des factures, avec les filtres de la liste.';

/** Agents fill the sidebar; a card up close; a new agent named by Haiku; one archived. */
export const Agents: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const newAt = c.at('name') - 4;
  const sent = c.word('name', 'demande') + 6;
  const named = c.word('name', 'nom');
  const menuAt = c.at('archive') + 6;
  const archived = c.word('archive', 'conversation');
  const isNew = frame >= newAt + 6;
  const fresh: AgentInfo = {
    name: frame >= named ? 'export-csv-factures' : 'agent-6',
    status: frame >= sent ? 'running' : 'idle',
    model: 'Opus 5.5',
    time: frame >= sent ? '0m 04s' : '0m 00s',
    tokens: frame >= sent ? '3 k' : '0',
    cost: frame >= sent ? '≈ 0,04 $' : '0,00 $',
    files: 0,
  };
  // fix-login renamed by a double-click, then deleted with its worktree.
  const dbl = c.at('manage') + 2;
  const renamed = c.word('manage', 'supprimer') - 12;
  const delMenu = c.word('manage', 'supprimer') - 4;
  const confirmAt = c.word('manage', 'arrête');
  const deleted = c.end('manage') + 6;
  const renaming = frame >= dbl && frame < renamed;
  const login = frame >= renamed ? 'fix-login-oauth' : 'fix-login';
  const agents = [
    ...onSonnet(AGENTS)
      .filter((a) => (a.name !== 'docs-api' || frame < archived) && (a.name !== 'fix-login' || frame < deleted))
      .map((a) => (a.name === 'fix-login' ? { ...a, name: login, branch: 'fix-login' } : a)),
    ...(isNew ? [fresh] : []),
  ];
  const flash = ramp(frame, named, 6) - ramp(frame, named + 24, 12);
  const cam = camPath(frame, [
    [c.word('card', 'carte') - 10, 24, { x: 330, y: 250, zoom: 2 }],
    [c.end('card') + 4, 22, { x: 750, y: 422, zoom: 1 }],
  ]);
  return (
    <Stage>
      <Title out={cam.zoom > 1 ? Math.min(1, (cam.zoom - 1) * 3) : 0} />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          status={{ ...STATUS, active: frame >= sent ? 4 : 3 }}
          sidebar={
            <AgentsSidebar
              agents={agents}
              selected={isNew ? fresh.name : 'refacto-auth'}
              enters={agents.map((a, i) => (a === fresh ? pop(frame, fps, newAt + 6) : pop(frame, fps, c.at('many') + i * 10)))}
              archived={frame >= archived ? 1 : undefined}
              pressedNew={ramp(frame, newAt - 3, 3) - ramp(frame, newAt, 3)}
              names={{
                ...(renaming
                  ? {
                      [login]: (
                        <span
                          style={{
                            display: 'inline-block',
                            minWidth: 170,
                            padding: '0 6px',
                            border: `1px solid ${C.spark}`,
                            borderRadius: 4,
                            background: C.bg,
                          }}
                        >
                          {frame < dbl + 10 ? (
                            <span style={{ background: soft(C.spark, 45) }}>fix-login</span>
                          ) : (
                            typed('fix-login-oauth', frame, fps, dbl + 10, 14)
                          )}
                          <span
                            style={{ display: 'inline-block', width: 2, height: 15, background: C.text, marginLeft: 1, verticalAlign: -2 }}
                          />
                        </span>
                      ),
                    }
                  : {}),
                [fresh.name]: (
                  <span style={{ background: soft(C.spark, 35 * flash), borderRadius: 4, padding: '0 3px', margin: '0 -3px' }}>
                    {fresh.name}
                  </span>
                ),
              }}
            />
          }
          overlay={
            <>
              <Spotlight x={12} y={158} w={278} h={90} on={ramp(frame, c.word('card', 'carte'), 10) - ramp(frame, c.end('card'), 10)} />
              <ContextMenu
                x={170}
                y={500}
                enter={frame >= menuAt && frame < archived ? pop(frame, fps, menuAt, 20) : 0}
                hover={frame >= c.word('archive', 'archive-le') - 8 ? 1 : undefined}
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
              <ContextMenu
                x={170}
                y={470}
                enter={frame >= delMenu && frame < confirmAt ? pop(frame, fps, delMenu, 20) : 0}
                hover={frame >= delMenu + 8 ? 6 : undefined}
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
              <ConfirmModal
                title="Supprimer l'agent « fix-login-oauth » ?"
                body="Le processus Claude est arrêté et la conversation est retirée de l’application (la session Claude Code reste sur le disque)."
                confirm="Supprimer"
                danger
                option={{ label: 'Supprimer aussi le worktree et la branche escouade/fix-login', checked: true }}
                enter={frame >= confirmAt && frame < deleted ? pop(frame, fps, confirmAt, 18) : 0}
                pressed={ramp(frame, deleted - 6, 3) - ramp(frame, deleted, 3)}
              />
              <PointerPath
                keys={[
                  [newAt - 24, 330, 300],
                  [newAt - 3, 225, 130, true],
                  [menuAt - 16, 200, 400],
                  [menuAt, 150, 492, true],
                  [c.word('archive', 'archive-le') - 8, 230, 553],
                  [archived - 2, 232, 553, true],
                  [dbl - 14, 220, 520],
                  [dbl, 200, 460, true],
                  [delMenu - 8, 190, 470],
                  [delMenu, 180, 462, true],
                  [confirmAt - 6, 250, 663],
                  [confirmAt - 1, 252, 663, true],
                  [deleted - 20, 900, 540],
                  [deleted - 6, 958, 517, true],
                ]}
              />
            </>
          }
        >
          {isNew ? (
            <>
              <ConvHeader name={fresh.name} status={frame >= sent ? 'running' : 'idle'} sub="demo-api / main" />
              {frame >= sent ? (
                <Conversation>
                  <UserMsg text={ASK} enter={pop(frame, fps, sent)} />
                  <Working />
                </Conversation>
              ) : (
                <EmptyConv cwd="C:\dev\demo-api" />
              )}
              <Composer
                text={frame >= sent ? '' : typed(ASK, frame, fps, newAt + 10, 60)}
                placeholder="Décris la tâche à confier à Claude…"
                busy={frame >= sent}
              />
            </>
          ) : (
            <>
              <ConvHeader name="refacto-auth" status="running" sub="demo-api / main" />
              <Conversation>
                <RefactoConv />
                <Working />
              </Conversation>
              <Composer placeholder="Envoyer un message à refacto-auth…" busy />
            </>
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

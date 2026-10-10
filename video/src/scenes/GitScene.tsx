import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import { AssistantMsg, Conversation, ConvHeader, Diffstat, PatchView, ToolCall, Working, type PatchLine } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { commitsOf, DiffPane, FullDiff, GitGraph, SidePanel, SyncMenu, type ChangedFile, type DiffRow } from '../ui/Git';
import { ConfirmModal } from '../ui/Modal';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, camPath, Spotlight, Stage, Title } from '../ui/Stage';
import { AppToast } from '../ui/Toast';
import { PATCH } from './Chat';

const ROWS: DiffRow[] = [
  { kind: 'ctx', l: "router.get('/users', async (req) => {", r: "router.get('/users', async (req) => {" },
  { kind: 'chg', l: '  const users = await db.users.all();', r: '  const { limit = 50, cursor } = req.query;' },
  { kind: 'add', r: '  const page = await paginate(db.users, {' },
  { kind: 'add', r: '    limit: Math.min(limit, 100),' },
  { kind: 'add', r: '    cursor,' },
  { kind: 'add', r: '  });' },
  { kind: 'chg', l: '  return users;', r: '  return { users: page.items, next: page.next };' },
  { kind: 'ctx', l: '});', r: '});' },
];
const TEST_PATCH: PatchLine[] = [
  { kind: ' ', n: 40, text: "describe('GET /users', () => {" },
  { kind: '+', n: 41, text: "  it('pages by 50, with a cursor to the next page', async () => {" },
  { kind: '+', n: 42, text: "    const first = await get('/users');" },
  { kind: '+', n: 43, text: '    expect(first.users).toHaveLength(50);' },
  { kind: '+', n: 44, text: '    const next = await get(`/users?cursor=${first.next}`);' },
  { kind: '+', n: 45, text: '    expect(next.users[0].id).not.toBe(first.users[0].id);' },
  { kind: '+', n: 46, text: '  });' },
  { kind: '-', n: 47, text: "  it.todo('pagination');" },
  { kind: ' ', n: 47, text: '});' },
];
const MINE: ChangedFile[] = [
  { status: 'M', path: 'src/routes/users.ts', add: 24, del: 3 },
  { status: 'A', path: 'src/db/paginate.ts', add: 41, del: 0 },
  { status: 'M', path: 'tests/users.test.ts', add: 37, del: 2 },
];
/** The other agents' files, in « Tout le projet ». */
const othersOf = (refactor: string): ChangedFile[] => [
  { status: 'M', path: 'src/auth/middleware.ts', add: 58, del: 21, agent: refactor },
  { status: 'A', path: 'src/auth/jwt.ts', add: 58, del: 0, agent: refactor },
  { status: 'M', path: 'e2e/login.spec.ts', add: 12, del: 4, agent: 'tests-e2e' },
];

/** A squash merge's toast: what git commit printed (agent-actions.ts, git.rs merge). */
const MERGED = [
  '[main 4e1b2c9] pagination-users',
  ' 3 files changed, 107 insertions(+), 5 deletions(-)',
  ' create mode 100644 src/db/paginate.ts',
].join('\n');

/** Split layout with the files and their diff, the scope, the history, the merge, then the sync with the remote. */
export const GitScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd, pick } = useFmt();
  const { agents: AGENTS, status: STATUS, refactor } = useDemo();
  const OTHERS = othersOf(refactor);
  const split = c.word('split', 'moitié-moitié') - 4;
  const writing = c.word('split', 'pendant');
  const project = frame >= c.word('scope', 'projet') && frame < c.at('history');
  const unified = frame >= c.word('scope', 'unifié') - 2;
  const history = frame >= c.word('history', 'historique') - 4 && frame < c.at('merge') - 6;
  const mergeClick = c.word('merge', 'clic') + 2;
  const squash = c.word('merge', 'squash');
  const confirmClick = c.end('merge') + 2;
  const merged = frame >= confirmClick + 8;
  // « en plein écran », then a commit of the history clicked: the full-screen diff view.
  const full = c.word('scope', 'plein') - 2;
  const commitClick = c.word('history', 'clic');
  const showMerged = merged && frame < c.at('sync');
  const syncClick = c.word('sync', 'ta') + 4;
  const pullClick = c.word('sync', 'pull');
  const pulled = frame >= pullClick + 24;
  const panel = pop(frame, fps, split, 18);
  const files = MINE.map((f, i) => (i === 0 ? { ...f, add: frame >= writing + 20 ? 29 : 24 } : f));
  const cam = camPath(frame, [[c.at('sync'), 26, { x: 1071, y: 603, zoom: 1.75 }]]);
  return (
    <Stage>
      <Title out={cam.zoom > 1 ? Math.min(1, (cam.zoom - 1) * 4) : 0} />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          status={{
            ...STATUS,
            // Level with origin/main before the merge (the graph shows it), the squashed commit to push after.
            sync: {
              branch: 'main',
              behind: pulled ? 0 : 3,
              ahead: merged ? 1 : 0,
              busy: frame >= pullClick && !pulled ? 'Pull' : undefined,
            },
          }}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.map((a) => (a.name === 'pagination-users' && merged ? { ...a, status: 'done' as const } : a))}
              selected="pagination-users"
            />
          }
          overlay={
            <>
              <Spotlight
                x={912}
                y={358}
                w={578}
                h={170}
                on={ramp(frame, c.word('scope', 'côte') - 4, 8) - ramp(frame, c.end('scope'), 10)}
              />
              <Spotlight x={912} y={768} w={576} h={40} on={ramp(frame, c.word('merge', 'worktree'), 8) - ramp(frame, mergeClick + 6, 8)} />
              <SyncMenu
                x={pick(1218, 1074)}
                y={808}
                behind={3}
                ahead={1}
                hover={frame >= pullClick - 10 ? 0 : undefined}
                enter={frame >= syncClick && frame < pullClick + 4 ? 1 : 0}
              />
              <FullDiff
                title={tr('Modifications de demo-api', 'Changes in demo-api')}
                files={[...files, ...OTHERS]}
                enter={frame >= full && frame < c.at('history') - 4 ? pop(frame, fps, full, 18) : 0}
              >
                <PatchView lines={PATCH} size={14} />
              </FullDiff>
              <FullDiff
                title={tr('7be01d4 · Tests de la pagination', '7be01d4 · Pagination tests')}
                files={[{ status: 'M', path: 'tests/users.test.ts', add: 37, del: 2 }]}
                enter={frame >= commitClick + 6 && frame < c.at('merge') - 6 ? pop(frame, fps, commitClick + 6, 18) : 0}
              >
                <PatchView lines={TEST_PATCH} size={14} />
              </FullDiff>
              <ConfirmModal
                title={tr('Merger escouade/pagination-users dans main ?', 'Merge escouade/pagination-users into main?')}
                body={tr(
                  "Les commits de l'agent « pagination-users » sont intégrés dans la branche courante du projet.",
                  'The commits of the agent “pagination-users” are merged into the project’s current branch.',
                )}
                confirm={tr('Merger', 'Merge')}
                option={{ label: tr('Squash (un seul commit)', 'Squash (a single commit)'), checked: frame >= squash }}
                enter={frame >= mergeClick + 4 && frame < confirmClick + 4 ? pop(frame, fps, mergeClick + 4, 18) : 0}
                pressed={ramp(frame, confirmClick - 3, 3) - ramp(frame, confirmClick + 3, 3)}
              />
              <AppToast text={MERGED} tone="ok" enter={pop(frame, fps, confirmClick + 10) - ramp(frame, c.at('sync') - 10, 10)} />
              <AppToast
                text={tr('3 commits tirés', '3 commits pulled')}
                tone="ok"
                enter={pop(frame, fps, pulled ? pullClick + 24 : 99999)}
              />
              <PointerPath
                keys={[
                  [full - 18, 1050, 640],
                  [full - 2, 956, 742, true],
                  [commitClick - 16, 1100, 260],
                  [commitClick, 1150, 172, true],
                  [mergeClick - 24, 900, 600],
                  [mergeClick, 1200, 788, true],
                  [squash - 12, 600, 470],
                  [squash, 492, 450, true],
                  [confirmClick - 2, 980, 508, true],
                  [syncClick - 26, pick(1150, 1006), 700],
                  [syncClick, pick(1262, 1118), 826, true],
                  [pullClick - 10, pick(1280, 1136), 700],
                  [pullClick, pick(1280, 1136), 700, true],
                ]}
              />
            </>
          }
        >
          <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
            <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
              <ConvHeader
                name="pagination-users"
                status={merged ? 'done' : 'running'}
                sub="demo-api / escouade/pagination-users"
                test="prepare"
                split={frame >= split}
                compact={frame >= split}
                metrics={{ model: 'Opus 5.5', tokens: tok(24.8, 1), cost: usd(0.27), files: files.length, duration: '4m 02s' }}
              />
              <Conversation>
                <AssistantMsg text={tr('La pagination est en place, avec ses tests.', 'Pagination is in place, with its tests.')} />
                <ToolCall tool="Edit" target="src/db/paginate.ts" meta={<Diffstat add={41} del={0} />} />
                <ToolCall tool="Bash" target="npm test" meta={<span style={{ fontSize: 13, color: C.muted }}>43 tests</span>} />
                {frame >= writing ? (
                  <ToolCall
                    tool="Edit"
                    target="src/routes/users.ts"
                    running={frame < writing + 20}
                    meta={<Diffstat add={5} del={0} />}
                    enter={pop(frame, fps, writing)}
                  />
                ) : null}
                {merged ? (
                  <AssistantMsg
                    text={tr('Branche fusionnée dans `main`.', 'Branch merged into `main`.')}
                    enter={pop(frame, fps, confirmClick + 10)}
                  />
                ) : !merged && frame < writing + 20 ? (
                  <Working />
                ) : null}
              </Conversation>
              <Composer
                placeholder={tr('Envoyer un message à pagination-users…', 'Send a message to pagination-users…')}
                model="Opus 5.5"
                busy={!merged && frame < writing + 20}
              />
            </div>
            {frame >= split ? (
              <div
                style={{
                  width: 600 * Math.min(1, panel),
                  flex: 'none',
                  borderLeft: `1px solid ${C.line}`,
                  background: C.panel,
                  overflow: 'hidden',
                }}
              >
                <div style={{ width: 600, height: '100%' }}>
                  <SidePanel
                    tab={history || showMerged ? 'history' : 'files'}
                    scope={project ? 'project' : 'agent'}
                    files={project ? [...files, ...OTHERS] : files}
                    hint={
                      project
                        ? tr('Toutes les modifications non commitées du projet', 'All uncommitted changes in the project')
                        : tr(
                            'worktree .claude/worktrees/pagination-users · isolé des autres agents',
                            'worktree .claude/worktrees/pagination-users · isolated from other agents',
                          )
                    }
                    merge={tr('Merger escouade/pagination-users → main…', 'Merge escouade/pagination-users → main…')}
                    pressedMerge={ramp(frame, mergeClick - 3, 3) - ramp(frame, mergeClick + 3, 4)}
                  >
                    {history || showMerged ? (
                      <GitGraph
                        commits={commitsOf(tr)}
                        from={showMerged ? 0 : 1}
                        shown={showMerged ? 8 : Math.floor(ramp(frame, c.word('history', 'historique'), 30) * 7)}
                        highlight={1}
                      />
                    ) : unified ? (
                      <div style={{ borderRadius: 8, border: `1px solid ${C.line}`, overflow: 'hidden' }}>
                        <PatchView lines={PATCH} size={12.5} />
                      </div>
                    ) : (
                      <DiffPane rows={ROWS} shown={Math.floor(ramp(frame, split + 10, 40) * 6) + (frame >= writing ? 2 : 0)} start={12} />
                    )}
                  </SidePanel>
                </div>
              </div>
            ) : null}
          </div>
        </Shell>
      </AppWindow>
    </Stage>
  );
};

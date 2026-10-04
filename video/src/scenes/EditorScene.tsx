import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { AGENTS, STATUS, TABS } from '../data';
import { AssistantMsg, Conversation, ConvHeader, Diffstat, ToolCall } from '../ui/Chat';
import { Composer } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { DiskBanner, EditorView, type EditorLine, type TreeNode } from '../ui/Editor';
import { KeyPress } from '../ui/Keys';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Spotlight, Stage, Title } from '../ui/Stage';

const TREE: TreeNode[] = [
  { name: 'src', depth: 0, dir: true, open: true, changed: true },
  { name: 'auth', depth: 1, dir: true },
  { name: 'db', depth: 1, dir: true, open: true, changed: true },
  { name: 'paginate.ts', depth: 2, status: 'A' },
  { name: 'routes', depth: 1, dir: true, open: true, changed: true },
  { name: 'invoices.ts', depth: 2 },
  { name: 'users.ts', depth: 2, status: 'M' },
  { name: 'app.ts', depth: 1 },
  { name: 'tests', depth: 0, dir: true, open: true, changed: true },
  { name: 'users.test.ts', depth: 1, status: 'M' },
  { name: 'package.json', depth: 0 },
  { name: 'README.md', depth: 0 },
];

const CODE: EditorLine[] = [
  { text: "import { Router } from 'express';" },
  { text: "import { db } from '../db';" },
  { text: "import { paginate } from '../db/paginate';", mark: 'added' },
  { text: '' },
  { text: 'export const router = Router();' },
  { text: '' },
  { text: '// La liste des utilisateurs, page par page.', mark: 'added' },
  { text: "router.get('/users', async (req) => {" },
  { text: '  const { limit = 50, cursor } = req.query;', mark: 'changed' },
  { text: '  const page = await paginate(db.users, { limit, cursor });', mark: 'added' },
  { text: '  return { users: page.items, next: page.next };', mark: 'changed' },
  { text: '});' },
  { text: '' },
  { text: "router.get('/users/:id', async (req) => db.users.find(req.params.id));" },
];
const EDITED = '  const { limit = 20, cursor } = req.query;';

/** The editor, opened from the agent's header: its tree, tabs, colours, search and marks; Ctrl+S; a change on disk. */
export const EditorScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const open = c.word('open', 'éditeur') + 4;
  const shown = frame >= open + 6;
  const search = frame >= c.word('look', 'recherche') - 4 && frame < c.word('look', 'lignes') - 2;
  const typedAt = c.at('save') - 30;
  const saved = c.word('save', 'contrôle') + 6;
  const disk = c.word('save', 'modifie') + 4;
  const again = c.word('save', 'si');
  const dirty = (frame >= typedAt && frame < saved) || frame >= again;
  const lines = CODE.map((l, i) =>
    i === 8 && frame >= typedAt ? { ...l, text: EDITED + (frame >= again ? typed(' // 100 au plus', frame, fps, again, 18) : '') } : l,
  );
  const spot = (word: string, until: string) => ramp(frame, c.word('look', word) - 4, 8) - ramp(frame, c.word('look', until) - 4, 8);
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          view={shown ? 'editor' : 'project'}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.map((a) => (a.name === 'pagination-users' ? { ...a, status: 'done' as const } : a))}
              selected="pagination-users"
            />
          }
          overlay={
            <>
              <Spotlight x={302} y={146} w={266} h={330} on={spot('arborescence', 'onglets')} />
              <Spotlight x={572} y={105} w={400} h={40} on={spot('onglets', 'coloration')} />
              <Spotlight
                x={566}
                y={186}
                w={50}
                h={300}
                on={ramp(frame, c.word('look', 'lignes') - 4, 8) - ramp(frame, c.end('look') + 8, 8)}
              />
              <PointerPath
                keys={[
                  [open - 26, 760, 300],
                  [open - 2, 667, 81, true],
                ]}
              />
            </>
          }
        >
          {shown ? (
            <EditorView
              source="worktree · pagination-users"
              tree={TREE}
              selected={6}
              tabs={[{ name: 'users.ts', dirty }, { name: 'paginate.ts' }, { name: 'users.test.ts' }]}
              activeTab={0}
              path="src/routes/users.ts"
              changed={5}
              lines={lines}
              first={1}
              cursorLine={9}
              dirty={dirty}
              search={search ? { query: 'paginate', hits: [3, 10] } : undefined}
              banner={<DiskBanner enter={pop(frame, fps, disk, 16)} />}
              pressedSave={ramp(frame, saved - 3, 3) - ramp(frame, saved + 3, 4)}
            />
          ) : (
            <>
              <ConvHeader
                name="pagination-users"
                status="done"
                sub="demo-api / escouade/pagination-users"
                test="prepare"
                metrics={{ model: 'Opus 5.5', tokens: '24,8 k', cost: '0,27 $', files: 3, duration: '4m 02s' }}
                pressed={{ editor: ramp(frame, open - 4, 3) - ramp(frame, open + 2, 4) }}
              />
              <Conversation>
                <AssistantMsg text="La pagination est en place, avec ses tests." />
                <ToolCall tool="Edit" target="src/routes/users.ts" link meta={<Diffstat add={24} del={3} />} />
                <ToolCall tool="Write" target="src/db/paginate.ts" link meta={<Diffstat add={41} del={0} />} />
              </Conversation>
              <Composer placeholder="Envoyer un message à pagination-users…" model="Opus 5.5" />
            </>
          )}
        </Shell>
      </AppWindow>
      <KeyPress keys={['Ctrl', 'S']} at={saved - 4} />
    </Stage>
  );
};

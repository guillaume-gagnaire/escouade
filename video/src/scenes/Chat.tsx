import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { count, pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { onSonnet, TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import {
  AssistantMsg,
  Conversation,
  ConvHeader,
  Diffstat,
  Output,
  PatchView,
  PermissionCard,
  QuestionCard,
  Thinking,
  ToolCall,
  TurnSep,
  UserMsg,
  Working,
  type PatchLine,
} from '../ui/Chat';
import { CodeBlock } from '../ui/code';
import { Composer } from '../ui/Composer';
import { KeyPress } from '../ui/Keys';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Spotlight, Stage, Title } from '../ui/Stage';

const CODE = [
  'export async function paginate<T>(query: Query<T>, { limit = 50, cursor }: Page) {',
  '  const rows = await query.after(cursor).take(limit + 1);',
  '  return { items: rows.slice(0, limit), next: rows[limit]?.id };',
  '}',
];
export const PATCH: PatchLine[] = [
  { kind: ' ', n: 12, text: "router.get('/users', async (req) => {" },
  { kind: '-', n: 13, text: '  const users = await db.users.all();' },
  { kind: '+', n: 13, text: '  const { limit = 50, cursor } = req.query;' },
  { kind: '+', n: 14, text: '  const page = await paginate(db.users, { limit, cursor });' },
  { kind: '-', n: 14, text: '  return users;' },
  { kind: '+', n: 15, text: '  return { users: page.items, next: page.next };' },
  { kind: ' ', n: 16, text: '});' },
];
const TESTS = [
  '> vitest run',
  '',
  ' ✓ tests/users.test.ts (12 tests)',
  ' ✓ tests/paginate.test.ts (6 tests)',
  '',
  ' Tests  42 passed (42)',
];

/** A turn of the conversation: markdown, tools unfolded, live cost, a question, a permission, a message sent during the turn. */
export const Chat: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd, pick } = useFmt();
  const { agents: AGENTS, status: STATUS } = useDemo();
  const LATER = tr('Et ajoute un test pour la dernière page.', 'And add a test for the last page.');
  const md = c.at('md');
  const unfoldEdit = c.word('diff', 'déplie-les');
  const unfoldBash = c.word('diff', 'sortie');
  const live = c.at('live');
  const askAt = c.at('ask');
  const permAt = c.word('ask', 'autorisation');
  const picked = c.word('ask', 'clic');
  const allowed = picked + 26;
  const typing = c.at('during') + 4;
  const sent = c.word('during', 'tient') - 4;
  const esc = c.word('during', 'échap') + 6;
  const phase = frame < live ? 'tools' : frame < askAt ? 'live' : frame < c.at('during') ? 'ask' : 'during';
  const tokens = count(frame, live, 150, 12.4, 18.4);
  const cost = count(frame, live, 150, 0.14, 0.21);
  const working = phase === 'live' || (phase === 'during' && frame < esc);
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={{ ...STATUS, cost: 3.34 + cost, estimated: working }}
          sidebar={
            <AgentsSidebar
              agents={onSonnet(AGENTS).map((a) =>
                a.name === 'pagination-users'
                  ? {
                      ...a,
                      status: phase === 'ask' && frame < allowed ? 'waiting' : 'running',
                      tokens: tok(tokens, 1),
                      cost: `${working ? '≈ ' : ''}${usd(cost)}`,
                    }
                  : a,
              )}
              selected="pagination-users"
            />
          }
          overlay={
            <Spotlight
              x={pick(1116, 1156)}
              y={52}
              w={pick(162, 140)}
              h={56}
              on={ramp(frame, c.word('live', 'tokens') - 6, 10) - ramp(frame, c.end('live'), 10)}
            />
          }
        >
          <ConvHeader
            name="pagination-users"
            status={phase === 'ask' && frame < allowed ? 'waiting' : 'running'}
            sub="demo-api / escouade/pagination-users"
            test="prepare"
            metrics={{
              model: 'Sonnet 5.5',
              tokens: tok(tokens, 1),
              cost: `${working ? '≈ ' : ''}${usd(cost)}`,
              files: 3,
              duration: '3m 15s',
            }}
          />
          <Conversation>
            {phase === 'tools' ? (
              <>
                <UserMsg
                  text={tr(
                    "Ajoute la pagination à l'endpoint `/users`, avec un curseur.",
                    'Add pagination to the `/users` endpoint, with a cursor.',
                  )}
                />
                <Thinking enter={pop(frame, fps, md)} />
                <AssistantMsg
                  text={tr(
                    'Voici le plan : **une limite par défaut**, un curseur opaque, et des tests pour chaque cas.',
                    'Here’s the plan: **a default limit**, an opaque cursor, and tests for every case.',
                  )}
                >
                  <CodeBlock lines={CODE} shown={Math.floor(ramp(frame, md + 5, 50) * CODE.length)} />
                </AssistantMsg>
                <ToolCall
                  tool="Read"
                  target="src/routes/users.ts"
                  meta={<span style={{ color: C.ok }}>✓</span>}
                  enter={pop(frame, fps, md + 55)}
                />
                <ToolCall
                  tool="Edit"
                  target="src/routes/users.ts"
                  link={frame >= unfoldEdit}
                  meta={<Diffstat add={24} del={3} />}
                  enter={pop(frame, fps, md + 75)}
                  detail={frame >= unfoldEdit && frame < unfoldBash ? <PatchView lines={PATCH} /> : undefined}
                />
                <ToolCall
                  tool="Bash"
                  target="npm test"
                  meta={<span style={{ fontSize: 13, color: C.muted }}>42 tests</span>}
                  enter={pop(frame, fps, md + 95)}
                  detail={frame >= unfoldBash ? <Output lines={TESTS} /> : undefined}
                />
              </>
            ) : null}
            {phase === 'live' ? (
              <>
                <ToolCall tool="Edit" target="src/routes/users.ts" meta={<Diffstat add={24} del={3} />} />
                <ToolCall tool="Bash" target="npm test" meta={<span style={{ fontSize: 13, color: C.muted }}>42 tests</span>} />
                <AssistantMsg
                  text={tr(
                    "Les tests passent. J'ajoute le cas de la dernière page, sans curseur suivant.",
                    'Tests pass. I’m adding the last-page case, with no next cursor.',
                  )}
                />
                <ToolCall
                  tool="Edit"
                  target="tests/users.test.ts"
                  running={frame < live + 70}
                  meta={<Diffstat add={18} del={0} />}
                  enter={pop(frame, fps, live + 10)}
                />
                <ToolCall tool="Bash" target="npm test -- users" running enter={pop(frame, fps, live + 80)} />
                <Working />
              </>
            ) : null}
            {phase === 'ask' ? (
              <>
                <AssistantMsg text={tr('Les 43 tests passent.', 'All 43 tests pass.')} />
                <QuestionCard
                  question={tr('Quelle taille de page par défaut ?', 'What default page size?')}
                  options={['20', '50', '100']}
                  picked={frame >= picked ? 1 : null}
                  enter={pop(frame, fps, askAt)}
                  cursor={
                    frame >= picked - 30 && frame < picked + 14
                      ? { target: 1, t: ramp(frame, picked - 30, 24), click: ramp(frame, picked, 14) }
                      : undefined
                  }
                />
                <PermissionCard
                  tool="Bash"
                  command="npm run db:migrate"
                  picked={frame >= allowed ? 0 : null}
                  enter={pop(frame, fps, permAt)}
                  cursor={
                    frame >= allowed - 24 && frame < allowed + 14
                      ? { target: 0, t: ramp(frame, allowed - 24, 20), click: ramp(frame, allowed, 14) }
                      : undefined
                  }
                />
              </>
            ) : null}
            {phase === 'during' ? (
              <>
                <AssistantMsg
                  text={tr(
                    "Migration appliquée. Je mets à jour la documentation de l'API.",
                    'Migration applied. I’m updating the API documentation.',
                  )}
                />
                <ToolCall tool="Edit" target="docs/api.md" meta={<Diffstat add={12} del={2} />} />
                <UserMsg text={LATER} tag={tr('transmis pendant le tour', 'delivered during the turn')} enter={pop(frame, fps, sent)} />
                {frame < esc ? (
                  <Working />
                ) : (
                  <TurnSep
                    text={tr(`Interrompu · ${tok(18.4, 1)} tokens · ${usd(0.21)}`, `Interrupted · ${tok(18.4, 1)} tokens · ${usd(0.21)}`)}
                    enter={pop(frame, fps, esc)}
                  />
                )}
              </>
            ) : null}
          </Conversation>
          <Composer
            text={phase === 'during' && frame < sent ? typed(LATER, frame, fps, typing, 34) : ''}
            placeholder={tr('Envoyer un message à pagination-users…', 'Send a message to pagination-users…')}
            model="Sonnet 5.5"
            busy={working}
            waiting={phase === 'ask' && frame < allowed}
          />
        </Shell>
      </AppWindow>
      <KeyPress keys={[tr('Échap', 'Esc')]} at={esc} />
    </Stage>
  );
};

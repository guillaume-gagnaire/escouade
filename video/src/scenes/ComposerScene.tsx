import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { onSonnet, TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import { AssistantMsg, Conversation, ConvHeader, Diffstat, ToolCall, TurnCard, UserMsg } from '../ui/Chat';
import { Composer, Suggestions, type Attachment, type Menu } from '../ui/Composer';
import { PointerPath } from '../ui/Cursor';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, camPath, Spotlight, Stage, Title } from '../ui/Stage';

const FILES = [
  { path: 'src/routes/users.ts', add: 24, del: 3 },
  { path: 'src/db/paginate.ts', add: 41, del: 0 },
  { path: 'tests/users.test.ts', add: 37, del: 2 },
];

/** The composer up close: model, effort and mode, attachments, @ and /; then the card ending the task. */
export const ComposerScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, tok, usd, pick } = useFmt();
  const { agents: AGENTS, status: STATUS } = useDemo();
  const w = (word: string, nth = 0) => c.word('menus', word, nth);
  const fable = w('fable');
  const menu: { which: Menu; pick: number; current: number } | undefined =
    frame < w('modèle') - 4
      ? undefined
      : frame < w('effort') - 4
        ? { which: 'model', pick: 2, current: 2 }
        : frame < w('mode') - 4
          ? { which: 'effort', pick: 2, current: 2 }
          : frame < fable - 6
            ? { which: 'mode', pick: 0, current: 0 }
            : frame < c.end('menus') + 4
              ? {
                  which: 'model',
                  pick: frame < w('opus') ? 0 : frame < w('sonnet') ? 1 : frame < w('haiku') ? 2 : frame < w('version') ? 3 : 1,
                  current: 2,
                }
              : undefined;
  const picked = frame >= c.end('menus') + 4;
  const a = (word: string, nth = 0) => c.word('attach', word, nth);
  const attachments: Attachment[] = [
    ...(frame >= a('image') ? [{ name: tr('capture.png', 'screenshot.png'), kind: 'image' as const }] : []),
    ...(frame >= a('pdf') ? [{ name: tr('maquette.pdf', 'mockup.pdf'), kind: 'pdf' as const }] : []),
    ...(frame >= a('fichier') ? [{ name: 'notes.md', kind: 'text' as const }] : []),
  ];
  const mention = tr('Reprends la maquette pour @src/rou', 'Rework the mockup for @src/rou');
  const text =
    frame >= a('slash') - 4
      ? typed('/rev', frame, fps, a('slash') - 4, 10)
      : frame >= a('cite') - 6
        ? typed(mention, frame, fps, a('cite') - 6, 40)
        : '';
  const done = c.at('done') - 6;
  const cam = camPath(frame, [
    [4, 26, { x: 900, y: 506, zoom: 1.25 }],
    [done - 8, 26, { x: 750, y: 422, zoom: 1 }],
  ]);
  const suggestions =
    frame >= a('slash') ? (
      <Suggestions
        items={[
          { label: '/review', detail: tr('Relit les changements de la branche', 'Reviews the branch’s changes') },
          {
            label: '/compact',
            detail: tr('Résume la conversation pour libérer du contexte', 'Summarizes the conversation to free up context'),
          },
          { label: '/init', detail: tr('Crée un CLAUDE.md pour le projet', 'Creates a CLAUDE.md for the project') },
          { label: '/pr-comments', detail: tr('Lit les commentaires d’une pull request', 'Reads a pull request’s comments') },
        ]}
      />
    ) : frame >= a('arobase') - 8 && frame < c.end('attach') + 10 ? (
      <Suggestions
        items={[
          { label: 'users.ts', detail: 'src/routes' },
          { label: 'invoices.ts', detail: 'src/routes' },
          { label: 'auth.ts', detail: 'src/routes' },
        ]}
      />
    ) : null;
  return (
    <Stage>
      <Title out={cam.zoom > 1 ? Math.min(1, (cam.zoom - 1) * 4) : 0} />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              agents={(picked ? AGENTS : onSonnet(AGENTS)).map((ag) =>
                ag.name === 'pagination-users' && frame >= done ? { ...ag, status: 'done' } : ag,
              )}
              selected="pagination-users"
            />
          }
          overlay={
            <>
              <Spotlight
                x={368}
                y={462}
                w={1112}
                h={212}
                on={ramp(frame, c.word('done', 'carte'), 10) - ramp(frame, c.word('done', 'clic'), 10)}
              />
              <PointerPath
                keys={[
                  [c.word('done', 'clic') - 20, 700, 600],
                  [c.word('done', 'revoir'), pick(458, 432), 636, true],
                  [c.word('done', 'commit') - 2, pick(574, 533), 636, true],
                ]}
              />
            </>
          }
        >
          <ConvHeader
            name="pagination-users"
            status={frame >= done ? 'done' : 'running'}
            sub="demo-api / escouade/pagination-users"
            test="prepare"
            metrics={{ model: picked ? 'Opus 5.5' : 'Sonnet 5.5', tokens: tok(24.8, 1), cost: usd(0.27), files: 3, duration: '4m 02s' }}
          />
          <Conversation>
            <UserMsg text={tr('Et ajoute un test pour la dernière page.', 'And add a test for the last page.')} />
            <ToolCall tool="Edit" target="tests/users.test.ts" meta={<Diffstat add={37} del={2} />} />
            <ToolCall tool="Bash" target="npm test" meta={<span style={{ fontSize: 13, color: C.muted }}>43 tests</span>} />
            <AssistantMsg
              text={tr(
                "C'est fait : la pagination de `/users` est en place, avec ses tests, et la doc de l'API est à jour.",
                'Done: pagination for `/users` is in place, with its tests, and the API docs are up to date.',
              )}
              enter={frame >= done ? 1 : 1}
            />
            <TurnCard
              duration="4m 02s"
              tokens={tok(24.8, 1)}
              cost={usd(0.27)}
              files={FILES}
              enter={pop(frame, fps, done)}
              pressed={{
                review: ramp(frame, c.word('done', 'revoir') - 3, 3) - ramp(frame, c.word('done', 'revoir') + 3, 4),
                commit: ramp(frame, c.word('done', 'commit') - 5, 3) - ramp(frame, c.word('done', 'commit') + 1, 4),
              }}
            />
          </Conversation>
          <Composer
            text={frame < done ? text : ''}
            placeholder={tr('Envoyer un message à pagination-users…', 'Send a message to pagination-users…')}
            model={picked ? 'Opus 5.5' : 'Sonnet 5.5'}
            menu={menu}
            flash={{ model: picked ? ramp(frame, c.end('menus') + 4, 4) - ramp(frame, c.end('menus') + 16, 10) : 0 }}
            attachments={frame < done ? attachments : []}
            suggestions={frame < done ? suggestions : null}
          />
        </Shell>
      </AppWindow>
    </Stage>
  );
};

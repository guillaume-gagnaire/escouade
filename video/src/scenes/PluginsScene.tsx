import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import { PointerPath } from '../ui/Cursor';
import { Chips, PluginRow, PluginsSide, StoreHead, type PluginRowInfo } from '../ui/Plugins';
import { Shell } from '../ui/Shell';
import { AppToast } from '../ui/Toast';
import { AppWindow, Stage, Title } from '../ui/Stage';

/** The harnesses, one plugin each, named as the voice says them; then the store, its seals, and a plugin installed in a click. */
export const PluginsScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const { status } = useDemo();
  const storeAt = c.at('store');
  const inStore = frame >= storeAt - 4;
  const sealed = c.word('store', 'vérifie');
  const glow = ramp(frame, sealed, 8) - ramp(frame, c.end('store'), 14);
  const click = c.at('install') + 4;
  const done = click + 26;

  const HARNESSES: (PluginRowInfo & { said: string })[] = [
    {
      said: 'Claude',
      name: 'Claude Code',
      letters: 'CC',
      hue: 45,
      version: 'v2.2.1',
      author: 'escouade',
      desc: tr('Les agents Claude Code : chat, plan, permissions, sous-agents.', 'Claude Code agents: chat, plan, permissions, subagents.'),
      installed: true,
    },
    {
      said: 'Codex',
      name: 'Codex',
      letters: 'Cx',
      hue: 160,
      version: 'v2.2.0',
      author: 'escouade',
      desc: tr('Codex d’OpenAI, ses comptes et ses quotas.', 'OpenAI’s Codex, its accounts and quotas.'),
      installed: true,
    },
    {
      said: 'Cursor',
      name: 'Cursor CLI',
      letters: 'Cu',
      hue: 250,
      version: 'v2.2.0',
      author: 'escouade',
      desc: tr('Cursor en ligne de commande, ses modèles au choix.', 'Cursor on the command line, its models to pick from.'),
      installed: true,
    },
    {
      said: 'OpenCode',
      name: 'OpenCode',
      letters: 'OC',
      hue: 300,
      version: 'v2.2.0',
      author: 'escouade',
      desc: tr('OpenCode et tous ses fournisseurs de modèles.', 'OpenCode and all its model providers.'),
    },
    {
      said: 'Copilot',
      name: 'Copilot CLI',
      letters: 'Co',
      hue: 210,
      version: 'v2.2.0',
      author: 'escouade',
      desc: tr('GitHub Copilot en ligne de commande.', 'GitHub Copilot on the command line.'),
    },
  ];
  const STORE: PluginRowInfo[] = [
    {
      name: 'Linear',
      letters: 'Li',
      hue: 85,
      version: 'v1.0.0',
      author: 'escouade · 512',
      desc: tr('Les tickets de Linear dans le Kanban, synchronisés dans les deux sens.', 'Linear issues in the Kanban, synced both ways.'),
      runs: 'api.linear.app',
    },
    {
      name: 'ElevenLabs',
      letters: 'EL',
      hue: 20,
      version: 'v1.1.0',
      author: 'escouade · 1 204',
      desc: tr('Parle à tes agents, à voix haute, et écoute-les répondre.', 'Talk to your agents out loud, and hear them answer.'),
      runs: 'api.elevenlabs.io',
      installed: true,
    },
    {
      name: 'GitLab',
      letters: 'GL',
      hue: 35,
      version: 'v2.1.0',
      author: 'escouade · 1 870',
      desc: tr('Les MR des tickets, la CI et les issues de gitlab.com.', 'Merge requests, CI and issues from gitlab.com.'),
      runs: 'gitlab.com',
    },
    {
      name: 'Jira',
      letters: 'Ji',
      hue: 240,
      version: 'v2.4.0',
      author: 'escouade · 2 310',
      desc: tr('Les tickets de Jira importés, leur statut tenu à jour.', 'Jira issues imported, their status kept up to date.'),
      runs: '*.atlassian.net',
      installed: true,
    },
    {
      name: 'Nord',
      letters: 'No',
      hue: 230,
      version: 'v2.0.0',
      author: 'amoreau · 1 120',
      desc: tr('La palette Nord, en clair et en sombre.', 'The Nord palette, light and dark.'),
      runs: tr('Sans programme', 'No program'),
    },
  ];
  const linear = frame >= done ? { ...STORE[0], installed: true } : STORE[0];
  const rows = inStore ? [linear, ...STORE.slice(1)] : HARNESSES;
  const ROW = 86;
  const TOP = 228;
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          view="plugins"
          status={status}
          sidebar={<PluginsSide on={inStore ? 'store' : 'harness'} />}
          overlay={
            <>
              <AppToast
                text={tr('Linear est installé, vérifié et signé.', 'Linear is installed, checked and signed.')}
                tone="ok"
                enter={pop(frame, fps, done + 4, 14)}
              />
              <PointerPath
                keys={[
                  [click - 22, 1100, 600],
                  [click, 1440, TOP + ROW / 2, true],
                  [done + 14, 1446, TOP + ROW / 2 + 8],
                ]}
              />
            </>
          }
        >
          <StoreHead
            title={inStore ? 'Store' : 'Harness'}
            sub={
              inStore
                ? tr('store.escouade.dev · chaque version vérifiée et signée', 'store.escouade.dev · every version checked and signed')
                : tr('6 plugins · un harness par agent, au choix', '6 plugins · one harness per agent, your pick')
            }
          />
          <Chips
            on={inStore ? 0 : 1}
            items={[
              [tr('Tous', 'All'), 47],
              ['Harness', 6],
              ['Kanban', 9],
              [tr('Hébergeurs', 'Code hosts'), 5],
              [tr('Voix', 'Voice'), 4],
              [tr('Thèmes', 'Themes'), 14],
              ['Notifications', 11],
            ]}
          />
          <div style={{ padding: '14px 28px 0', display: 'flex', flexDirection: 'column' }}>
            <div style={{ display: 'flex', fontSize: 14.5, fontWeight: 650, padding: '6px 0 4px' }}>
              {inStore ? tr('Vérifiés par Escouade', 'Verified by Escouade') : tr('Harness', 'Harnesses')}
              <span style={{ flex: 1 }} />
              <span style={{ color: C.muted, fontWeight: 400, fontSize: 13 }}>{tr('Tout voir', 'See all')}</span>
            </div>
            {rows.map((r, i) => {
              const said = 'said' in r ? c.word('harness', (r as { said: string }).said) : 0;
              return (
                <PluginRow
                  key={r.name}
                  {...r}
                  enter={inStore ? pop(frame, fps, storeAt - 4 + i * 3) : pop(frame, fps, c.at('harness') - 8 + i * 3)}
                  highlight={inStore ? 0 : ramp(frame, said - 2, 5) - ramp(frame, said + 18, 10)}
                  glow={inStore ? glow : 0}
                  pressed={inStore && i === 0 ? ramp(frame, click - 2, 3) - ramp(frame, click + 2, 4) : 0}
                  progress={inStore && i === 0 && frame >= click + 2 && frame < done ? ramp(frame, click + 2, done - click - 2) : undefined}
                />
              );
            })}
          </div>
        </Shell>
      </AppWindow>
    </Stage>
  );
};

import type { FC, ReactNode } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { AGENTS, STATUS, TABS } from '../data';
import { C, MONO } from '../theme';
import { PointerPath } from '../ui/Cursor';
import { ContextMenu } from '../ui/Modal';
import { Shell } from '../ui/Shell';
import { AgentsSidebar, TermsSection, type Term } from '../ui/Sidebar';
import { AppWindow, Stage, Title } from '../ui/Stage';

const TERMS: Term[] = [
  { name: 'PowerShell 7', shell: 'pwsh' },
  { name: 'Git Bash', shell: 'bash' },
  { name: 'WSL (Ubuntu)', shell: 'wsl' },
];

const G = '#9bd8a9';
const B = 'oklch(0.74 0.12 235)';
const Y = 'oklch(0.82 0.13 80)';

const Screen: FC<{ title: string; shell: string; children: ReactNode }> = ({ title, shell, children }) => (
  <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
    <div
      style={{
        height: 64,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '0 28px',
        borderBottom: `1px solid ${C.line}`,
      }}
    >
      <span style={{ fontSize: 17, fontWeight: 700 }}>{title}</span>
      <span style={{ fontFamily: MONO, fontSize: 12, padding: '2px 7px', borderRadius: 4, background: C.elev2, color: C.info }}>
        {shell}
      </span>
    </div>
    <div
      style={{
        flex: 1,
        background: C.term,
        padding: '16px 22px',
        fontFamily: MONO,
        fontSize: 16,
        lineHeight: 1.6,
        color: '#d8d0c4',
        whiteSpace: 'pre',
        overflow: 'hidden',
      }}
    >
      {children}
    </div>
  </div>
);

/** On a Mac, the same terminals run zsh, bash or fish. */
const MacShells: FC<{ enter: number }> = ({ enter }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        left: 1150,
        top: 560,
        width: 680,
        borderRadius: 12,
        overflow: 'hidden',
        background: '#1e1e1e',
        border: '1px solid rgba(255,255,255,0.12)',
        boxShadow: '0 30px 90px rgba(0,0,0,0.65)',
        opacity: Math.min(1, enter),
        transform: `translateY(${(1 - Math.min(1, enter)) * 30}px)`,
        fontFamily: MONO,
        zIndex: 60,
      }}
    >
      <div
        style={{
          height: 34,
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '0 14px',
          background: '#2b2b2b',
          color: '#bbbbbb',
          fontSize: 13,
        }}
      >
        {['#ec6a5e', '#f4bf4f', '#61c554'].map((c) => (
          <span key={c} style={{ width: 12, height: 12, borderRadius: 6, background: c }} />
        ))}
        <span style={{ flex: 1, textAlign: 'center' }}>Escouade · macOS</span>
      </div>
      <div style={{ padding: '14px 18px', display: 'flex', flexDirection: 'column', gap: 10, fontSize: 16, color: '#d8d0c4' }}>
        <span style={{ display: 'flex', gap: 8 }}>
          {[
            ['%_', 'zsh'],
            ['$_', 'bash'],
            ['>_', 'fish'],
          ].map(([g, n]) => (
            <span
              key={n}
              style={{
                display: 'flex',
                gap: 8,
                alignItems: 'center',
                padding: '4px 10px',
                borderRadius: 6,
                background: C.elev,
                fontSize: 14,
              }}
            >
              <span style={{ color: n === 'fish' ? 'oklch(0.78 0.13 60)' : G, fontWeight: 700 }}>{g}</span>
              {n}
            </span>
          ))}
        </span>
        <span>
          <span style={{ color: G }}>lea@mac</span> <span style={{ color: B }}>demo-api</span> % git switch pagination-users
        </span>
      </div>
    </div>
  );

const Caret = () => <span style={{ display: 'inline-block', width: 9, height: 19, background: '#d8d0c4', verticalAlign: -3 }} />;

/** A real terminal per shell: completion in PowerShell, history search in Git Bash, a full-screen program in WSL. */
export const Terminals: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const w = (word: string) => c.word('shells', word);
  const menu = w('terminal') - 4;
  const pick = w('autocomplétion') - 8;
  const which = frame < w('historique') - 6 ? 0 : frame < w('plein') - 6 ? 1 : 2;
  const opened = frame >= pick;
  const tabbed = frame >= pick + 34;
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.slice(0, 3)}
              foot={false}
              after={<TermsSection terms={opened ? TERMS : []} selected={opened ? which : undefined} />}
            />
          }
          overlay={
            <>
              <ContextMenu
                x={150}
                y={640}
                enter={frame >= menu && frame < pick ? 1 : 0}
                hover={frame < w('guite') - 4 ? 0 : frame < w('wsl') - 4 ? 1 : 2}
                items={[
                  { label: 'PowerShell 7', hint: 'PS' },
                  { label: 'Git Bash', hint: '$_' },
                  { label: 'WSL (Ubuntu)', hint: 'λ' },
                ]}
              />
              <PointerPath
                keys={[
                  [menu - 20, 600, 500],
                  [menu, 268, 780, true],
                  [pick - 10, 230, 663],
                  [pick - 2, 230, 663, true],
                ]}
              />
            </>
          }
        >
          {!opened ? (
            <Screen title="Terminaux" shell="—">
              <span style={{ color: C.dim }}>Ouvre un terminal : PowerShell, Git Bash ou WSL.</span>
            </Screen>
          ) : which === 0 ? (
            <Screen title="PowerShell 7" shell="pwsh">
              <div>
                <span style={{ color: B }}>PS C:\dev\demo-api&gt;</span>{' '}
                {tabbed ? 'git checkout ' : typed('git chec', frame, fps, pick + 6, 14)}
                <Caret />
              </div>
              {tabbed ? (
                <div style={{ marginTop: 8, display: 'flex', gap: 30 }}>
                  {['main', 'escouade/pagination-users', 'ticket/DEM-5', 'ticket/DEM-6'].map((b, i) => (
                    <span
                      key={b}
                      style={{ padding: '0 6px', background: i === 1 ? C.elev2 : 'transparent', color: i === 1 ? '#ffffff' : G }}
                    >
                      {b}
                    </span>
                  ))}
                </div>
              ) : null}
            </Screen>
          ) : which === 1 ? (
            <Screen title="Git Bash" shell="bash">
              <div style={{ color: G }}>
                léa@poste <span style={{ color: Y }}>MINGW64</span> <span style={{ color: B }}>~/dev/demo-api</span> (main)
              </div>
              <div>
                (reverse-i-search)`{typed('npm r', frame, fps, w('historique'), 8)}':{' '}
                <span style={{ color: '#ffffff' }}>npm run test -- --watch</span>
              </div>
            </Screen>
          ) : (
            <Screen title="WSL (Ubuntu)" shell="wsl">
              {[0, 1, 2, 3].map((i) => {
                const v = 20 + ((frame * (i + 3)) % 60);
                return (
                  <div key={i}>
                    <span style={{ color: B }}>{i}</span>[<span style={{ color: G }}>{'|'.repeat(Math.round(v / 3))}</span>
                    {' '.repeat(Math.max(0, 33 - Math.round(v / 3)))}
                    <span style={{ color: C.dim }}>{v.toFixed(1)}%</span>]
                  </div>
                );
              })}
              <div>
                <span style={{ color: B }}>Mem</span>[<span style={{ color: G }}>{'|'.repeat(18)}</span>
                <span style={{ color: Y }}>{'|'.repeat(6)}</span>
                {' '.repeat(9)}
                <span style={{ color: C.dim }}>5.21G/15.5G</span>]
              </div>
              <div style={{ marginTop: 10, background: G, color: '#131210' }}>
                {'  PID USER      CPU% MEM%   TIME+  Command                                      '}
              </div>
              {[
                ['  812 lea       12.4  2.1  0:42.18 node worker.js'],
                [' 1024 lea        6.0  4.3  1:03.77 claude -p --input-format stream-json'],
                [' 1188 lea        3.2  1.2  0:12.04 npm run dev'],
                [' 1290 lea        0.7  0.4  0:01.92 bash'],
              ].map(([l]) => (
                <div key={l}>{l}</div>
              ))}
              <div style={{ marginTop: 10, color: C.dim }}>F1Aide F2Config F3Chercher F9Tuer F10Quitter</div>
            </Screen>
          )}
        </Shell>
      </AppWindow>
      <MacShells enter={pop(frame, fps, w('zsh') - 6, 16) - ramp(frame, w('autocomplétion') - 10, 10)} />
    </Stage>
  );
};

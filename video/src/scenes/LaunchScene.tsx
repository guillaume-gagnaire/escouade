import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt, type Tr } from '../lang';
import { C, MONO } from '../theme';
import { PointerPath } from '../ui/Cursor';
import { Field, Modal } from '../ui/Modal';
import { LogView, RunsSection, type Run } from '../ui/Runs';
import { Shell } from '../ui/Shell';
import { AgentsSidebar, Button } from '../ui/Sidebar';
import { AppWindow, Sfx, Stage, Title } from '../ui/Stage';
import { AppToast } from '../ui/Toast';

/** The project's folder, as the app shortens it. */
const ROOT = '~\\dev\\demo-api';
const COMMANDS = [
  { name: 'Front', command: 'npm run dev', shell: 'PowerShell 7', dir: 'web', word: 'front' },
  { name: 'API', command: 'npm run api', shell: 'PowerShell 7', dir: 'api', word: 'api' },
  { name: 'Worker', command: 'node worker.js', shell: 'Git Bash', dir: 'api', word: 'worker' },
];
const FRONT = [
  '$ npm run dev',
  '',
  '> demo-web@2.4.0 dev',
  '> vite',
  '',
  '  VITE v6.4.3  ready in 412 ms',
  '',
  '  ➜  Local:   http://localhost:5173/',
  '  ➜  Network: use --host to expose',
  '',
  '  12:04:31 [vite] page reload src/routes/users.ts',
];
const workerOf = (tr: Tr) => [
  '$ node worker.js',
  tr('worker: connexion à la file « factures »…', 'worker: connecting to the “invoices” queue…'),
  'Error: connect ECONNREFUSED 127.0.0.1:6379',
  '    at TCPConnectWrap.afterConnect [as oncomplete] (node:net:1611:16)',
  '',
  tr('── terminé · code 1 ──', '── finished · code 1 ──'),
];

/** The launch commands: configured, all started, one crashes and says so. */
export const LaunchScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const { agents: AGENTS, status: STATUS } = useDemo();
  const close = c.end('config') + 4;
  const all = c.word('run', 'tout') + 2;
  const started = (i: number) => frame >= all + 6 + i * 6;
  const crash = c.word('crash', 'plante');
  const crashed = frame >= crash;
  const showWorker = frame >= c.word('crash', 'code');
  const runs: Run[] = COMMANDS.map((x, i) => ({
    name: x.name,
    status: i === 2 && crashed ? 'crashed' : started(i) ? 'running' : 'ready',
    code: 1,
  }));
  const shownLines = frame < all + 8 ? 0 : Math.min(FRONT.length, Math.floor((frame - all - 8) / 5) + 1);
  return (
    <Stage>
      <Title />
      <Sfx at={crash + 2} name="chime" />
      <AppWindow>
        <Shell
          tabs={TABS}
          status={STATUS}
          sidebar={
            <AgentsSidebar
              agents={AGENTS.slice(0, 3)}
              foot={false}
              after={
                <RunsSection
                  runs={runs}
                  selected={showWorker ? 'Worker' : 'Front'}
                  pressed={ramp(frame, all - 3, 3) - ramp(frame, all + 3, 4)}
                />
              }
            />
          }
          overlay={
            <>
              <Modal
                title={tr('Commandes de lancement · demo-api', 'Launch commands · demo-api')}
                width={720}
                enter={frame >= close ? 0 : pop(frame, fps, 0, 18)}
                footer={
                  <>
                    <Button>{tr('Annuler', 'Cancel')}</Button>
                    <Button primary={C.spark}>{tr('Enregistrer', 'Save')}</Button>
                  </>
                }
              >
                {COMMANDS.map((x) => {
                  const at = c.word('config', x.word);
                  const e = Math.min(1, pop(frame, fps, at - 8));
                  return (
                    <div
                      key={x.name}
                      style={{
                        display: 'flex',
                        flexDirection: 'column',
                        gap: 8,
                        padding: 12,
                        borderRadius: 8,
                        border: `1px solid ${C.line}`,
                        opacity: e,
                        transform: `translateY(${(1 - e) * 14}px)`,
                      }}
                    >
                      <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
                        <Field value={x.name} />
                        <span
                          style={{
                            height: 38,
                            display: 'flex',
                            alignItems: 'center',
                            gap: 8,
                            padding: '0 12px',
                            borderRadius: 6,
                            border: `1px solid ${frame >= c.word('config', 'shell') - 4 && frame < c.word('config', 'dossier') ? C.spark : C.line2}`,
                            background: C.bg,
                            fontSize: 14,
                          }}
                        >
                          {x.shell} <span style={{ color: C.dim, fontSize: 10 }}>▾</span>
                        </span>
                        <span style={{ color: C.dim }}>×</span>
                      </div>
                      <div style={{ display: 'flex', gap: 10 }}>
                        <Field value={x.command} mono />
                        <span
                          style={{
                            width: 150,
                            height: 38,
                            display: 'flex',
                            alignItems: 'center',
                            padding: '0 12px',
                            borderRadius: 6,
                            border: `1px solid ${frame >= c.word('config', 'dossier') - 4 ? C.spark : C.line2}`,
                            background: C.bg,
                            fontFamily: MONO,
                            fontSize: 13.5,
                          }}
                        >
                          {x.dir}
                        </span>
                      </div>
                    </div>
                  );
                })}
                <span style={{ alignSelf: 'flex-start' }}>
                  <Button>{tr('+ Ajouter une commande', '+ Add a command')}</Button>
                </span>
                <span style={{ fontSize: 13, color: C.muted }}>
                  {tr('Fichiers copiés dans les worktrees : ', 'Files copied into worktrees: ')}
                  <span style={{ fontFamily: MONO, color: C.text }}>.env*</span>
                </span>
              </Modal>
              <AppToast
                text={tr("« Worker » s'est arrêté en erreur (code 1)", '“Worker” stopped with an error (code 1)')}
                enter={pop(frame, fps, crash + 2, 16)}
              />
              <PointerPath
                keys={[
                  [all - 26, 700, 400],
                  [all, 203, 678, true],
                ]}
              />
            </>
          }
        >
          {showWorker ? (
            <LogView
              title="Worker"
              status={tr('planté (code 1)', 'crashed (code 1)')}
              state="ended"
              statusColor={C.del}
              lines={workerOf(tr)}
              shell="Git Bash"
              command="node worker.js"
              where={`${ROOT}\\api`}
            />
          ) : (
            <LogView
              title="Front"
              status={started(0) ? tr('en cours', 'running') : tr('prêt', 'ready')}
              state={started(0) ? 'running' : 'never'}
              statusColor={started(0) ? C.ok : C.dim}
              lines={FRONT.slice(0, shownLines)}
              command="npm run dev"
              where={`${ROOT}\\web`}
            />
          )}
        </Shell>
      </AppWindow>
    </Stage>
  );
};

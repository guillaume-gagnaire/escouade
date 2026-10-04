import type { FC } from 'react';
import { C, MONO, UI } from '../theme';
import { Dot } from './Shell';
import { Button, SectionHead } from './Sidebar';

export type RunStatus = 'ready' | 'running' | 'crashed' | 'stopped' | 'done';

export interface Run {
  name: string;
  status: RunStatus;
  code?: number;
}

const LABEL = (r: Run) =>
  r.status === 'ready'
    ? 'prêt'
    : r.status === 'running'
      ? 'en cours'
      : r.status === 'stopped'
        ? 'arrêté'
        : r.status === 'done'
          ? 'terminé'
          : `planté (code ${r.code})`;
const COLOR: Record<RunStatus, string> = { ready: C.dim, running: C.ok, crashed: C.del, stopped: C.dim, done: C.ok };

const Row: FC<{ r: Run; selected: boolean }> = ({ r, selected }) => (
  <div
    style={{
      display: 'flex',
      alignItems: 'center',
      gap: 9,
      height: 34,
      padding: '0 10px 0 12px',
      borderRadius: 6,
      background: selected ? C.elev : 'transparent',
      border: `1px solid ${selected ? C.line2 : 'transparent'}`,
    }}
  >
    <Dot color={COLOR[r.status]} size={7} pulse={r.status === 'running'} />
    <span style={{ flex: 1, fontSize: 14, fontWeight: 600 }}>{r.name}</span>
    <span style={{ fontFamily: MONO, fontSize: 11.5, color: COLOR[r.status] }}>{LABEL(r)}</span>
    {r.status === 'running' ? (
      <span style={{ display: 'flex', gap: 8, color: C.muted, fontSize: 11 }}>
        <span>⟳</span>
        <span>■</span>
      </span>
    ) : (
      <span style={{ width: 16, color: C.ok, fontSize: 11 }}>▶</span>
    )}
  </div>
);

/** The "Lancement" section: the project's commands, then each agent's test steps under its name. */
export const RunsSection: FC<{
  runs: Run[];
  groups?: { agent: string; runs: Run[] }[];
  selected?: string;
  pressed?: number;
}> = ({ runs, groups = [], selected, pressed = 0 }) => {
  const running = runs.filter((r) => r.status === 'running').length;
  return (
    <div style={{ flex: 'none', margin: '0 -10px', borderTop: `1px solid ${C.line}`, padding: '6px 10px 8px' }}>
      <SectionHead
        label="Lancement"
        count={`${running}/${runs.length}`}
        action={
          <span style={{ display: 'flex', gap: 6 }}>
            <Button pressed={pressed} small>
              {running ? 'Tout arrêter' : 'Tout lancer'}
            </Button>
            <Button small>⚙</Button>
          </span>
        }
      />
      {runs.map((r) => (
        <Row key={r.name} r={r} selected={selected === r.name} />
      ))}
      {groups.map((g) => (
        <div key={g.agent}>
          <div style={{ padding: '8px 12px 4px', fontFamily: MONO, fontSize: 11.5, color: C.dim }}>{g.agent}</div>
          {g.runs.map((r) => (
            <Row key={g.agent + r.name} r={r} selected={selected === `${g.agent}/${r.name}`} />
          ))}
        </div>
      ))}
    </div>
  );
};

/**
 * A read-only log, as in the app's launch terminals (RunView.svelte): « depuis », Relancer and Stopper while it runs,
 * Relancer once it ended, Lancer before its first run.
 */
export const LogView: FC<{
  lines: string[];
  title: string;
  status: string;
  state: 'running' | 'ended' | 'never';
  statusColor: string;
  shell?: string;
  command?: string;
  where?: string;
  since?: string;
}> = ({ lines, title, status, state, statusColor, shell = 'PowerShell', command, where = '~\\dev\\demo-api', since = '14:02' }) => (
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
      <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <span style={{ fontSize: 17, fontWeight: 700 }}>{title}</span>
          <span style={{ fontFamily: MONO, fontSize: 12, padding: '2px 7px', borderRadius: 4, background: C.elev2, color: C.info }}>
            {shell}
          </span>
          <span style={{ fontFamily: MONO, fontSize: 13, color: statusColor, fontWeight: 600 }}>{status}</span>
          {state === 'running' ? <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>depuis {since}</span> : null}
        </span>
        {command ? (
          <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>
            <span style={{ color: C.muted }}>{command}</span> · {where}
          </span>
        ) : null}
      </div>
      <div style={{ flex: 1 }} />
      <span style={{ display: 'flex', gap: 8, fontSize: 13 }}>
        <Button small>⌕</Button>
        <Button small>Effacer</Button>
        {state === 'running' ? (
          <>
            <Button small>⟳ Relancer</Button>
            <Button small>
              <span style={{ color: C.del }}>■ Stopper</span>
            </Button>
          </>
        ) : (
          <Button small primary={C.spark}>
            {state === 'ended' ? '⟳ Relancer' : '▶ Lancer'}
          </Button>
        )}
      </span>
    </div>
    <div
      style={{
        flex: 1,
        background: C.term,
        padding: '16px 22px',
        fontFamily: MONO,
        fontSize: 15,
        lineHeight: 1.6,
        color: '#d8d0c4',
        whiteSpace: 'pre',
        overflow: 'hidden',
        position: 'relative',
      }}
    >
      {/* Before its first run (RunView.svelte). */}
      {state === 'never' ? (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 14,
            fontFamily: UI,
            color: C.dim,
          }}
        >
          <span style={{ fontSize: 15 }}>Pas encore lancée.</span>
          <Button primary={C.spark}>▶ Lancer</Button>
        </div>
      ) : null}
      {lines.map((l, i) => (
        <div
          key={i}
          style={{
            color:
              l.startsWith('$') || l.startsWith('──')
                ? C.dim
                : /ready|listening|prêt/i.test(l)
                  ? '#9bd8a9'
                  : /error|Error|code 1/.test(l)
                    ? '#e8877a'
                    : undefined,
          }}
        >
          {l || ' '}
        </div>
      ))}
    </div>
  </div>
);

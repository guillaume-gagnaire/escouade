import type { FC, ReactNode } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { onSonnet, TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C, MONO, soft } from '../theme';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, Stage, Title } from '../ui/Stage';

// The diagram of a workflow (src/components/WorkflowDiagram.svelte, design/workflow-path, direction A): from the entry the
// agent came in by, its steps drawn as boxes, what may happen dashed, the agent's path traced over it in full lines, each
// passage numbered on its step, a way back in red.

const RED = 'oklch(0.72 0.15 22)';

interface Step {
  x: number;
  name: string;
  cmd: string;
  passes: number[];
  state?: 'now' | 'next';
  agent?: { name: string; color: string };
}

const Num: FC<{ n: number; now?: boolean; on: number }> = ({ n, now, on }) =>
  on <= 0 ? null : (
    <span
      style={{
        width: 22,
        height: 22,
        display: 'grid',
        placeItems: 'center',
        borderRadius: 11,
        background: now ? C.ok : C.text,
        color: C.onPrimary,
        fontSize: 12,
        fontWeight: 700,
        transform: `scale(${Math.min(1.2, on)})`,
      }}
    >
      {n}
    </span>
  );

const Box: FC<{ s: Step; y: number; enter: number; shown: (n: number) => number; agentIn: number }> = ({ s, y, enter, shown, agentIn }) => {
  const { tr } = useFmt();
  const now = s.state === 'now';
  return (
    <div
      style={{
        position: 'absolute',
        left: s.x,
        top: y,
        width: 232,
        opacity: Math.min(1, enter),
        transform: `scale(${0.9 + 0.1 * Math.min(1, enter)})`,
      }}
    >
      <div
        style={{
          height: 84,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          padding: '11px 12px 10px 14px',
          borderRadius: 10,
          background: now ? soft(C.ok, 9) : C.elev,
          boxShadow: `inset 0 0 0 ${now ? 2 : 1.5}px ${now ? C.ok : C.line3}${now ? `, 0 0 26px ${soft(C.ok, 25)}` : ''}`,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 5 }}>
          <b style={{ flex: 1, fontSize: 16, fontWeight: 650 }}>{s.name}</b>
          {s.passes.map((n, i) => (
            <Num key={n} n={n} now={now && i === s.passes.length - 1} on={shown(n)} />
          ))}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, whiteSpace: 'nowrap' }}>
          <span style={{ flex: 1, fontFamily: MONO, fontSize: 12, color: C.muted, overflow: 'hidden', textOverflow: 'ellipsis' }}>
            {s.cmd}
          </span>
          {now && shown(6) > 0 ? <span style={{ color: C.ok, fontSize: 13, fontWeight: 600 }}>{tr('en cours', 'running')}</span> : null}
          {s.state === 'next' && shown(6) > 0 ? <span style={{ color: C.muted, fontSize: 13 }}>{tr('prochaine', 'next')}</span> : null}
        </div>
      </div>
      {s.agent && agentIn > 0 ? (
        <span
          style={{
            marginTop: 9,
            display: 'inline-flex',
            alignItems: 'center',
            gap: 7,
            height: 24,
            padding: '0 10px',
            borderRadius: 12,
            boxShadow: `inset 0 0 0 1px ${C.line2}`,
            background: C.bg,
            fontSize: 13,
            opacity: Math.min(1, agentIn),
            transform: `translateY(${(1 - Math.min(1, agentIn)) * 10}px)`,
          }}
        >
          <span style={{ width: 7, height: 7, borderRadius: 4, background: s.agent.color }} />
          {s.agent.name}
        </span>
      ) : null}
    </div>
  );
};

const Pill: FC<{ x: number; y: number; children: ReactNode; dashed?: boolean; enter: number }> = ({ x, y, children, dashed, enter }) => (
  <div
    style={{
      position: 'absolute',
      left: x,
      top: y,
      height: 36,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '0 14px',
      borderRadius: 18,
      border: `1.5px ${dashed ? 'dashed' : 'solid'} ${dashed ? C.line3 : C.soft}`,
      background: C.bg,
      fontSize: 14,
      whiteSpace: 'nowrap',
      opacity: Math.min(1, enter),
    }}
  >
    {children}
  </div>
);

/** A workflow of the repository, drawn: its steps, the agents on them, then the path the agent took, passage by passage. */
export const WorkflowScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const { agents, status, refactor } = useDemo();
  const steps = c.word('draw', 'étape');
  const who = c.word('draw', 'agent');
  const path = c.word('draw', 'chemin');
  const trace = ramp(frame, path - 6, Math.max(20, c.end('draw') - path + 6));
  /** Passages 1 to 6, one after the other along the trace. */
  const shown = (n: number) => pop(frame, fps, path - 6 + (n - 1) * ((c.end('draw') - path) / 6));
  const Y = 250;
  const STEPS: Step[] = [
    { x: 200, name: tr('Planifier', 'Plan'), cmd: '/gsd:plan-phase', passes: [1, 2, 5], agent: { name: 'onboarding', color: C.muted } },
    { x: 486, name: tr('Exécuter', 'Execute'), cmd: '/gsd:execute-phase', passes: [3, 6], state: 'now' },
    {
      x: 772,
      name: tr('Vérifier', 'Verify'),
      cmd: '/gsd:verify-work',
      passes: [4],
      state: 'next',
      agent: { name: refactor, color: C.wait },
    },
  ];
  const mid = Y + 42;
  const solid = (x1: number, x2: number, on: number) => (
    <>
      <line x1={x1} y1={mid} x2={x2} y2={mid} stroke={C.line3} strokeWidth="1.5" strokeDasharray="5 5" />
      <line x1={x1} y1={mid} x2={x1 + (x2 - x1) * on} y2={mid} stroke={C.text} strokeWidth="2" />
      {on >= 1 ? (
        <path d={`M${x2 - 8} ${mid - 6} L${x2} ${mid} L${x2 - 8} ${mid + 6}`} fill="none" stroke={C.text} strokeWidth="2" />
      ) : null}
    </>
  );
  const seg = (k: number) => Math.max(0, Math.min(1, trace * 4 - k));
  const back = seg(3);
  return (
    <Stage>
      <Title />
      <AppWindow>
        <Shell tabs={TABS} status={status} sidebar={<AgentsSidebar agents={onSonnet(agents)} selected={refactor} />}>
          <div
            style={{ display: 'flex', alignItems: 'center', gap: 10, height: 58, padding: '0 22px', borderBottom: `1px solid ${C.line}` }}
          >
            <b style={{ fontSize: 16 }}>GSD</b>
            <span style={{ color: C.muted, fontSize: 13.5 }}>
              · {tr('entré par', 'entered by')} <span style={{ fontFamily: MONO, fontSize: 12.5 }}>/gsd:plan-phase</span> · {refactor},{' '}
              {tr('6 passages', '6 passes')}
            </span>
            <span style={{ flex: 1 }} />
            {['− 100 % +', tr('Ajuster', 'Fit'), tr('Copier en Mermaid', 'Copy as Mermaid'), '✕'].map((b) => (
              <span
                key={b}
                style={{
                  height: 30,
                  display: 'grid',
                  placeItems: 'center',
                  padding: '0 11px',
                  borderRadius: 7,
                  boxShadow: `inset 0 0 0 1px ${C.line2}`,
                  fontSize: 13,
                }}
              >
                {b}
              </span>
            ))}
          </div>
          <div
            style={{
              position: 'relative',
              flex: 1,
              backgroundImage: `radial-gradient(${C.line2} 1px, transparent 1px)`,
              backgroundSize: '22px 22px',
            }}
          >
            <svg width="1152" height="560" style={{ position: 'absolute', left: 0, top: 0, overflow: 'visible' }}>
              {solid(178, 200, seg(0))}
              {solid(432, 486, seg(1))}
              {solid(718, 772, seg(2))}
              <line x1={1004} y1={mid} x2={1040} y2={mid} stroke={C.line3} strokeWidth="1.5" strokeDasharray="5 5" />
              {/* The way back, from Vérifier to Planifier: taken. */}
              <path
                d={`M 990 ${Y + 84} V ${Y + 146} H 416 V ${Y + 92}`}
                fill="none"
                stroke={soft(RED, 40)}
                strokeWidth="1.5"
                strokeDasharray="5 5"
              />
              <path
                d={`M 990 ${Y + 84} V ${Y + 146} H 416 V ${Y + 92}`}
                fill="none"
                stroke={RED}
                strokeWidth="2.2"
                pathLength={1}
                strokeDasharray="1"
                strokeDashoffset={1 - back}
              />
              {back >= 1 ? (
                <path d={`M 410 ${Y + 100} L 416 ${Y + 90} L 422 ${Y + 100}`} fill="none" stroke={RED} strokeWidth="2.2" />
              ) : null}
            </svg>
            <Pill x={28} y={mid - 18} enter={ramp(frame, steps - 10, 8)}>
              <span style={{ color: C.muted }}>{tr('Entrée', 'Entry')}</span>
              <span style={{ fontFamily: MONO, fontSize: 13 }}>/gsd:plan</span>
            </Pill>
            {STEPS.map((s, i) => (
              <Box
                key={s.name}
                s={s}
                y={Y}
                enter={pop(frame, fps, steps - 6 + i * 5)}
                shown={shown}
                agentIn={pop(frame, fps, who + i * 4)}
              />
            ))}
            <Pill x={1040} y={mid - 18} dashed enter={ramp(frame, steps + 12, 8)}>
              ◎ {tr('Fini', 'Done')}
            </Pill>
            {back > 0.5 ? (
              <span
                style={{
                  position: 'absolute',
                  left: 520,
                  top: Y + 132,
                  height: 28,
                  display: 'flex',
                  alignItems: 'center',
                  padding: '0 12px',
                  borderRadius: 14,
                  border: `1.5px solid ${RED}`,
                  background: C.bg,
                  color: RED,
                  fontSize: 13.5,
                  fontWeight: 600,
                  opacity: Math.min(1, (back - 0.5) * 3),
                }}
              >
                ↩ {tr('écarts : 2 tests rouges dans checkout.spec.ts', 'gaps: 2 red tests in checkout.spec.ts')}
              </span>
            ) : null}
            <div
              style={{
                position: 'absolute',
                left: 0,
                right: 0,
                bottom: 0,
                height: 50,
                display: 'flex',
                alignItems: 'center',
                gap: 22,
                padding: '0 22px',
                borderTop: `1px solid ${C.line}`,
                background: C.bg,
                fontSize: 13,
                color: C.muted,
              }}
            >
              {[
                [C.text, false, tr("pris par l'agent", 'taken by the agent')],
                [C.line3, true, tr('possible', 'possible')],
                [RED, false, tr('retour pris', 'way back taken')],
              ].map(([color, dashed, label]) => (
                <span key={String(label)} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ width: 24, borderTop: `2px ${dashed ? 'dashed' : 'solid'} ${color}` }} />
                  {label}
                </span>
              ))}
            </div>
          </div>
        </Shell>
      </AppWindow>
    </Stage>
  );
};

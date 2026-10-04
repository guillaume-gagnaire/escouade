import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { C, MONO, soft } from '../theme';
import { blink, Dot } from './Shell';

export type AgentStatus = 'running' | 'waiting' | 'idle' | 'done' | 'error';

export interface AgentInfo {
  name: string;
  status: AgentStatus;
  model: string;
  time: string;
  tokens: string;
  cost: string;
  files: number;
  /** Its worktree's branch. */
  branch?: string;
  /** « DEM-4 · boucle 2/5 ». */
  ticket?: string;
  remote?: boolean;
  /** « Reprise à 14:05 »: stopped by the usage limit. */
  resume?: string;
}

const LABEL: Record<AgentStatus, string> = { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' };
export const STATUS_COLOR: Record<AgentStatus, string> = { running: C.ok, waiting: C.wait, idle: C.dim, done: C.ok, error: C.del };

export const SectionHead: FC<{ label: string; count?: string; action?: ReactNode }> = ({ label, count, action }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 8px 8px' }}>
    <span style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: C.muted }}>{label}</span>
    {count ? <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>{count}</span> : null}
    <div style={{ flex: 1 }} />
    {action}
  </div>
);

export const Button: FC<{ children: ReactNode; pressed?: number; accent?: boolean; primary?: string; small?: boolean }> = ({
  children,
  pressed = 0,
  accent,
  primary,
  small,
}) => (
  <span
    style={{
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 6,
      height: small ? 26 : 30,
      padding: small ? '0 9px' : '0 12px',
      borderRadius: 6,
      border: `1px solid ${primary ? primary : accent ? C.spark : C.line2}`,
      background: primary ? primary : pressed ? soft(C.spark, 30 * pressed) : C.elev,
      color: primary ? '#1b1512' : C.text,
      fontSize: small ? 12.5 : 13.5,
      fontWeight: primary ? 700 : 600,
      whiteSpace: 'nowrap',
      transform: `scale(${1 - 0.06 * pressed})`,
      flex: 'none',
    }}
  >
    {children}
  </span>
);

/** « Agents | Tableau » at the top of the sidebar; `badge`: tickets to test. */
export const Switcher: FC<{ board: boolean; badge?: number }> = ({ board, badge }) => (
  <div style={{ display: 'flex', gap: 2, padding: 3, borderRadius: 10, background: C.bg, border: `1px solid ${C.line}`, flex: 'none' }}>
    {[false, true].map((b) => (
      <span
        key={String(b)}
        style={{
          flex: 1,
          height: 32,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          gap: 8,
          borderRadius: 6,
          background: board === b ? C.elev2 : 'transparent',
          color: board === b ? C.text : C.muted,
          fontSize: 14,
          fontWeight: 600,
        }}
      >
        {b ? 'Tableau' : 'Agents'}
        {b && badge ? (
          <span
            style={{
              minWidth: 19,
              height: 19,
              padding: '0 5px',
              borderRadius: 10,
              background: C.wait,
              color: '#2a1f05',
              fontFamily: MONO,
              fontSize: 11,
              fontWeight: 700,
              display: 'inline-flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            {badge}
          </span>
        ) : null}
      </span>
    ))}
  </div>
);

const Wifi: FC<{ on: boolean }> = ({ on }) => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={on ? C.info : C.dim} strokeWidth="2.2" strokeLinecap="round">
    <path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0" />
    <circle cx="12" cy="19.5" r="1" fill={on ? C.info : C.dim} />
  </svg>
);

export const AgentCard: FC<
  AgentInfo & { selected?: boolean; enter?: number; ring?: boolean; alert?: string; nameOverride?: ReactNode }
> = ({
  name,
  status,
  model,
  time,
  tokens,
  cost,
  files,
  branch,
  ticket,
  remote,
  resume,
  selected,
  enter = 1,
  ring,
  alert,
  nameOverride,
}) => {
  const frame = useCurrentFrame();
  const a = alert ? blink(frame) : 0;
  if (enter <= 0) return null;
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 7,
        padding: '11px 12px',
        borderRadius: 10,
        border: `1px solid ${alert ? alert : selected ? C.line2 : 'transparent'}`,
        background: alert ? soft(alert, 18 * a) : selected ? C.elev : 'transparent',
        opacity: Math.min(1, enter),
        transform: `translateX(${(1 - Math.min(1, enter)) * -40}px)`,
        boxShadow: ring ? `0 0 0 2px ${soft(C.wait, 70)}, 0 0 28px ${soft(C.wait, 35)}` : 'none',
        flex: 'none',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 9 }}>
        <Dot color={STATUS_COLOR[status]} pulse={status === 'running' || status === 'waiting'} />
        <span style={{ flex: 1, fontSize: 15, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {nameOverride ?? name}
        </span>
        {remote !== undefined ? <Wifi on={remote} /> : null}
        {status === 'waiting' ? (
          <span style={{ fontSize: 12, fontWeight: 700, color: '#1b1512', background: C.wait, borderRadius: 9, padding: '1px 8px' }}>
            Question
          </span>
        ) : resume ? (
          <span style={{ fontSize: 12, color: C.wait }}>{resume}</span>
        ) : (
          <span style={{ fontSize: 12, color: STATUS_COLOR[status] }}>{LABEL[status]}</span>
        )}
      </div>
      <div style={{ display: 'flex', gap: 9, paddingLeft: 17, fontFamily: MONO, fontSize: 12, color: C.muted, whiteSpace: 'nowrap' }}>
        <span>{model}</span>
        <span style={{ color: C.dim }}>·</span>
        <span>{time}</span>
        {branch ? (
          <>
            <span style={{ color: C.dim }}>·</span>
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>⎇ {branch}</span>
          </>
        ) : null}
      </div>
      <div style={{ display: 'flex', gap: 10, paddingLeft: 17, fontFamily: MONO, fontSize: 12, color: C.dim }}>
        <span>{tokens} tok</span>
        <span>{cost}</span>
        <span>{files} fich.</span>
      </div>
      {ticket ? (
        <span
          style={{
            marginLeft: 17,
            alignSelf: 'flex-start',
            fontFamily: MONO,
            fontSize: 11.5,
            padding: '2px 7px',
            borderRadius: 3,
            background: C.elev2,
            color: C.spark,
          }}
        >
          ▸ {ticket}
        </span>
      ) : null}
    </div>
  );
};

/** The project's folder and branch, at the bottom of the sidebar. */
export const SidebarFoot: FC<{ path?: string; branch?: string; counts?: [number, number, number] }> = ({
  path = '~/dev/demo-api',
  branch = 'main',
  counts = [5, 2, 0],
}) => (
  <div
    style={{
      flex: 'none',
      margin: '0 -10px',
      borderTop: `1px solid ${C.line}`,
      padding: '12px 18px 14px',
      display: 'flex',
      flexDirection: 'column',
      gap: 7,
      fontFamily: MONO,
      fontSize: 12,
    }}
  >
    <span style={{ color: C.muted }}>{path}</span>
    <span style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
      <span style={{ width: 7, height: 7, borderRadius: '50%', border: `1.5px solid ${C.muted}` }} />
      {branch}
      <span style={{ flex: 1 }} />
      <span style={{ fontSize: 11, padding: '1px 8px', border: `1px solid ${C.line2}`, borderRadius: 5, color: C.muted }}>Parcourir</span>
    </span>
    <span style={{ display: 'flex', gap: 12, fontSize: 11.5 }}>
      <span style={{ color: C.wait }}>~{counts[0]} modifiés</span>
      <span style={{ color: C.ok }}>+{counts[1]} ajoutés</span>
      <span style={{ color: C.del }}>−{counts[2]} supprimés</span>
    </span>
  </div>
);

export interface Term {
  name: string;
  shell: 'pwsh' | 'bash' | 'wsl';
}
const GLYPH = { pwsh: { g: 'PS', c: C.info }, bash: { g: '$_', c: C.ok }, wsl: { g: 'λ', c: 'oklch(0.78 0.13 60)' } };

export const TermsSection: FC<{ terms: Term[]; selected?: number }> = ({ terms, selected }) => (
  <div style={{ flex: 'none', margin: '0 -10px', borderTop: `1px solid ${C.line}`, padding: '6px 10px 8px' }}>
    <SectionHead label="Terminaux" count={String(terms.length)} action={<Button small>+</Button>} />
    {terms.map((t, i) => (
      <div
        key={t.name}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          height: 34,
          padding: '0 10px',
          borderRadius: 6,
          background: i === selected ? C.elev : 'transparent',
          border: `1px solid ${i === selected ? C.line2 : 'transparent'}`,
        }}
      >
        <span
          style={{
            fontFamily: MONO,
            fontSize: 11,
            fontWeight: 700,
            padding: '2px 5px',
            borderRadius: 3,
            background: C.bg,
            minWidth: 26,
            textAlign: 'center',
            color: GLYPH[t.shell].c,
          }}
        >
          {GLYPH[t.shell].g}
        </span>
        <span style={{ flex: 1, fontSize: 14, fontWeight: 600 }}>{t.name}</span>
        <span style={{ color: C.dim }}>×</span>
      </div>
    ))}
  </div>
);

/** The sidebar of a project: the switcher, the agents, then what follows them (launch, terminals, foot). */
export const AgentsSidebar: FC<{
  agents: AgentInfo[];
  selected?: string | null;
  enters?: number[];
  ring?: string;
  alerts?: Record<string, string>;
  board?: boolean;
  badge?: number;
  archived?: number;
  names?: Record<string, ReactNode>;
  pressedNew?: number;
  after?: ReactNode;
  foot?: boolean | { path?: string; branch?: string; counts?: [number, number, number] };
}> = ({ agents, selected, enters, ring, alerts, board = false, badge, archived, names, pressedNew, after, foot = true }) => (
  <>
    <Switcher board={board} badge={badge} />
    <SectionHead label="Agents" count={String(agents.length)} action={<Button pressed={pressedNew}>+ Nouvel agent</Button>} />
    <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 4, overflow: 'hidden' }}>
      {agents.map((a, i) => (
        <AgentCard
          key={a.name + i}
          {...a}
          selected={a.name === selected}
          enter={enters?.[i] ?? 1}
          ring={a.name === ring}
          alert={alerts?.[a.name]}
          nameOverride={names?.[a.name]}
        />
      ))}
      {archived ? <span style={{ padding: '8px 10px', fontSize: 13, color: C.dim }}>▸ Archivés ({archived})</span> : null}
    </div>
    {after}
    {foot ? <SidebarFoot {...(foot === true ? {} : foot)} /> : null}
  </>
);

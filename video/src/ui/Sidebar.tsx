import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { useFmt, type Tr } from '../lang';
import { C, MONO, soft } from '../theme';
import { Icon, type IconName } from './icons';
import { blink, Dot, Kbd } from './Shell';

export type AgentStatus = 'running' | 'waiting' | 'idle' | 'done' | 'error';
export type Harness = 'Claude' | 'Codex' | 'Cursor' | 'OpenCode' | 'Copilot';

export interface AgentInfo {
  name: string;
  status: AgentStatus;
  model: string;
  time: string;
  tokens: string;
  cost: string;
  files: number;
  /** Its harness, Claude Code by default. */
  harness?: Harness;
  /** The temperature of its prompt cache, in °C (0 to 100): hot, its next turn costs less. */
  heat?: number;
  /** What it does now: « Modifie src/auth.ts ». */
  activity?: string;
  /** Its worktree's branch. */
  branch?: string;
  /** « DEM-4 · boucle 2/5 ». */
  ticket?: string;
  remote?: boolean;
  /** « Reprise à 14:05 »: stopped by the usage limit. */
  resume?: string;
}

const labelOf = (tr: Tr): Record<AgentStatus, string> => ({
  running: tr('En cours', 'Running'),
  waiting: 'Question',
  idle: tr('Prêt', 'Ready'),
  done: tr('Terminé', 'Done'),
  error: tr('Erreur', 'Error'),
});
export const STATUS_COLOR: Record<AgentStatus, string> = { running: C.ok, waiting: C.wait, idle: C.dim, done: C.muted, error: C.del };

/** The colour of a temperature, as the app's (src/lib/heat.svelte.ts). */
export const heatColor = (deg: number) => (deg >= 67 ? C.heatHot : deg >= 34 ? C.heatMid : C.heatCool);

export const SectionHead: FC<{ label: string; count?: string; action?: ReactNode }> = ({ label, count, action }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 8px' }}>
    <span style={{ fontSize: 13, fontWeight: 600, color: C.soft }}>{label}</span>
    {count ? <span style={{ fontSize: 12.5, color: C.muted, fontVariantNumeric: 'tabular-nums' }}>{count}</span> : null}
    <div style={{ flex: 1 }} />
    {action}
  </div>
);

/** A button of the app: outlined, or `primary` (the project's accent gives the white primary, another colour fills it). */
export const Button: FC<{ children: ReactNode; pressed?: number; accent?: boolean; primary?: string; small?: boolean }> = ({
  children,
  pressed = 0,
  accent,
  primary,
  small,
}) => {
  const fill = primary === C.spark ? C.primary : primary;
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 6,
        height: small ? 26 : 30,
        padding: small ? '0 9px' : '0 12px',
        borderRadius: 7,
        boxShadow: fill ? 'none' : `inset 0 0 0 1px ${accent ? C.pInk : C.line2}`,
        background: fill ? fill : pressed ? soft(C.spark, 30 * pressed) : 'rgba(255, 255, 255, 0.03)',
        color: fill ? C.onPrimary : C.text,
        fontSize: small ? 12.5 : 13.5,
        fontWeight: fill ? 600 : 500,
        whiteSpace: 'nowrap',
        transform: `scale(${1 - 0.06 * pressed})`,
        filter: fill && pressed ? `brightness(${1 - 0.15 * pressed})` : undefined,
        flex: 'none',
      }}
    >
      {children}
    </span>
  );
};

/** The sidebar's views, « Agents | Kanban | PR | Docs »; `badge`: tickets to test. */
export const Switcher: FC<{ board: boolean; badge?: number; count?: number }> = ({ board, badge, count }) => {
  const tabs: { id: string; icon: IconName; label: string; on: boolean; n?: number; badge?: number }[] = [
    { id: 'agents', icon: 'chat', label: 'Agents', on: !board, n: count },
    { id: 'kanban', icon: 'kanban', label: 'Kanban', on: board, badge },
    { id: 'pr', icon: 'pr', label: 'PR', on: false },
    { id: 'docs', icon: 'doc', label: 'Docs', on: false },
  ];
  return (
    <div style={{ display: 'flex', gap: 2, padding: 3, borderRadius: 9, boxShadow: `inset 0 0 0 1px ${C.line}`, flex: 'none' }}>
      {tabs.map((t) => (
        <span
          key={t.id}
          style={{
            flex: t.id === 'agents' || t.id === 'kanban' ? 1.3 : 0.8,
            height: 28,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 5,
            borderRadius: 6,
            background: t.on ? 'rgba(255, 255, 255, 0.06)' : 'transparent',
            boxShadow: t.on ? `inset 0 0 0 1px ${C.line2}` : 'none',
            color: t.on ? C.text : C.muted,
            fontSize: 12.5,
            fontWeight: 550,
            whiteSpace: 'nowrap',
          }}
        >
          <Icon name={t.icon} size={13} />
          {t.label}
          {t.n ? <span style={{ color: C.muted, fontWeight: 500 }}>{t.n}</span> : null}
          {t.badge ? (
            <span
              style={{
                minWidth: 16,
                height: 16,
                padding: '0 4px',
                borderRadius: 8,
                background: C.wait,
                color: C.onWait,
                fontSize: 10.5,
                fontWeight: 700,
                display: 'inline-flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              {t.badge}
            </span>
          ) : null}
        </span>
      ))}
    </div>
  );
};

const Wifi: FC<{ on: boolean }> = ({ on }) => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke={on ? C.info : C.dim} strokeWidth="2.2" strokeLinecap="round">
    <path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0" />
    <circle cx="12" cy="19.5" r="1" fill={on ? C.info : C.dim} />
  </svg>
);

/** The harness's name, boxed, as the cards and the headers show it. */
export const HarnessChip: FC<{ name: string }> = ({ name }) => (
  <span
    style={{
      flex: 'none',
      padding: '0 5px',
      height: 17,
      display: 'inline-flex',
      alignItems: 'center',
      borderRadius: 4,
      boxShadow: `inset 0 0 0 1px ${C.line2}`,
      color: C.soft,
      fontSize: 11,
      fontWeight: 600,
    }}
  >
    {name}
  </span>
);

const Sep = () => <span style={{ color: '#4a4e55' }}>·</span>;

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
  harness = 'Claude',
  heat,
  activity,
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
  const { tr, trx, plural } = useFmt();
  const a = alert ? blink(frame) : 0;
  if (enter <= 0) return null;
  const asks = status === 'waiting';
  const color = STATUS_COLOR[status];
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 5,
        padding: '9px 10px',
        borderRadius: 9,
        boxShadow: `inset 0 0 0 1px ${alert ? alert : asks ? soft(C.wait, 32) : selected ? C.line2 : 'transparent'}${
          ring ? `, 0 0 0 2px ${soft(C.wait, 70)}, 0 0 28px ${soft(C.wait, 30)}` : ''
        }`,
        background: alert ? soft(alert, 16 * a) : asks ? soft(C.wait, 7) : selected ? 'rgba(255, 255, 255, 0.045)' : 'transparent',
        opacity: Math.min(1, enter),
        transform: `translateX(${(1 - Math.min(1, enter)) * -40}px)`,
        flex: 'none',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <Dot color={color} size={7} pulse={status === 'running' || asks} />
        <span
          style={{
            flex: 1,
            fontSize: 14,
            fontWeight: 600,
            letterSpacing: '-0.01em',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
        >
          {nameOverride ?? name}
        </span>
        {remote !== undefined ? <Wifi on={remote} /> : null}
        {resume ? (
          <span style={{ fontSize: 12, color: C.wait }}>{resume}</span>
        ) : (
          <span style={{ fontSize: 12, fontWeight: asks ? 600 : 500, color: color === C.dim ? C.muted : color }}>
            {labelOf(tr)[status]}
          </span>
        )}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 6, paddingLeft: 15, fontSize: 12, color: C.muted, whiteSpace: 'nowrap' }}>
        <HarnessChip name={harness} />
        {ticket ? (
          <span style={{ color: C.pInk, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis' }}>{ticket}</span>
        ) : (
          <span>{model}</span>
        )}
        <Sep />
        <span>{time}</span>
        {branch && !ticket ? (
          <>
            <Sep />
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, minWidth: 0, overflow: 'hidden' }}>
              <Icon name="branch" size={12} />
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>{branch}</span>
            </span>
          </>
        ) : null}
        {heat !== undefined ? (
          <span style={{ marginLeft: 'auto', paddingLeft: 6, fontFamily: MONO, fontSize: 12, color: heatColor(heat) }}>{heat} °C</span>
        ) : null}
      </div>
      <div style={{ display: 'flex', gap: 6, paddingLeft: 15, fontSize: 12, color: C.muted, whiteSpace: 'nowrap' }}>
        {activity ? (
          <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', color: C.soft }}>{activity}</span>
        ) : (
          <span style={{ flex: 1 }}>{trx(<>{files} fich.</>, `${files} ${plural(files, '', '', 'file', 'files')}`)}</span>
        )}
        <span style={{ fontVariantNumeric: 'tabular-nums' }}>{tokens}</span>
        <Sep />
        <span style={{ fontVariantNumeric: 'tabular-nums' }}>{cost}</span>
      </div>
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
      borderTop: `1px solid ${C.line}`,
      padding: '10px 8px 12px',
      display: 'flex',
      flexDirection: 'column',
      gap: 7,
      fontFamily: MONO,
      fontSize: 12,
      color: C.soft,
    }}
  >
    <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <Icon name="doc" size={13} color={C.muted} />
      {path}
    </span>
    <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
      <Icon name="branch" size={13} color={C.muted} />
      {branch}
      <span style={{ flex: 1 }} />
      <span style={{ color: C.wait }}>~{counts[0]}</span>
      <span style={{ color: C.ok }}>+{counts[1]}</span>
      <span style={{ color: C.del }}>−{counts[2]}</span>
    </span>
  </div>
);

export interface Term {
  name: string;
  shell: 'pwsh' | 'bash' | 'wsl';
}
const GLYPH = { pwsh: { g: 'PS', c: C.info }, bash: { g: '$_', c: C.ok }, wsl: { g: 'λ', c: 'oklch(0.78 0.13 60)' } };

export const TermsSection: FC<{ terms: Term[]; selected?: number }> = ({ terms, selected }) => {
  const { tr } = useFmt();
  return (
    <div style={{ flex: 'none', borderTop: `1px solid ${C.line}`, padding: '4px 0 6px' }}>
      <SectionHead
        label={tr('Terminaux', 'Terminals')}
        count={String(terms.length)}
        action={
          <span style={{ color: C.muted }}>
            <Icon name="plus" size={14} />
          </span>
        }
      />
      {terms.map((t, i) => (
        <div
          key={t.name}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            height: 30,
            padding: '0 8px',
            borderRadius: 7,
            background: i === selected ? 'rgba(255, 255, 255, 0.045)' : 'transparent',
            boxShadow: i === selected ? `inset 0 0 0 1px ${C.line2}` : 'none',
          }}
        >
          <span
            style={{
              fontFamily: MONO,
              fontSize: 10.5,
              fontWeight: 600,
              padding: '1px 4px',
              borderRadius: 4,
              minWidth: 24,
              textAlign: 'center',
              color: GLYPH[t.shell].c,
              boxShadow: `inset 0 0 0 1px ${C.line2}`,
            }}
          >
            {GLYPH[t.shell].g}
          </span>
          <span style={{ flex: 1, fontSize: 13, fontWeight: 500 }}>{t.name}</span>
          <span style={{ color: C.dim }}>×</span>
        </div>
      ))}
    </div>
  );
};

/** « + Nouvel agent  Ctrl N », at the top of the agents. */
const NewAgent: FC<{ pressed?: number }> = ({ pressed = 0 }) => {
  const { tr } = useFmt();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 9,
        height: 32,
        padding: '0 8px',
        borderRadius: 8,
        background: pressed ? soft(C.spark, 25 * pressed) : 'transparent',
        transform: `scale(${1 - 0.04 * pressed})`,
        color: C.text,
        fontSize: 13.5,
        fontWeight: 500,
        flex: 'none',
      }}
    >
      <span
        style={{
          width: 20,
          height: 20,
          display: 'grid',
          placeItems: 'center',
          borderRadius: 6,
          boxShadow: `inset 0 0 0 1px ${C.line2}`,
          color: C.soft,
        }}
      >
        <Icon name="plus" size={12} />
      </span>
      <span style={{ flex: 1 }}>{tr('Nouvel agent', 'New agent')}</span>
      <Kbd>Ctrl N</Kbd>
      <span style={{ color: C.muted }}>
        <Icon name="chevron" size={13} />
      </span>
    </div>
  );
};

/** The sidebar of a project: its views, the new agent, the agents, then what follows them (launch, terminals, foot). */
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
}> = ({ agents, selected, enters, ring, alerts, board = false, badge, archived, names, pressedNew, after, foot = true }) => {
  const { trx } = useFmt();
  return (
    <>
      <Switcher board={board} badge={badge} count={agents.length} />
      <NewAgent pressed={pressedNew} />
      <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 2, overflow: 'hidden' }}>
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
        {archived ? (
          <span style={{ padding: '8px 10px', fontSize: 12.5, color: C.muted }}>
            {trx(<>▸ Archivés ({archived})</>, `▸ Archived (${archived})`)}
          </span>
        ) : null}
      </div>
      {after}
      {foot ? <SidebarFoot {...(foot === true ? {} : foot)} /> : null}
    </>
  );
};

import type { FC, ReactNode } from 'react';
import { C, MONO, soft } from '../theme';
import { Button } from './Sidebar';

export interface ChangedFile {
  status: 'M' | 'A' | 'D';
  path: string;
  add: number;
  del: number;
  /** The agent whose worktree has it (« Tout le projet »). */
  agent?: string;
}

const STATUS_COLOR = { M: C.wait, A: C.ok, D: C.del };

const Segmented: FC<{ items: string[]; on: number }> = ({ items, on }) => (
  <div style={{ display: 'flex', gap: 2, padding: 2, borderRadius: 7, background: C.bg, border: `1px solid ${C.line}` }}>
    {items.map((it, i) => (
      <span
        key={it}
        style={{
          flex: 1,
          height: 28,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 5,
          background: i === on ? C.elev2 : 'transparent',
          color: i === on ? C.text : C.muted,
          fontSize: 13,
          fontWeight: 600,
        }}
      >
        {it}
      </span>
    ))}
  </div>
);

/** The side panel: « Non commités » (scope, files, diff, commit, merge) or « Historique ». */
export const SidePanel: FC<{
  tab: 'files' | 'history';
  scope?: 'agent' | 'project';
  files: ChangedFile[];
  selected?: number;
  hint?: string;
  merge?: string;
  pressedMerge?: number;
  children?: ReactNode;
}> = ({ tab, scope = 'agent', files, selected = 0, hint, merge, pressedMerge, children }) => (
  <div style={{ height: '100%', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
    <div style={{ display: 'flex', gap: 20, padding: '14px 18px 0', borderBottom: `1px solid ${C.line}`, fontSize: 14, fontWeight: 600 }}>
      {(['files', 'history'] as const).map((t) => (
        <span
          key={t}
          style={{ color: tab === t ? C.text : C.dim, borderBottom: `2px solid ${tab === t ? C.spark : 'transparent'}`, paddingBottom: 10 }}
        >
          {t === 'files' ? (
            <>
              Non commités <span style={{ fontFamily: MONO, color: C.dim }}>{files.length}</span>
            </>
          ) : (
            'Historique'
          )}
        </span>
      ))}
    </div>
    {tab === 'files' ? (
      <>
        <div style={{ padding: '12px 14px 4px' }}>
          <Segmented items={['Cet agent', 'Tout le projet']} on={scope === 'agent' ? 0 : 1} />
        </div>
        {hint ? <div style={{ padding: '4px 16px 6px', fontFamily: MONO, fontSize: 11.5, color: C.dim }}>{hint}</div> : null}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2, padding: '0 8px' }}>
          {files.map((f, i) => (
            <div
              key={f.path}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 9,
                padding: '6px 10px',
                borderRadius: 6,
                background: i === selected ? C.elev : 'transparent',
                fontSize: 13,
              }}
            >
              <span style={{ fontFamily: MONO, fontWeight: 700, color: STATUS_COLOR[f.status], width: 12 }}>{f.status}</span>
              <span style={{ fontFamily: MONO, fontWeight: 600 }}>{f.path.split('/').pop()}</span>
              <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim, flex: 1, overflow: 'hidden', whiteSpace: 'nowrap' }}>
                {f.path.split('/').slice(0, -1).join('/')}
              </span>
              {f.agent ? (
                <span style={{ fontFamily: MONO, fontSize: 11, padding: '1px 6px', borderRadius: 3, background: C.elev2, color: C.spark }}>
                  {f.agent}
                </span>
              ) : null}
              <span style={{ fontFamily: MONO, fontSize: 12, color: C.ok }}>+{f.add}</span>
              <span style={{ fontFamily: MONO, fontSize: 12, color: C.del }}>−{f.del}</span>
            </div>
          ))}
        </div>
        <div style={{ flex: 1, minHeight: 0, overflow: 'hidden', padding: '8px 12px' }}>{children}</div>
        <div style={{ display: 'flex', gap: 8, padding: '10px 12px', borderTop: `1px solid ${C.line}` }}>
          <span style={{ flex: 1, display: 'flex' }}>
            <Button>Voir le diff</Button>
          </span>
          <Button primary={C.spark}>{scope === 'agent' ? 'Commit…' : 'Commit tout…'}</Button>
        </div>
        {merge && scope === 'agent' ? (
          <div style={{ padding: '0 12px 12px', display: 'flex' }}>
            <span style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
              <Button pressed={pressedMerge}>{merge}</Button>
            </span>
          </div>
        ) : null}
      </>
    ) : (
      <div style={{ flex: 1, minHeight: 0, overflow: 'hidden' }}>{children}</div>
    )}
  </div>
);

export interface DiffRow {
  kind: 'ctx' | 'chg' | 'add';
  l?: string;
  r?: string;
}

/** A side-by-side diff; `shown` rows are visible. */
export const DiffPane: FC<{ rows: DiffRow[]; shown: number; start: number }> = ({ rows, shown, start }) => (
  <div style={{ borderRadius: 8, border: `1px solid ${C.line}`, overflow: 'hidden', fontFamily: MONO, fontSize: 12.5, background: C.term }}>
    {rows.slice(0, shown).map((row, i) => (
      <div key={i} style={{ display: 'flex', minHeight: 25, alignItems: 'center' }}>
        {(['l', 'r'] as const).map((side) => {
          const text = row[side];
          const bg =
            row.kind === 'ctx' || text === undefined
              ? 'transparent'
              : side === 'l'
                ? soft(C.del, row.kind === 'chg' ? 16 : 0)
                : soft(C.ok, 16);
          return (
            <div
              key={side}
              style={{
                flex: 1,
                minWidth: 0,
                overflow: 'hidden',
                display: 'flex',
                alignSelf: 'stretch',
                alignItems: 'center',
                background: bg,
                borderLeft: side === 'r' ? `1px solid ${C.line}` : 'none',
              }}
            >
              <span style={{ width: 32, flex: 'none', textAlign: 'right', paddingRight: 8, color: C.dim }}>
                {text === undefined ? '' : start + i}
              </span>
              <span style={{ whiteSpace: 'pre', color: C.text }}>{text ?? ''}</span>
            </div>
          );
        })}
      </div>
    ))}
  </div>
);

export interface Commit {
  lane: number;
  msg: string;
  hash: string;
  refs?: string[];
}

/** Newest first. Lane 0: main; lane 1: the agent's branch; lane 2: another agent. */
export const COMMITS: Commit[] = [
  // The squash merge: one commit on main, named after the agent (core.rs merge_agent), no link to its branch.
  { lane: 0, msg: 'pagination-users', hash: '4e1b2c9', refs: ['main'] },
  { lane: 1, msg: 'Pagination de /users (limite, curseur)', hash: 'a3f9c21', refs: ['pagination-users'] },
  { lane: 1, msg: 'Tests de la pagination', hash: '7be01d4' },
  { lane: 0, msg: 'Corrige le login OAuth', hash: '19ce8a0', refs: ['origin/main'] },
  { lane: 2, msg: 'Docs : endpoints v2', hash: 'c04d7f2', refs: ['docs-api'] },
  { lane: 1, msg: 'Prépare le modèle User', hash: '5d2e9ab' },
  { lane: 0, msg: 'Release 1.4.0', hash: 'e8a1f30', refs: ['v1.4.0'] },
  { lane: 0, msg: 'Ajoute le rate limiting global', hash: '2c71e5a' },
];

const LANE = [C.muted, C.spark, 'oklch(0.72 0.12 200)'];

/** The git graph; rows before `from` (the squashed merge) are hidden, `shown` rows appear from there. */
export const GitGraph: FC<{ commits: Commit[]; from: number; shown: number; highlight: number }> = ({
  commits,
  from,
  shown,
  highlight,
}) => {
  const rows = commits.slice(from, from + shown);
  const y = (i: number) => 24 + i * 48;
  const x = (lane: number) => 26 + lane * 28;
  const last = commits.length - 1 - from;
  const lanes = [0, 1, 2].map((lane) => rows.map((c, i) => (c.lane === lane ? i : -1)).filter((i) => i >= 0));
  return (
    <div style={{ position: 'relative', margin: '8px 14px' }}>
      <svg width="110" height={y(Math.max(rows.length, 1))} style={{ position: 'absolute', left: 0, top: 0 }}>
        {lanes.map((idx, lane) => {
          if (!idx.length) return null;
          const top = lane === 0 ? 0 : idx[0];
          const bottom = lane === 0 ? rows.length - 1 : Math.min(idx[idx.length - 1] + 1, last);
          const color = lane === highlight ? C.spark : LANE[lane];
          const width = lane === highlight ? 4 : 2.5;
          return (
            <g key={lane} stroke={color} strokeWidth={width} fill="none" opacity={lane === highlight || lane === 0 ? 1 : 0.5}>
              <line x1={x(lane)} y1={y(top)} x2={x(lane)} y2={y(Math.max(top, bottom - (lane ? 1 : 0)))} />
              {lane && bottom <= rows.length - 1 ? (
                <path d={`M${x(lane)} ${y(bottom - 1)} C ${x(lane)} ${y(bottom) - 10}, ${x(0)} ${y(bottom) - 30}, ${x(0)} ${y(bottom)}`} />
              ) : null}
            </g>
          );
        })}
        {rows.map((c, i) => (
          <circle
            key={c.hash}
            cx={x(c.lane)}
            cy={y(i)}
            r={c.lane === highlight ? 8 : 6.5}
            fill={C.bg}
            stroke={c.lane === highlight ? C.spark : LANE[c.lane]}
            strokeWidth={3}
          />
        ))}
      </svg>
      <div style={{ paddingLeft: 100 }}>
        {rows.map((c) => (
          <div
            key={c.hash}
            style={{ height: 48, display: 'flex', alignItems: 'center', gap: 9, fontSize: 14, whiteSpace: 'nowrap', overflow: 'hidden' }}
          >
            {c.refs?.map((r) => (
              <span
                key={r}
                style={{
                  fontFamily: MONO,
                  fontSize: 12,
                  padding: '2px 7px',
                  borderRadius: 4,
                  background: r === 'pagination-users' ? soft(C.spark, 25) : C.elev2,
                  color: r === 'pagination-users' ? C.spark : C.muted,
                }}
              >
                {r}
              </span>
            ))}
            <span
              style={{
                minWidth: 0,
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                color: c.lane === highlight ? C.text : C.muted,
                fontWeight: c.lane === highlight ? 600 : 400,
                opacity: c.lane === highlight || c.lane === 0 ? 1 : 0.6,
              }}
            >
              {c.msg}
            </span>
            <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>{c.hash}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

/** The status bar's sync menu (Pull, Push, Fetch), opening upwards from (x, y). */
export const SyncMenu: FC<{ x: number; y: number; behind: number; ahead: number; hover?: number; enter: number }> = ({
  x,
  y,
  behind,
  ahead,
  hover,
  enter,
}) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        left: x,
        bottom: 844 - y,
        minWidth: 250,
        padding: 5,
        borderRadius: 10,
        background: C.elev,
        border: `1px solid ${C.line2}`,
        boxShadow: '0 18px 50px rgba(0, 0, 0, 0.55)',
        zIndex: 45,
        opacity: Math.min(1, enter),
      }}
    >
      {[
        ['Pull', `↓${behind}`],
        ['Push', `↑${ahead}`],
        ['Fetch', 'maintenant'],
      ].map(([l, h], i) => (
        <div
          key={l}
          style={{
            borderTop: i === 2 ? `1px solid ${C.line}` : undefined,
            marginTop: i === 2 ? 4 : 0,
            display: 'flex',
            gap: 14,
            padding: '8px 12px',
            borderRadius: 6,
            background: i === hover ? C.elev2 : 'transparent',
            fontSize: 14.5,
          }}
        >
          {l}
          <span style={{ flex: 1 }} />
          <span style={{ fontSize: 12.5, color: C.dim }}>{h}</span>
        </div>
      ))}
    </div>
  );

/** The full-screen diff view (« Voir le diff », or a commit of the history): its files, then the selected one's diff. */
export const FullDiff: FC<{ title: string; files: ChangedFile[]; selected?: number; children: ReactNode; enter: number }> = ({
  title,
  files,
  selected = 0,
  children,
  enter,
}) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        background: `rgba(10, 9, 8, ${0.55 * Math.min(1, enter)})`,
        zIndex: 40,
        display: 'flex',
        padding: 28,
      }}
    >
      <div
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          borderRadius: 14,
          background: C.panel,
          border: `1px solid ${C.line2}`,
          boxShadow: '0 30px 90px rgba(0, 0, 0, 0.6)',
          opacity: Math.min(1, enter),
          transform: `scale(${0.97 + 0.03 * Math.min(1, enter)})`,
          overflow: 'hidden',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '14px 18px', borderBottom: `1px solid ${C.line}` }}>
          <span style={{ fontSize: 17, fontWeight: 700 }}>{title}</span>
          <span style={{ fontFamily: MONO, fontSize: 12.5, color: C.dim }}>
            {files.length} fichier{files.length > 1 ? 's' : ''}
          </span>
          <div style={{ flex: 1 }} />
          <span style={{ width: 210 }}>
            <Segmented items={['Unifié', 'Côte à côte']} on={0} />
          </span>
          <span style={{ color: C.dim, fontSize: 18 }}>×</span>
        </div>
        <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
          <div
            style={{
              width: 300,
              flex: 'none',
              borderRight: `1px solid ${C.line}`,
              padding: 8,
              display: 'flex',
              flexDirection: 'column',
              gap: 2,
            }}
          >
            {files.map((f, i) => (
              <span
                key={f.path}
                style={{
                  display: 'flex',
                  gap: 8,
                  padding: '6px 10px',
                  borderRadius: 6,
                  background: i === selected ? C.elev : 'transparent',
                  fontFamily: MONO,
                  fontSize: 12.5,
                }}
              >
                <span style={{ color: STATUS_COLOR[f.status], fontWeight: 700 }}>{f.status}</span>
                <span style={{ flex: 1, overflow: 'hidden', whiteSpace: 'nowrap', textOverflow: 'ellipsis' }}>{f.path}</span>
                <span style={{ color: C.ok }}>+{f.add}</span>
                <span style={{ color: C.del }}>−{f.del}</span>
              </span>
            ))}
          </div>
          <div style={{ flex: 1, minWidth: 0, padding: '14px 18px', display: 'flex', flexDirection: 'column', gap: 10 }}>
            <span style={{ fontFamily: MONO, fontSize: 13.5 }}>
              {files[selected]?.path} <span style={{ color: C.ok }}>+{files[selected]?.add}</span>{' '}
              <span style={{ color: C.del }}>−{files[selected]?.del}</span>
            </span>
            <div style={{ borderRadius: 8, border: `1px solid ${C.line}`, overflow: 'hidden' }}>{children}</div>
          </div>
        </div>
      </div>
    </div>
  );

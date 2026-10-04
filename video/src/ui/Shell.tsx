import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { fr } from '../anim';
import { C, hue, MONO, soft, UI } from '../theme';
import { Logo } from './Logo';

export interface Tab {
  name: string;
  hue: number;
  delta?: number;
  waiting?: number;
  running?: boolean;
  /** Blinks in this colour: asked, finished or failed out of sight. */
  alert?: string;
  /** Appearance, 0 to 1. */
  enter?: number;
}

export interface Status {
  active: number;
  waiting: number;
  done: number;
  /** Quotas, in %, and when they reset. */
  session: number;
  weekly: number;
  sessionReset?: string;
  weeklyReset?: string;
  cost: number;
  estimated?: boolean;
  procs?: { n: number; mem: string; cpu: string };
  sync?: { branch: string; behind: number; ahead: number; busy?: string };
  /** A newer version, offered at the right. */
  update?: string;
}

export const Dot: FC<{ color: string; size?: number; pulse?: boolean }> = ({ color, size = 8, pulse }) => {
  const frame = useCurrentFrame();
  const k = pulse ? (frame % 30) / 30 : 0;
  return (
    <span
      style={{
        display: 'inline-block',
        flex: 'none',
        width: size,
        height: size,
        borderRadius: '50%',
        background: color,
        boxShadow: pulse ? `0 0 0 ${k * 7}px ${soft(color, (1 - k) * 45)}` : 'none',
      }}
    />
  );
};

/** 0 to 1 and back, about once a second: the app's blinking. */
export const blink = (frame: number) => 0.5 - 0.5 * Math.cos((frame / 33) * Math.PI);

const Pill: FC<{ n: number }> = ({ n }) => {
  const frame = useCurrentFrame();
  return (
    <span
      style={{
        fontSize: 12,
        fontWeight: 800,
        color: '#1b1512',
        background: C.wait,
        borderRadius: 9,
        padding: '0 7px',
        boxShadow: `0 0 0 ${3 + 3 * Math.sin(frame / 4)}px ${soft(C.wait, 30)}`,
      }}
    >
      {n}
    </span>
  );
};

const TitleBar: FC<{ tabs: Tab[]; active: number; view: 'project' | 'stats' | 'editor' }> = ({ tabs, active, view }) => {
  const frame = useCurrentFrame();
  return (
    <header
      style={{
        height: 48,
        flex: 'none',
        display: 'flex',
        alignItems: 'flex-end',
        gap: 4,
        padding: '0 0 0 14px',
        background: C.bar,
        borderBottom: `1px solid ${C.line}`,
      }}
    >
      <div style={{ alignSelf: 'center', paddingRight: 12, display: 'flex' }}>
        <Logo size={26} />
      </div>
      {tabs.map((t, i) => {
        const e = t.enter ?? 1;
        if (e <= 0) return null;
        const on = i === active && view !== 'stats';
        const a = t.alert && !on ? blink(frame) : 0;
        return (
          <div
            key={t.name}
            style={{
              height: 38,
              display: 'flex',
              alignItems: 'center',
              gap: 9,
              padding: '0 15px',
              borderRadius: '9px 9px 0 0',
              background: on ? C.bg : a ? soft(t.alert!, 22 * a) : 'transparent',
              border: `1px solid ${on ? C.line2 : t.alert ? t.alert : 'transparent'}`,
              borderBottom: 'none',
              fontSize: 15,
              fontWeight: 600,
              color: on ? C.text : C.muted,
              opacity: Math.min(1, e),
              transform: `translateY(${(1 - e) * 20}px)`,
            }}
          >
            <span style={{ width: 9, height: 9, borderRadius: 3, background: hue(t.hue) }} />
            {t.name}
            {t.running && !t.waiting ? <Dot color={C.ok} size={6} /> : null}
            {t.delta ? (
              <span style={{ fontFamily: MONO, fontSize: 12, color: C.muted, background: C.elev, padding: '1px 6px', borderRadius: 4 }}>
                Δ {t.delta}
              </span>
            ) : null}
            {t.waiting ? <Pill n={t.waiting} /> : null}
          </div>
        );
      })}
      <div style={{ width: 30, height: 38, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.dim, fontSize: 20 }}>
        +
      </div>
      <div style={{ flex: 1 }} />
      <div style={{ alignSelf: 'stretch', display: 'flex', alignItems: 'flex-end', gap: 2, fontSize: 14, fontWeight: 600 }}>
        {[
          { id: 'editor', label: 'Éditeur', icon: <span style={{ fontFamily: MONO, fontSize: 12, color: C.spark }}>{'</>'}</span> },
          {
            id: 'stats',
            label: 'Stats',
            icon: (
              <span style={{ display: 'inline-flex', alignItems: 'flex-end', gap: 2, height: 12 }}>
                {[6, 12, 9].map((h, i) => (
                  <span key={i} style={{ width: 3, height: h, background: C.spark, borderRadius: 1 }} />
                ))}
              </span>
            ),
          },
        ].map((b) => (
          <div
            key={b.id}
            style={{
              height: 38,
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '0 14px',
              borderRadius: '9px 9px 0 0',
              background: b.id === 'stats' && view === b.id ? C.bg : 'transparent',
              border: `1px solid ${b.id === 'stats' && view === b.id ? C.line2 : 'transparent'}`,
              borderBottom: 'none',
              color: b.id === 'stats' && view === b.id ? C.text : C.muted,
            }}
          >
            {b.icon}
            {b.label}
          </div>
        ))}
      </div>
      <div style={{ alignSelf: 'stretch', display: 'flex', marginLeft: 10 }}>
        {['—', '☐', '✕'].map((s) => (
          <span
            key={s}
            style={{ width: 52, display: 'flex', alignItems: 'center', justifyContent: 'center', color: C.muted, fontSize: 14 }}
          >
            {s}
          </span>
        ))}
      </div>
    </header>
  );
};

const Quota: FC<{ label: string; pct: number; reset?: string }> = ({ label, pct, reset }) => (
  <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
    {label}
    <span style={{ width: 56, height: 6, borderRadius: 3, background: C.elev2, overflow: 'hidden' }}>
      <span style={{ display: 'block', width: `${pct}%`, height: '100%', background: pct > 80 ? C.wait : C.spark }} />
    </span>
    <b style={{ color: C.text }}>{Math.round(pct)} %</b>
    {reset ? <span style={{ color: C.dim }}>reset {reset}</span> : null}
  </span>
);

const Sep = () => <span style={{ width: 1, height: 14, background: C.line2 }} />;

export const StatusBar: FC<Status> = (s) => (
  <footer
    style={{
      height: 34,
      flex: 'none',
      display: 'flex',
      alignItems: 'center',
      gap: 13,
      padding: '0 10px 0 16px',
      borderTop: `1px solid ${C.line}`,
      background: C.bar,
      fontFamily: MONO,
      fontSize: 12,
      color: C.muted,
      whiteSpace: 'nowrap',
    }}
  >
    <span style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
      <Dot color={C.ok} size={7} /> {s.active} actif{s.active > 1 ? 's' : ''}
    </span>
    <span style={{ display: 'flex', alignItems: 'center', gap: 7, color: s.waiting ? C.wait : C.muted }}>
      <Dot color={s.waiting ? C.wait : C.dim} size={7} pulse={s.waiting > 0} /> {s.waiting} en attente
    </span>
    <span>
      <span style={{ color: C.ok }}>✓</span> {s.done} terminé{s.done > 1 ? 's' : ''}
    </span>
    {s.procs ? (
      <>
        <Sep />
        <span>
          {s.procs.n} Claude · <b style={{ color: C.text, fontWeight: 500 }}>{s.procs.mem}</b> ·{' '}
          <b style={{ color: C.text, fontWeight: 500 }}>{s.procs.cpu}</b> CPU
        </span>
      </>
    ) : null}
    <Sep />
    <Quota label="Session 5 h" pct={s.session} reset={s.sessionReset} />
    <Quota label="Hebdo" pct={s.weekly} reset={s.weeklyReset} />
    <Sep />
    <span>
      Aujourd'hui{' '}
      <b style={{ color: C.text }}>
        {s.estimated ? '≈ ' : ''}
        {fr(s.cost)} $
      </b>
    </span>
    {s.sync ? (
      <>
        <Sep />
        <span style={{ display: 'flex', gap: 8 }}>
          <b style={{ color: C.text, fontWeight: 500 }}>⎇ {s.sync.branch}</b>
          {s.sync.busy ? (
            <span>{s.sync.busy}…</span>
          ) : (
            <>
              <span style={{ color: s.sync.behind ? C.wait : C.dim }}>↓{s.sync.behind}</span>
              <span style={{ color: s.sync.ahead ? C.text : C.dim }}>↑{s.sync.ahead}</span>
            </>
          )}
        </span>
      </>
    ) : null}
    <div style={{ flex: 1 }} />
    {s.update ? (
      <span
        style={{
          padding: '3px 10px',
          borderRadius: 6,
          background: soft(C.ok, 18),
          color: C.ok,
          fontFamily: UI,
          fontSize: 12.5,
          fontWeight: 700,
        }}
      >
        Mise à jour {s.update} disponible → installer
      </span>
    ) : null}
    <span style={{ padding: '2px 8px', border: `1px solid ${C.line2}`, borderRadius: 5 }}>♪ On</span>
    <span style={{ padding: '2px 8px', border: `1px solid ${C.line2}`, borderRadius: 5 }}>⚙</span>
  </footer>
);

/** The app: tabs, sidebar, main area, status bar; `overlay` covers it all (modals, menus, toasts). */
export const Shell: FC<{
  tabs: Tab[];
  active?: number;
  view?: 'project' | 'stats' | 'editor';
  sidebar?: ReactNode;
  children?: ReactNode;
  status: Status;
  overlay?: ReactNode;
}> = ({ tabs, active = 0, view = 'project', sidebar, children, status, overlay }) => (
  <div
    style={{
      position: 'relative',
      width: '100%',
      height: '100%',
      borderRadius: 14,
      overflow: 'hidden',
      background: C.bg,
      border: `1px solid ${C.line2}`,
      boxShadow: '0 40px 120px rgba(0, 0, 0, 0.6)',
      display: 'flex',
      flexDirection: 'column',
      fontFamily: UI,
      color: C.text,
    }}
  >
    <TitleBar tabs={tabs} active={active} view={view} />
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      {sidebar !== undefined ? (
        <aside
          style={{
            width: 300,
            flex: 'none',
            background: C.panel,
            borderRight: `1px solid ${C.line}`,
            display: 'flex',
            flexDirection: 'column',
            padding: '12px 10px 0',
            gap: 6,
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          {sidebar}
        </aside>
      ) : null}
      <main style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', position: 'relative' }}>{children}</main>
    </div>
    <StatusBar {...status} />
    {overlay}
  </div>
);

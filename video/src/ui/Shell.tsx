import type { CSSProperties, FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { fr } from '../anim';
import { useFmt } from '../lang';
import { C, hue, MONO, soft, UI } from '../theme';
import { Icon, type IconName } from './icons';
import { Logo } from './Logo';

/** A project of the rail (its tile), as the app 2.x draws it. */
export interface Tab {
  name: string;
  hue: number;
  /** Its tile's letters; the name's first ones by default. */
  letters?: string;
  /** Its tile's label, under it; the name by default. */
  label?: string;
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
  /** Claude's quotas, in %, and when they reset. */
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

/** A key, as the app shows its shortcuts. */
export const Kbd: FC<{ children: ReactNode; style?: CSSProperties }> = ({ children, style }) => (
  <span
    style={{
      display: 'inline-flex',
      alignItems: 'center',
      height: 18,
      padding: '0 5px',
      borderRadius: 4,
      boxShadow: `inset 0 0 0 1px ${C.line2}`,
      color: C.muted,
      fontFamily: UI,
      fontSize: 11,
      fontWeight: 500,
      ...style,
    }}
  >
    {children}
  </span>
);

const lettersOf = (t: Tab) =>
  t.letters ??
  t.name
    .split(/[-_ ]/)
    .map((w) => w[0])
    .join('')
    .slice(0, 2)
    .toUpperCase();

export const RAIL = 70;
export const TITLE_H = 44;
export const STATUS_H = 32;
export const SIDE = 272;

/** A project's tile: its letters on its colour, ringed when in front; the agents that ask, or work, on its corner. */
export const ProjectTile: FC<{ tab: Tab; on: boolean; size?: number }> = ({ tab: t, on, size = 38 }) => {
  const frame = useCurrentFrame();
  const a = t.alert && !on ? blink(frame) : 0;
  const color = hue(t.hue);
  return (
    <span
      style={{
        position: 'relative',
        display: 'grid',
        placeItems: 'center',
        width: size,
        height: size,
        borderRadius: 10,
        background: color,
        color: C.onTile,
        fontSize: lettersOf(t).length > 2 ? 11.5 : 13,
        fontWeight: 700,
        letterSpacing: '0.01em',
        opacity: on ? 1 : 0.82,
        boxShadow: on
          ? `0 0 0 2px ${C.panel}, 0 0 0 4px color-mix(in oklch, ${color} 70%, white)`
          : a
            ? `0 0 0 2px ${C.panel}, 0 0 0 ${2 + 3 * a}px ${t.alert}`
            : 'none',
      }}
    >
      {lettersOf(t)}
      {t.waiting ? (
        <span
          style={{
            position: 'absolute',
            top: -6,
            right: -7,
            minWidth: 17,
            height: 17,
            padding: '0 4px',
            borderRadius: 9,
            background: C.wait,
            boxShadow: `0 0 0 2px ${C.panel}, 0 0 0 ${2 + 3 * Math.abs(Math.sin(frame / 6))}px ${soft(C.wait, 35)}`,
            color: C.onWait,
            fontSize: 10.5,
            fontWeight: 700,
            lineHeight: '17px',
            textAlign: 'center',
          }}
        >
          {t.waiting}
        </span>
      ) : t.running ? (
        <span
          style={{
            position: 'absolute',
            right: -3,
            bottom: -3,
            width: 10,
            height: 10,
            borderRadius: '50%',
            background: C.ok,
            boxShadow: `0 0 0 2px ${C.panel}`,
          }}
        />
      ) : null}
    </span>
  );
};

/** A tile of what no project owns (the overview, the stats, the plugins, the settings). */
const GlobalTile: FC<{ icon: IconName; label: string; on?: boolean; add?: boolean }> = ({ icon, label, on, add }) => (
  <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
    <span
      style={{
        display: 'grid',
        placeItems: 'center',
        width: 36,
        height: 36,
        borderRadius: 10,
        background: add ? 'transparent' : on ? C.pSoft : 'rgba(255, 255, 255, 0.06)',
        boxShadow: add ? `inset 0 0 0 1px ${C.line2}` : on ? `0 0 0 2px ${C.panel}, 0 0 0 4px ${C.pInk}` : 'none',
        color: on ? C.pInk : C.muted,
      }}
    >
      <Icon name={icon} size={17} />
    </span>
    {label ? <span style={{ fontSize: 10.5, color: on ? C.text : C.muted, fontWeight: 500 }}>{label}</span> : null}
  </div>
);

export type View = 'project' | 'stats' | 'editor' | 'plugins' | 'settings' | 'overview';

const Rail: FC<{ tabs: Tab[]; active: number; view: View }> = ({ tabs, active, view }) => (
  <nav
    style={{
      width: RAIL,
      flex: 'none',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 12,
      padding: '6px 0 12px',
    }}
  >
    <GlobalTile icon="overview" label="" on={view === 'overview'} />
    <span style={{ width: 24, height: 1, background: C.line2, flex: 'none' }} />
    {tabs.map((t, i) => {
      const e = t.enter ?? 1;
      if (e <= 0) return null;
      const on = i === active && view === 'project';
      return (
        <div
          key={t.name}
          style={{
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: 5,
            opacity: Math.min(1, e),
            transform: `translateY(${(1 - Math.min(1, e)) * 20}px) scale(${0.7 + 0.3 * Math.min(1, e)})`,
          }}
        >
          {on ? (
            <span
              style={{ position: 'absolute', left: -16, top: 8, width: 4, height: 22, borderRadius: '0 3px 3px 0', background: C.pInk }}
            />
          ) : null}
          <ProjectTile tab={t} on={on} />
          <span
            style={{
              maxWidth: RAIL - 6,
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              fontSize: 10.5,
              fontWeight: 500,
              color: on ? C.text : C.muted,
            }}
          >
            {t.label ?? t.name}
          </span>
        </div>
      );
    })}
    <GlobalTile icon="plus" label="" add />
    <div style={{ flex: 1 }} />
    <GlobalTile icon="stats" label="" on={view === 'stats'} />
    <GlobalTile icon="puzzle" label="" on={view === 'plugins'} />
    <GlobalTile icon="sliders" label="" on={view === 'settings'} />
  </nav>
);

const TitleBar: FC<{ tab?: Tab; view: View }> = ({ tab, view }) => {
  const { tr } = useFmt();
  const title =
    view === 'stats'
      ? tr('Statistiques', 'Statistics')
      : view === 'plugins'
        ? 'Plugins'
        : view === 'settings'
          ? tr('Réglages', 'Settings')
          : view === 'overview'
            ? tr("Vue d'ensemble", 'Overview')
            : null;
  return (
    <header
      style={{
        height: TITLE_H,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        background: C.panel,
        fontSize: 14,
      }}
    >
      <div style={{ width: RAIL, display: 'grid', placeItems: 'center' }}>
        <Logo size={24} />
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 9, paddingLeft: 10, minWidth: 330 }}>
        {title ? (
          <b style={{ fontWeight: 600 }}>{title}</b>
        ) : tab ? (
          <>
            <span style={{ width: 9, height: 9, borderRadius: 2.5, background: hue(tab.hue) }} />
            <b style={{ fontWeight: 600, letterSpacing: '-0.01em' }}>{tab.name}</b>
            {tab.delta ? <span style={{ color: C.muted, fontSize: 12.5 }}>Δ {tab.delta}</span> : null}
            <span style={{ color: C.muted }}>
              <Icon name="chevron" size={14} />
            </span>
          </>
        ) : null}
      </div>
      <div style={{ flex: 1, display: 'flex', justifyContent: 'center' }}>
        <div
          style={{
            width: 340,
            height: 30,
            display: 'flex',
            alignItems: 'center',
            gap: 9,
            padding: '0 6px 0 11px',
            borderRadius: 8,
            background: 'rgba(255, 255, 255, 0.04)',
            boxShadow: `inset 0 0 0 1px ${C.line}`,
            color: C.muted,
            fontSize: 13.5,
          }}
        >
          <Icon name="search" size={14} />
          <span style={{ flex: 1 }}>{tr('Rechercher', 'Search')}</span>
          <Kbd>Ctrl K</Kbd>
        </div>
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 7,
          padding: '0 12px',
          height: 30,
          borderRadius: 8,
          background: view === 'editor' ? 'rgba(255, 255, 255, 0.07)' : 'transparent',
          color: view === 'editor' ? C.text : C.soft,
          fontSize: 13.5,
        }}
      >
        <Icon name="code" size={15} />
        {tr('Éditeur', 'Editor')}
      </div>
      <div style={{ alignSelf: 'stretch', display: 'flex', marginLeft: 14 }}>
        {['M3 8h10', 'M3.5 3.5h9v9h-9z', 'M3.5 3.5l9 9M12.5 3.5l-9 9'].map((d) => (
          <span key={d} style={{ width: 46, display: 'grid', placeItems: 'center', color: C.muted }}>
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.3">
              <path d={d} />
            </svg>
          </span>
        ))}
      </div>
    </header>
  );
};

const Meter: FC<{ label: string; pct: number }> = ({ label, pct }) => (
  <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
    {label}
    <span style={{ width: 34, height: 4, borderRadius: 2, background: 'rgba(255, 255, 255, 0.1)', overflow: 'hidden' }}>
      <span style={{ display: 'block', width: `${pct}%`, height: '100%', background: pct > 80 ? C.wait : C.soft }} />
    </span>
  </span>
);

const Sep = () => <span style={{ width: 1, height: 13, background: C.line2 }} />;

export const StatusBar: FC<Status> = (s) => {
  const { tr, trx, usd, pct } = useFmt();
  return (
    <footer
      style={{
        height: STATUS_H,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 15,
        padding: '0 12px 0 16px',
        background: C.panel,
        fontSize: 12.5,
        color: C.muted,
        whiteSpace: 'nowrap',
        fontVariantNumeric: 'tabular-nums',
      }}
    >
      <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <Dot color={C.ok} size={7} />
        <b style={{ color: C.text, fontWeight: 600 }}>{s.active}</b> {trx(<>actif{s.active > 1 ? 's' : ''}</>, 'active')}
      </span>
      <span style={{ display: 'flex', alignItems: 'center', gap: 6, color: s.waiting ? C.wait : C.muted }}>
        <Dot color={s.waiting ? C.wait : C.dim} size={7} pulse={s.waiting > 0} />
        <b style={{ fontWeight: 600, color: s.waiting ? C.wait : C.text }}>{s.waiting}</b> {tr('en attente', 'waiting')}
        {s.waiting ? <Kbd>Ctrl J</Kbd> : null}
      </span>
      <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
        <Icon name="check" size={13} />
        <b style={{ color: C.text, fontWeight: 600 }}>{s.done}</b> {trx(<>terminé{s.done > 1 ? 's' : ''}</>, 'done')}
      </span>
      {s.procs ? (
        <span>
          {trx(<>{s.procs.n} agents</>, `${s.procs.n} agents`)} · <b style={{ color: C.text, fontWeight: 500 }}>{s.procs.mem}</b> ·{' '}
          <b style={{ color: C.text, fontWeight: 500 }}>{s.procs.cpu}</b> CPU
        </span>
      ) : null}
      <div style={{ flex: 1 }} />
      <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <b style={{ color: C.soft, fontWeight: 600 }}>Claude</b>
        <Meter label={tr('5 h', '5h')} pct={s.session} />
        <span style={{ color: C.text }}>{pct(Math.round(s.session))}</span>
        {s.sessionReset ? <span style={{ color: C.dim }}>↻ {s.sessionReset}</span> : null}
        <Meter label={tr('7 j', '7d')} pct={s.weekly} />
        <span style={{ color: C.text }}>{pct(Math.round(s.weekly))}</span>
      </span>
      <Sep />
      <span>
        {tr("Aujourd'hui", 'Today')}{' '}
        <b style={{ color: C.text, fontWeight: 600 }}>
          {s.estimated ? '≈ ' : ''}
          {trx(<>{fr(s.cost)} $</>, usd(s.cost))}
        </b>
      </span>
      {s.sync ? (
        <>
          <Sep />
          <span style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
            <Icon name="branch" size={13} />
            <b style={{ color: C.text, fontWeight: 500, fontFamily: MONO, fontSize: 12 }}>{s.sync.branch}</b>
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
      {s.update ? (
        <span
          style={{
            padding: '2px 10px',
            borderRadius: 6,
            background: soft(C.ok, 16),
            color: C.ok,
            fontSize: 12.5,
            fontWeight: 600,
          }}
        >
          {trx(<>Mise à jour {s.update} disponible → installer</>, `Update ${s.update} available → install`)}
        </span>
      ) : null}
      <Icon name="speaker" size={14} />
    </footer>
  );
};

/** The app: the title bar, the projects' rail, the sidebar, the raised panel of the work, the status bar; `overlay` covers it all. */
export const Shell: FC<{
  tabs: Tab[];
  active?: number;
  view?: View;
  sidebar?: ReactNode;
  children?: ReactNode;
  status: Status;
  overlay?: ReactNode;
}> = ({ tabs, active = 0, view = 'project', sidebar, children, status, overlay }) => (
  <div
    style={{
      ['--p' as string]: hue(tabs[active]?.hue ?? 48),
      position: 'relative',
      width: '100%',
      height: '100%',
      borderRadius: 14,
      overflow: 'hidden',
      background: C.panel,
      boxShadow: `inset 0 0 0 1px rgba(255, 255, 255, 0.1)`,
      display: 'flex',
      flexDirection: 'column',
      fontFamily: UI,
      color: C.text,
      fontFeatureSettings: '"cv11", "ss01"',
    }}
  >
    <TitleBar tab={tabs[active]} view={view} />
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      <Rail tabs={tabs} active={active} view={view} />
      {sidebar !== undefined && sidebar !== null ? (
        <aside
          style={{
            width: SIDE,
            flex: 'none',
            display: 'flex',
            flexDirection: 'column',
            padding: '2px 8px 0 0',
            gap: 6,
            minHeight: 0,
            overflow: 'hidden',
          }}
        >
          {sidebar}
        </aside>
      ) : null}
      <main
        style={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          flexDirection: 'column',
          position: 'relative',
          margin: '0 6px 0 0',
          borderRadius: 10,
          background: C.bg,
          boxShadow: `inset 0 0 0 1px ${C.line}`,
          overflow: 'hidden',
        }}
      >
        {children}
      </main>
    </div>
    <StatusBar {...status} />
    {overlay}
  </div>
);

import type { FC, ReactNode } from 'react';
import { useFmt } from '../lang';
import { C, hue, MONO, soft } from '../theme';
import { Icon, type IconName } from './icons';
import { Kbd } from './Shell';

// The view of the plugins (src/components/plugins/*, design/plugins): its sidebar (the store, what is installed, the
// categories), and the store's lists, each plugin with its tile, its seal and its button.

export interface SideItem {
  id: string;
  label: string;
  icon: IconName;
  n: number;
}

const Head: FC<{ children: ReactNode }> = ({ children }) => (
  <div style={{ padding: '14px 10px 6px', fontSize: 11, fontWeight: 600, letterSpacing: '0.06em', color: C.muted }}>{children}</div>
);

/** The sidebar of the plugins: `on` is the item shown. */
export const PluginsSide: FC<{ on: string }> = ({ on }) => {
  const { tr } = useFmt();
  const top: SideItem[] = [
    { id: 'store', label: 'Store', icon: 'store', n: 47 },
    { id: 'installed', label: tr('Installés', 'Installed'), icon: 'check', n: 14 },
    { id: 'updates', label: tr('Mises à jour', 'Updates'), icon: 'download', n: 1 },
  ];
  const cats: SideItem[] = [
    { id: 'harness', label: 'Harness', icon: 'code', n: 6 },
    { id: 'kanban', label: tr('Sources du Kanban', 'Kanban sources'), icon: 'kanban', n: 9 },
    { id: 'hosts', label: tr('Hébergeurs de code', 'Code hosts'), icon: 'pr', n: 5 },
    { id: 'voice', label: tr('Voix', 'Voice'), icon: 'mic', n: 4 },
    { id: 'themes', label: tr('Thèmes', 'Themes'), icon: 'palette', n: 14 },
    { id: 'notifications', label: 'Notifications', icon: 'bell', n: 11 },
    { id: 'tools', label: tr('Outils', 'Tools'), icon: 'puzzle', n: 7 },
  ];
  const row = (i: SideItem) => (
    <div
      key={i.id}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        height: 32,
        padding: '0 10px',
        borderRadius: 7,
        background: i.id === on ? 'rgba(255, 255, 255, 0.06)' : 'transparent',
        boxShadow: i.id === on ? `inset 0 0 0 1px ${C.line2}` : 'none',
        color: i.id === on ? C.text : C.soft,
        fontSize: 13.5,
      }}
    >
      <span style={{ color: i.id === on ? C.text : C.muted }}>
        <Icon name={i.icon} size={14} />
      </span>
      <span style={{ flex: 1 }}>{i.label}</span>
      <span style={{ color: C.muted, fontSize: 12.5, fontVariantNumeric: 'tabular-nums' }}>{i.n}</span>
    </div>
  );
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
      <Head>PLUGINS</Head>
      {top.map(row)}
      <Head>{tr('CATÉGORIES', 'CATEGORIES')}</Head>
      {cats.map(row)}
    </div>
  );
};

/** « Vérifié v1.2.0 »: a plugin the store checked and signed, its version. */
export const Seal: FC<{ version: string; glow?: number }> = ({ version, glow = 0 }) => {
  const { tr } = useFmt();
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        height: 18,
        padding: '0 6px 0 4px',
        borderRadius: 5,
        background: soft(C.brand, 12 + 18 * glow),
        boxShadow: `inset 0 0 0 1px ${soft(C.brand, 35 + 40 * glow)}${glow ? `, 0 0 ${16 * glow}px ${soft(C.brand, 50 * glow)}` : ''}`,
        color: 'oklch(0.86 0.08 190)',
        fontSize: 11,
        fontWeight: 600,
        whiteSpace: 'nowrap',
      }}
    >
      <Icon name="shield" size={11} />
      {tr('Vérifié', 'Verified')}
      <span style={{ fontFamily: MONO, fontWeight: 500 }}>{version}</span>
    </span>
  );
};

export interface PluginRowInfo {
  name: string;
  letters: string;
  hue: number;
  version: string;
  author: string;
  desc: string;
  /** What it may do, under its description. */
  runs?: string;
  installed?: boolean;
}

/** The button of a plugin: Installer, installing (`progress` 0 to 1), installed. */
const InstallButton: FC<{ installed?: boolean; progress?: number; pressed?: number }> = ({ installed, progress, pressed = 0 }) => {
  const { tr } = useFmt();
  if (installed)
    return (
      <span
        style={{ display: 'inline-flex', alignItems: 'center', gap: 5, color: C.ok, fontSize: 13, fontWeight: 500, whiteSpace: 'nowrap' }}
      >
        <Icon name="check" size={13} />
        {tr('Installé', 'Installed')}
      </span>
    );
  return (
    <span
      style={{
        position: 'relative',
        overflow: 'hidden',
        display: 'inline-flex',
        alignItems: 'center',
        justifyContent: 'center',
        minWidth: 84,
        height: 30,
        padding: '0 12px',
        borderRadius: 7,
        boxShadow: `inset 0 0 0 1px ${C.line2}`,
        background: pressed ? soft(C.spark, 25 * pressed) : 'rgba(255, 255, 255, 0.03)',
        transform: `scale(${1 - 0.05 * pressed})`,
        fontSize: 13,
        fontWeight: 600,
        whiteSpace: 'nowrap',
      }}
    >
      {progress !== undefined ? (
        <span style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: `${progress * 100}%`, background: soft(C.ok, 22) }} />
      ) : null}
      <span style={{ position: 'relative' }}>
        {progress !== undefined ? tr('Installation…', 'Installing…') : tr('Installer', 'Install')}
      </span>
    </span>
  );
};

export const PluginRow: FC<PluginRowInfo & { glow?: number; progress?: number; pressed?: number; highlight?: number; enter?: number }> = ({
  name,
  letters,
  hue: h,
  version,
  author,
  desc,
  runs,
  installed,
  glow,
  progress,
  pressed,
  highlight = 0,
  enter = 1,
}) => {
  const { tr } = useFmt();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        padding: '13px 12px',
        margin: '0 -12px',
        borderRadius: 10,
        borderBottom: `1px solid ${C.line}`,
        background: highlight ? soft(C.spark, 12 * highlight) : 'transparent',
        boxShadow: highlight ? `inset 0 0 0 1px ${soft(C.pInk, 45 * highlight)}` : 'none',
        opacity: Math.min(1, enter),
        transform: `translateY(${(1 - Math.min(1, enter)) * 14}px)`,
      }}
    >
      <span
        style={{
          width: 40,
          height: 40,
          flex: 'none',
          display: 'grid',
          placeItems: 'center',
          borderRadius: 10,
          background: hue(h),
          color: C.onTile,
          fontSize: 14,
          fontWeight: 700,
        }}
      >
        {letters}
      </span>
      <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 3 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, whiteSpace: 'nowrap' }}>
          <b style={{ fontSize: 15, fontWeight: 650 }}>{name}</b>
          <Seal version={version} glow={glow} />
          <span style={{ fontSize: 12.5, color: C.muted }}>{author}</span>
        </div>
        <span style={{ fontSize: 13, color: C.soft, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{desc}</span>
        <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 12, color: C.muted }}>
          <Icon name="code" size={11} />
          {runs ?? tr('Lance un programme', 'Runs a program')}
        </span>
      </div>
      <InstallButton installed={installed} progress={progress} pressed={pressed} />
    </div>
  );
};

/** The store's header: its title, where its catalogue comes from, its search. */
export const StoreHead: FC<{ title: string; sub: string }> = ({ title, sub }) => {
  const { tr } = useFmt();
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14, padding: '22px 28px 6px' }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12 }}>
        <b style={{ fontSize: 22, fontWeight: 700, letterSpacing: '-0.02em' }}>{title}</b>
        <span style={{ fontSize: 13, color: C.muted }}>{sub}</span>
      </div>
      <div
        style={{
          height: 36,
          display: 'flex',
          alignItems: 'center',
          gap: 9,
          padding: '0 8px 0 12px',
          borderRadius: 8,
          boxShadow: `inset 0 0 0 1px ${C.line2}`,
          color: C.muted,
          fontSize: 14,
        }}
      >
        <Icon name="search" size={14} />
        <span style={{ flex: 1 }}>{tr('Chercher dans 47 plugins', 'Search 47 plugins')}</span>
        <Kbd>Ctrl F</Kbd>
      </div>
    </div>
  );
};

/** The categories, as chips. */
export const Chips: FC<{ on: number; items: [string, number][] }> = ({ on, items }) => (
  <div style={{ display: 'flex', gap: 8, padding: '6px 28px 4px', flexWrap: 'wrap' }}>
    {items.map(([label, n], i) => (
      <span
        key={label}
        style={{
          height: 28,
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          padding: '0 11px',
          borderRadius: 7,
          background: i === on ? C.primary : 'transparent',
          color: i === on ? C.onPrimary : C.soft,
          boxShadow: i === on ? 'none' : `inset 0 0 0 1px ${C.line2}`,
          fontSize: 13,
          fontWeight: 500,
        }}
      >
        {label}
        <span style={{ color: i === on ? '#4a4e55' : C.muted }}>{n}</span>
      </span>
    ))}
  </div>
);

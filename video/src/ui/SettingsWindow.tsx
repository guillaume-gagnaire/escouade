import type { FC, ReactNode } from 'react';
import { useFmt } from '../lang';
import { C, hue, MONO, soft } from '../theme';
import { Icon, type IconName } from './icons';
import { Kbd } from './Shell';

// The settings of the 2.2 (src/components/settings/*, design/settings-2-2): a window over the app, its pages on the left,
// Escouade's then each project's, a search at their top; on the right, the page's title, what it holds, its rows.

export type SettingsPageId = 'general' | 'agents' | 'accounts' | 'services' | 'voice' | 'notifications' | 'plugins' | 'advanced';

const pagesOf = (tr: (fr: string, en: string) => string): { id: SettingsPageId; label: string; icon: IconName }[] => [
  { id: 'general', label: tr('Général', 'General'), icon: 'sliders' },
  { id: 'agents', label: 'Agents', icon: 'chat' },
  { id: 'accounts', label: tr('Comptes et harness', 'Accounts and harnesses'), icon: 'user' },
  { id: 'services', label: tr('Services connectés', 'Connected services'), icon: 'globe' },
  { id: 'voice', label: tr('Voix', 'Voice'), icon: 'mic' },
  { id: 'notifications', label: 'Notifications', icon: 'bell' },
  { id: 'plugins', label: 'Plugins', icon: 'puzzle' },
  { id: 'advanced', label: tr('Avancé', 'Advanced'), icon: 'keyboard' },
];

/** A switch, on or off. */
export const Toggle: FC<{ on: boolean }> = ({ on }) => (
  <span
    style={{
      position: 'relative',
      width: 34,
      height: 20,
      flex: 'none',
      borderRadius: 10,
      background: on ? 'oklch(0.62 0.1 190)' : 'rgba(255, 255, 255, 0.12)',
    }}
  >
    <span style={{ position: 'absolute', top: 3, left: on ? 17 : 3, width: 14, height: 14, borderRadius: 7, background: '#f2f3f4' }} />
  </span>
);

/** A field's value, as the page shows it. */
export const Field: FC<{ children: ReactNode; mono?: boolean; select?: boolean; width?: number }> = ({
  children,
  mono,
  select,
  width = 300,
}) => (
  <span
    style={{
      width,
      height: 32,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '0 10px',
      borderRadius: 7,
      background: 'rgba(255, 255, 255, 0.035)',
      boxShadow: `inset 0 0 0 1px ${C.line2}`,
      fontFamily: mono ? MONO : undefined,
      fontSize: mono ? 12.5 : 13.5,
      fontWeight: select ? 600 : 400,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}
  >
    <span style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>{children}</span>
    {select ? <Icon name="chevron" size={13} /> : null}
  </span>
);

/** A row of a page: its label and what it says, then its control; `glow` (0 to 1) shows it. */
export const SettingRow: FC<{ label: string; hint?: string; glow?: number; children: ReactNode }> = ({
  label,
  hint,
  glow = 0,
  children,
}) => (
  <div
    style={{
      display: 'flex',
      alignItems: 'center',
      gap: 24,
      padding: '12px 14px',
      margin: '0 -14px',
      borderRadius: 9,
      borderBottom: `1px solid ${C.line}`,
      background: glow ? soft(C.spark, 12 * glow) : 'transparent',
      boxShadow: glow ? `inset 0 0 0 1px ${soft(C.pInk, 55 * glow)}` : 'none',
    }}
  >
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
      <span style={{ fontSize: 14, fontWeight: 550 }}>{label}</span>
      {hint ? <span style={{ fontSize: 12.5, color: C.muted }}>{hint}</span> : null}
    </div>
    {children}
  </div>
);

export const GroupHead: FC<{ children: ReactNode }> = ({ children }) => (
  <div
    style={{ padding: '18px 0 4px', fontSize: 12, fontWeight: 600, letterSpacing: '0.05em', color: C.muted, textTransform: 'uppercase' }}
  >
    {children}
  </div>
);

const PROJECTS: [string, number][] = [
  ['demo-api', 48],
  ['studio-web', 300],
  ['mobile-app', 200],
  ['infra', 150],
];

/** The window of the settings, on page `page`; `enter` (0 to 1) brings it in. */
export const SettingsWindow: FC<{ page: SettingsPageId; title: string; desc: string; enter?: number; children: ReactNode }> = ({
  page,
  title,
  desc,
  enter = 1,
  children,
}) => {
  const { tr } = useFmt();
  if (enter <= 0) return null;
  const nav = (on: boolean, icon: ReactNode, label: string) => (
    <div
      key={label}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        height: 31,
        padding: '0 10px',
        borderRadius: 7,
        background: on ? 'rgba(255, 255, 255, 0.07)' : 'transparent',
        boxShadow: on ? `inset 0 0 0 1px ${C.line2}` : 'none',
        color: on ? C.text : C.soft,
        fontSize: 13.5,
        fontWeight: on ? 600 : 500,
      }}
    >
      {icon}
      {label}
    </div>
  );
  return (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        background: `rgba(3, 4, 5, ${0.62 * Math.min(1, enter)})`,
        zIndex: 40,
        display: 'grid',
        placeItems: 'center',
      }}
    >
      <div
        style={{
          width: 1120,
          height: 760,
          display: 'flex',
          borderRadius: 14,
          overflow: 'hidden',
          background: C.bg,
          boxShadow: `inset 0 0 0 1px ${C.line2}, 0 40px 100px rgba(0, 0, 0, 0.6)`,
          opacity: Math.min(1, enter),
          transform: `translateY(${(1 - Math.min(1, enter)) * 24}px) scale(${0.97 + 0.03 * Math.min(1, enter)})`,
        }}
      >
        <aside style={{ width: 250, flex: 'none', display: 'flex', flexDirection: 'column', gap: 2, padding: 12, background: C.panel }}>
          <div
            style={{
              height: 32,
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              padding: '0 6px 0 10px',
              marginBottom: 12,
              borderRadius: 7,
              boxShadow: `inset 0 0 0 1px ${C.line2}`,
              color: C.muted,
              fontSize: 13,
            }}
          >
            <Icon name="search" size={13} />
            <span style={{ flex: 1 }}>{tr('Rechercher un réglage', 'Search settings')}</span>
            <Kbd>Ctrl+F</Kbd>
          </div>
          <span style={{ padding: '0 10px 6px', fontSize: 12, fontWeight: 600, color: C.muted }}>Escouade</span>
          {pagesOf(tr).map((p) =>
            nav(
              p.id === page,
              <span style={{ color: p.id === page ? C.text : C.muted }}>
                <Icon name={p.icon} size={15} />
              </span>,
              p.label,
            ),
          )}
          <span style={{ padding: '16px 10px 6px', fontSize: 12, fontWeight: 600, color: C.muted }}>{tr('Projets', 'Projects')}</span>
          {PROJECTS.map(([name, h]) =>
            nav(false, <span style={{ width: 9, height: 9, borderRadius: 2.5, background: hue(h), margin: '0 3px' }} />, name),
          )}
          <span style={{ flex: 1 }} />
          {nav(
            false,
            <span style={{ color: C.muted }}>
              <Icon name="doc" size={15} />
            </span>,
            tr('À propos', 'About'),
          )}
        </aside>
        <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', padding: '26px 34px' }}>
          <div style={{ display: 'flex', alignItems: 'flex-start' }}>
            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 6 }}>
              <b style={{ fontSize: 19, fontWeight: 700, letterSpacing: '-0.01em' }}>{title}</b>
              <span style={{ fontSize: 13.5, color: C.muted }}>{desc}</span>
            </div>
            <span style={{ color: C.muted, fontSize: 18 }}>✕</span>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', marginTop: 8 }}>{children}</div>
        </div>
      </div>
    </div>
  );
};

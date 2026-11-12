// The settings of the 2.2 (src/components/settings/*): Escouade's pages, then each project's, on the left; a page on the right.

import type { FC, ReactNode } from 'react';
import { TABS as PROJECTS } from '../data';
import { useFmt, type Tr } from '../lang';
import { C, hue } from '../theme';
import { Icon, type IconName } from './icons';
import { Kbd } from './Shell';
import { Button } from './Sidebar';

export type SettingsTabId = 'board' | 'integrations';

/** The pages of the 2.2: Escouade's, then those of the project in front. */
const appPagesOf = (tr: Tr): { label: string; icon: IconName }[] => [
  { label: tr('Général', 'General'), icon: 'sliders' },
  { label: 'Agents', icon: 'chat' },
  { label: tr('Comptes et harness', 'Accounts and harnesses'), icon: 'user' },
  { label: tr('Services connectés', 'Connected services'), icon: 'globe' },
  { label: tr('Voix', 'Voice'), icon: 'mic' },
  { label: 'Notifications', icon: 'bell' },
  { label: 'Plugins', icon: 'puzzle' },
  { label: tr('Avancé', 'Advanced'), icon: 'keyboard' },
];
const projectPagesOf = (tr: Tr): { id?: SettingsTabId; label: string; icon: IconName; desc?: string }[] => [
  { label: tr('Projet', 'Project'), icon: 'doc' },
  { label: tr('Git et worktrees', 'Git and worktrees'), icon: 'branch' },
  { label: tr('Lancement', 'Launch'), icon: 'code' },
  {
    id: 'board',
    label: 'Kanban',
    icon: 'kanban',
    desc: tr(
      'Les tickets validés, le pilote auto, les agents des tickets et le workflow',
      'Approved tickets, autopilot, ticket agents and the workflow',
    ),
  },
  {
    id: 'integrations',
    label: tr('Tickets importés', 'Imported tickets'),
    icon: 'download',
    desc: tr(
      'Les sources liées à ce projet et la correspondance de leurs statuts',
      'The sources linked to this project and how their statuses map',
    ),
  },
  { label: tr('Équipe', 'Team'), icon: 'user' },
];

const tabsOf = (tr: Tr) => projectPagesOf(tr).filter((p) => p.id) as { id: SettingsTabId; label: string; icon: IconName; desc: string }[];

/** The settings: their tabs on the left, one tab at a time; `enter` (0 to 1) brings them in, `scroll` moves the tab's body up. */
export const SettingsShell: FC<{
  tab: SettingsTabId;
  enter: number;
  height?: number;
  scroll?: number;
  /** Tabs marked as changed, not yet saved. */
  changed?: SettingsTabId[];
  savePressed?: number;
  children: ReactNode;
}> = ({ tab, enter, height = 700, scroll = 0, changed = [], savePressed = 0, children }) => {
  const { tr } = useFmt();
  if (enter <= 0) return null;
  const TABS = tabsOf(tr);
  const current = TABS.find((t) => t.id === tab)!;
  const e = Math.min(1, enter);
  return (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        background: `rgba(3, 4, 5, ${0.62 * e})`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 40,
      }}
    >
      <div
        style={{
          width: 980,
          height,
          borderRadius: 14,
          background: C.bg,
          boxShadow: `inset 0 0 0 1px ${C.line2}, 0 30px 90px rgba(0, 0, 0, 0.6)`,
          opacity: e,
          transform: `translateY(${(1 - e) * 30}px) scale(${0.96 + 0.04 * e})`,
          display: 'flex',
          overflow: 'hidden',
        }}
      >
        <nav
          style={{
            width: 220,
            flex: 'none',
            display: 'flex',
            flexDirection: 'column',
            gap: 1,
            padding: '12px 10px 12px',
            background: C.panel,
          }}
        >
          <div
            style={{
              height: 30,
              display: 'flex',
              alignItems: 'center',
              gap: 7,
              padding: '0 5px 0 9px',
              marginBottom: 8,
              borderRadius: 7,
              boxShadow: `inset 0 0 0 1px ${C.line2}`,
              color: C.muted,
              fontSize: 12.5,
              whiteSpace: 'nowrap',
            }}
          >
            <Icon name="search" size={12} />
            <span style={{ flex: 1 }}>{tr('Rechercher un réglage', 'Search settings')}</span>
            <Kbd>Ctrl+F</Kbd>
          </div>
          <span style={{ padding: '2px 9px 4px', fontSize: 11.5, fontWeight: 600, color: C.muted }}>Escouade</span>
          {appPagesOf(tr).map((t) => (
            <span
              key={t.label}
              style={{ height: 27, display: 'flex', alignItems: 'center', gap: 9, padding: '0 9px', color: C.soft, fontSize: 13 }}
            >
              <span style={{ color: C.muted }}>
                <Icon name={t.icon} size={13} />
              </span>
              {t.label}
            </span>
          ))}
          <span
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 7,
              padding: '12px 9px 4px',
              fontSize: 11.5,
              fontWeight: 600,
              color: C.muted,
            }}
          >
            <span style={{ width: 8, height: 8, borderRadius: 2.5, background: hue(PROJECTS[0].hue) }} />
            {PROJECTS[0].name}
          </span>
          {projectPagesOf(tr).map((t) => {
            const on = t.id === tab;
            return (
              <span
                key={t.label}
                style={{
                  height: 28,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 9,
                  padding: '0 9px',
                  borderRadius: 7,
                  background: on ? 'rgba(255, 255, 255, 0.07)' : 'transparent',
                  boxShadow: on ? `inset 0 0 0 1px ${C.line2}` : 'none',
                  color: on ? C.text : C.soft,
                  fontSize: 13,
                  fontWeight: on ? 600 : 400,
                }}
              >
                <span style={{ color: on ? C.text : C.muted }}>
                  <Icon name={t.icon} size={13} />
                </span>
                {t.label}
                {t.id && changed.includes(t.id) ? (
                  <span style={{ width: 7, height: 7, marginLeft: 'auto', borderRadius: '50%', background: C.pInk }} />
                ) : null}
              </span>
            );
          })}
          <div style={{ flex: 1 }} />
          <span style={{ padding: '0 9px', fontSize: 12, color: C.muted }}>{tr('À propos', 'About')} · 2.2</span>
        </nav>
        <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
          <div
            style={{
              flex: 'none',
              display: 'flex',
              alignItems: 'flex-start',
              padding: '22px 20px 15px 30px',
              borderBottom: `1px solid ${C.line}`,
            }}
          >
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={{ fontSize: 19.5, fontWeight: 700 }}>{current.label}</span>
              <span style={{ fontSize: 13.5, color: C.muted }}>{current.desc}</span>
            </div>
            <div style={{ flex: 1 }} />
            <span style={{ color: C.dim, fontSize: 17 }}>✕</span>
          </div>
          <div
            style={{
              flex: 'none',
              display: 'flex',
              alignItems: 'center',
              gap: 11,
              padding: '12px 30px',
              borderBottom: `1px solid ${C.line}`,
            }}
          >
            <span style={{ fontSize: 12.5, fontWeight: 700, color: C.dim }}>{tr('Projet', 'Project')}</span>
            {PROJECTS.map((p, i) => (
              <span
                key={p.name}
                style={{
                  height: 30,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '0 11px',
                  borderRadius: 6,
                  border: `1px solid ${i === 0 ? C.line2 : 'transparent'}`,
                  background: i === 0 ? C.elev : 'transparent',
                  color: i === 0 ? C.text : C.muted,
                  fontSize: 13.5,
                  fontWeight: 600,
                }}
              >
                <span style={{ width: 9, height: 9, borderRadius: 3, background: hue(p.hue) }} />
                {p.name}
              </span>
            ))}
          </div>
          <div style={{ flex: 1, minHeight: 0, overflow: 'hidden' }}>
            <div
              style={{ padding: '6px 30px 20px', display: 'flex', flexDirection: 'column', gap: 4, transform: `translateY(${-scroll}px)` }}
            >
              {children}
            </div>
          </div>
          <div
            style={{
              flex: 'none',
              display: 'flex',
              justifyContent: 'flex-end',
              gap: 10,
              padding: '14px 22px',
              borderTop: `1px solid ${C.line}`,
            }}
          >
            <Button>{tr('Annuler', 'Cancel')}</Button>
            <span style={{ transform: `scale(${1 - 0.06 * savePressed})`, display: 'flex' }}>
              <Button primary={C.spark}>{tr('Enregistrer', 'Save')}</Button>
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};

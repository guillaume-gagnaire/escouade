// The settings, in tabs (SettingsModal.svelte of the app).

import type { FC, ReactNode } from 'react';
import { TABS as PROJECTS } from '../data';
import { useFmt, type Tr } from '../lang';
import { C, hue, MONO } from '../theme';
import { Button } from './Sidebar';

const TAB_IDS = ['claude', 'notifications', 'projects', 'board', 'integrations', 'terminals', 'network', 'about'] as const;
export type SettingsTabId = (typeof TAB_IDS)[number];

const tabsOf = (tr: Tr): { id: SettingsTabId; label: string; icon: string; desc: string }[] => [
  {
    id: 'claude',
    label: 'Claude Code',
    icon: '✳',
    desc: tr('Exécutable, modèle et permissions par défaut', 'Executable, model and default permissions'),
  },
  { id: 'notifications', label: 'Notifications', icon: '♪', desc: tr('Alertes visuelles et sonores', 'Visual and sound alerts') },
  {
    id: 'projects',
    label: tr('Projets', 'Projects'),
    icon: '▤',
    desc: tr('Réglages propres à chaque projet', 'Settings specific to each project'),
  },
  { id: 'board', label: 'Kanban', icon: '▦', desc: tr('Pilote auto et tickets validés', 'Autopilot and approved tickets') },
  {
    id: 'integrations',
    label: tr('Intégrations', 'Integrations'),
    icon: '⧉',
    desc: tr('Jira, Trello et GitHub Issues', 'Jira, Trello and GitHub Issues'),
  },
  {
    id: 'terminals',
    label: tr('Terminaux', 'Terminals'),
    icon: '$_',
    desc: tr('Shells disponibles dans les terminaux intégrés', 'Shells available in the built-in terminals'),
  },
  {
    id: 'network',
    label: tr('Réseau', 'Network'),
    icon: '⇄',
    desc: tr('Proxy HTTP(S) pour Claude Code et les mises à jour', 'HTTP(S) proxy for Claude Code and updates'),
  },
  { id: 'about', label: tr('À propos', 'About'), icon: 'ⓘ', desc: tr('Version et données locales', 'Version and local data') },
];

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
        background: `rgba(10, 9, 8, ${0.55 * e})`,
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
          background: C.panel,
          border: `1px solid ${C.line2}`,
          boxShadow: '0 30px 90px rgba(0, 0, 0, 0.6)',
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
            padding: '22px 11px 18px',
            background: C.bg,
            borderRight: `1px solid ${C.line}`,
          }}
        >
          <span style={{ padding: '0 11px 15px', fontSize: 18.5, fontWeight: 700 }}>{tr('Réglages', 'Settings')}</span>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {TABS.map((t) => {
              const on = t.id === tab;
              return (
                <span
                  key={t.id}
                  style={{
                    height: 37,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 11,
                    padding: '0 11px',
                    borderRadius: 6,
                    background: on ? C.elev2 : 'transparent',
                    color: on ? C.text : C.muted,
                    fontSize: 14,
                    fontWeight: 600,
                  }}
                >
                  <span style={{ width: 20, textAlign: 'center', fontFamily: MONO, fontSize: 13, color: on ? C.spark : C.dim }}>
                    {t.icon}
                  </span>
                  {t.label}
                  {changed.includes(t.id) ? (
                    <span style={{ width: 7, height: 7, marginLeft: 'auto', borderRadius: '50%', background: C.spark }} />
                  ) : null}
                </span>
              );
            })}
          </div>
          <div style={{ flex: 1 }} />
          <span style={{ padding: '0 11px', fontFamily: MONO, fontSize: 11.5, color: C.dim }}>Escouade 1.4.0</span>
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

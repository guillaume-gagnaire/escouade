import type { FC, ReactNode } from 'react';
import { C, MONO, soft, UI } from '../theme';
import { CodeLine } from './code';
import { Button } from './Sidebar';

export interface TreeNode {
  name: string;
  depth: number;
  dir?: boolean;
  open?: boolean;
  status?: 'M' | 'A';
  /** A folder that holds changed files. */
  changed?: boolean;
}

export interface EditorLine {
  text: string;
  /** Changed against the reference: a bar in the gutter. */
  mark?: 'changed' | 'added';
}

/** The editor: tree, tabs, code with its changed lines, status line; `banner` warns of a change on disk. */
export const EditorView: FC<{
  source: string;
  tree: TreeNode[];
  selected: number;
  tabs: { name: string; dirty?: boolean }[];
  activeTab: number;
  path: string;
  changed: number;
  lines: EditorLine[];
  first: number;
  cursorLine: number;
  dirty?: boolean;
  search?: { query: string; hits: number[] };
  banner?: ReactNode;
  highlight?: { tree?: number; tabs?: number; gutter?: number };
  pressedSave?: number;
}> = ({ source, tree, selected, tabs, activeTab, path, changed, lines, first, cursorLine, dirty, search, banner, pressedSave }) => (
  <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
    <div
      style={{
        height: 56,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        padding: '0 18px',
        borderBottom: `1px solid ${C.line}`,
      }}
    >
      <span style={{ fontSize: 14, fontWeight: 600, color: C.muted }}>← Conversation</span>
      <span style={{ width: 1, height: 20, background: C.line2 }} />
      <span
        style={{
          height: 32,
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '0 12px',
          borderRadius: 6,
          border: `1px solid ${C.line2}`,
          background: C.panel,
          fontFamily: MONO,
          fontSize: 13,
        }}
      >
        <span style={{ width: 7, height: 7, borderRadius: '50%', background: C.ok }} />
        <span style={{ fontFamily: 'inherit' }}>{source.split(' · ')[1]}</span>
        <span style={{ fontSize: 11, padding: '1px 6px', borderRadius: 3, background: C.elev2, color: C.muted }}>
          {source.split(' · ')[0]}
        </span>
        <span style={{ color: C.dim, fontSize: 10 }}>▾</span>
      </span>
      <div style={{ flex: 1 }} />
      <span style={{ fontFamily: MONO, fontSize: 12.5, color: dirty ? C.wait : C.dim }}>
        {dirty ? '● Non enregistré · Ctrl+S' : 'Enregistré'}
      </span>
      <Button primary={dirty ? C.spark : undefined} pressed={pressedSave}>
        Enregistrer
      </Button>
    </div>
    <div style={{ flex: 1, minHeight: 0, display: 'flex' }}>
      <aside
        style={{
          width: 270,
          flex: 'none',
          borderRight: `1px solid ${C.line}`,
          background: C.panel,
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <div style={{ padding: '12px 14px 8px', display: 'flex', flexDirection: 'column', gap: 4 }}>
          <span style={{ display: 'flex', gap: 8, alignItems: 'baseline' }}>
            <span style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.08em', textTransform: 'uppercase', color: C.muted }}>
              Fichiers
            </span>
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>128 · 3 modif.</span>
          </span>
          <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>.claude/worktrees/{source.split(' · ')[1]}</span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', padding: '0 6px' }}>
          {tree.map((n, i) => (
            <span
              key={i}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                height: 27,
                paddingLeft: 8 + n.depth * 14,
                paddingRight: 8,
                borderRadius: 5,
                background: i === selected ? C.elev2 : 'transparent',
                fontFamily: MONO,
                fontSize: 13,
                color: n.status === 'A' ? C.ok : n.status === 'M' ? C.wait : n.dir ? C.muted : C.text,
              }}
            >
              <span style={{ width: 10, color: C.dim, fontSize: 10 }}>{n.dir ? (n.open ? '▾' : '▸') : ''}</span>
              <span style={{ flex: 1 }}>
                {n.name}
                {n.dir && n.changed ? <span style={{ color: C.wait, marginLeft: 6 }}>•</span> : null}
              </span>
              {n.status ? <span style={{ fontWeight: 700, color: n.status === 'A' ? C.ok : C.wait }}>{n.status}</span> : null}
            </span>
          ))}
        </div>
      </aside>
      <section style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', background: C.term }}>
        <div
          style={{
            height: 40,
            flex: 'none',
            display: 'flex',
            alignItems: 'flex-end',
            background: C.panel,
            borderBottom: `1px solid ${C.line}`,
          }}
        >
          {tabs.map((t, i) => (
            <span
              key={t.name}
              style={{
                height: 40,
                display: 'flex',
                alignItems: 'center',
                gap: 9,
                padding: '0 16px',
                borderRight: `1px solid ${C.line}`,
                background: i === activeTab ? C.term : 'transparent',
                borderTop: `2px solid ${i === activeTab ? C.spark : 'transparent'}`,
                fontFamily: MONO,
                fontSize: 13,
                color: i === activeTab ? C.text : C.muted,
              }}
            >
              {t.name}
              <span style={{ color: t.dirty ? C.wait : C.dim }}>{t.dirty ? '●' : '×'}</span>
            </span>
          ))}
        </div>
        <div
          style={{
            display: 'flex',
            gap: 16,
            padding: '8px 18px',
            fontFamily: MONO,
            fontSize: 12.5,
            color: C.dim,
            borderBottom: `1px solid ${C.line}`,
          }}
        >
          <span>{path.split('/').join('  /  ')}</span>
          <span style={{ flex: 1 }} />
          <span style={{ color: C.ok }}>{changed} lignes modifiées vs main</span>
        </div>
        {banner}
        <div
          style={{
            flex: 1,
            minHeight: 0,
            position: 'relative',
            padding: '10px 0',
            fontFamily: MONO,
            fontSize: 15,
            lineHeight: '25px',
            overflow: 'hidden',
          }}
        >
          {search ? (
            <div
              style={{
                display: 'flex',
                flexDirection: 'column',
                gap: 6,
                padding: '8px 14px',
                marginTop: -10,
                marginBottom: 8,
                background: C.panel,
                borderBottom: `1px solid ${C.line}`,
                fontFamily: UI,
                fontSize: 13,
                color: C.muted,
              }}
            >
              <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <span
                  style={{
                    width: 200,
                    padding: '3px 8px',
                    borderRadius: 4,
                    border: `1px solid ${C.spark}`,
                    background: C.bg,
                    color: C.text,
                    fontFamily: MONO,
                  }}
                >
                  {search.query}
                </span>
                <span>suivant</span>
                <span>précédent</span>
                <span>tout</span>
                <span>☐ respecter la casse</span>
                <span>☐ expression régulière</span>
                <span>☐ mot entier</span>
                <span style={{ flex: 1 }} />
                <span>×</span>
              </span>
              <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <span
                  style={{
                    width: 200,
                    padding: '3px 8px',
                    borderRadius: 4,
                    border: `1px solid ${C.line2}`,
                    background: C.bg,
                    color: C.dim,
                  }}
                >
                  Remplacer
                </span>
                <span>remplacer</span>
                <span>tout remplacer</span>
              </span>
            </div>
          ) : null}
          {lines.map((l, i) => {
            const n = first + i;
            const hit = search?.hits.includes(n);
            return (
              <div key={n} style={{ display: 'flex', whiteSpace: 'pre', background: n === cursorLine ? soft(C.spark, 9) : 'transparent' }}>
                <span style={{ width: 4, flex: 'none', background: l.mark ? C.ok : 'transparent' }} />
                <span style={{ width: 52, flex: 'none', textAlign: 'right', paddingRight: 16, color: n === cursorLine ? C.text : C.dim }}>
                  {n}
                </span>
                <span style={{ background: hit ? soft(C.wait, 30) : 'transparent' }}>
                  <CodeLine line={l.text} />
                  {n === cursorLine ? (
                    <span style={{ display: 'inline-block', width: 2, height: 19, background: C.text, verticalAlign: -4 }} />
                  ) : null}
                </span>
              </div>
            );
          })}
        </div>
        <div
          style={{
            height: 30,
            flex: 'none',
            display: 'flex',
            alignItems: 'center',
            gap: 18,
            padding: '0 16px',
            borderTop: `1px solid ${C.line}`,
            background: C.panel,
            fontFamily: MONO,
            fontSize: 12,
            color: C.muted,
          }}
        >
          <span>Ln {cursorLine}, Col 22</span>
          <span>TypeScript</span>
          <span>UTF-8</span>
          <span>LF</span>
          <span>Espaces : 2</span>
          <span style={{ flex: 1 }} />
          <span>{source}</span>
        </div>
      </section>
    </div>
  </div>
);

/** « Ce fichier a été modifié sur le disque. » with its choices. */
export const DiskBanner: FC<{ enter: number }> = ({ enter }) =>
  enter <= 0 ? null : (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '10px 18px',
        background: soft(C.wait, 14 * Math.min(1, enter)),
        borderBottom: `1px solid ${soft(C.wait, 50)}`,
        fontSize: 14.5,
        color: C.wait,
        opacity: Math.min(1, enter),
      }}
    >
      <span style={{ flex: 1 }}>Ce fichier a changé sur le disque.</span>
      <Button small>Recharger</Button>
      <Button small>Garder ma version</Button>
    </div>
  );

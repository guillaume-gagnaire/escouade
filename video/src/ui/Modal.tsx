import type { FC, ReactNode } from 'react';
import { C, MONO, soft } from '../theme';
import { Cursor } from './Cursor';
import { Button } from './Sidebar';

/** A modal over the app's window; `enter` (0 to 1) brings it in. */
export const Modal: FC<{ title: string; sub?: string; width?: number; enter: number; children: ReactNode; footer?: ReactNode }> = ({
  title,
  sub,
  width = 620,
  enter,
  children,
  footer,
}) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        background: `rgba(10, 9, 8, ${0.55 * Math.min(1, enter)})`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 40,
      }}
    >
      <div
        style={{
          width,
          borderRadius: 14,
          background: C.panel,
          border: `1px solid ${C.line2}`,
          boxShadow: '0 30px 90px rgba(0, 0, 0, 0.6)',
          opacity: Math.min(1, enter),
          transform: `translateY(${(1 - Math.min(1, enter)) * 30}px) scale(${0.96 + 0.04 * Math.min(1, enter)})`,
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <div style={{ padding: '18px 22px 14px', borderBottom: `1px solid ${C.line}`, display: 'flex', alignItems: 'baseline', gap: 10 }}>
          <span style={{ fontSize: 18, fontWeight: 700 }}>{title}</span>
          {sub ? <span style={{ fontFamily: MONO, fontSize: 12.5, color: C.dim }}>{sub}</span> : null}
          <div style={{ flex: 1 }} />
          <span style={{ color: C.dim }}>✕</span>
        </div>
        <div style={{ padding: '18px 22px', display: 'flex', flexDirection: 'column', gap: 16 }}>{children}</div>
        {footer ? (
          <div style={{ padding: '14px 22px', borderTop: `1px solid ${C.line}`, display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
            {footer}
          </div>
        ) : null}
      </div>
    </div>
  );

export const Label: FC<{ children: ReactNode }> = ({ children }) => (
  <span style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.06em', textTransform: 'uppercase', color: C.muted }}>{children}</span>
);

export const Field: FC<{ value: string; mono?: boolean; placeholder?: string; focus?: boolean; grow?: boolean }> = ({
  value,
  mono,
  placeholder,
  focus,
  grow = true,
}) => (
  <span
    style={{
      flex: grow ? 1 : 'none',
      height: 38,
      display: 'flex',
      alignItems: 'center',
      padding: '0 12px',
      borderRadius: 6,
      border: `1px solid ${focus ? C.spark : C.line2}`,
      background: C.bg,
      fontFamily: mono ? MONO : 'inherit',
      fontSize: mono ? 13.5 : 15,
      color: value ? C.text : C.dim,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}
  >
    {value || placeholder}
    {focus ? <span style={{ width: 2, height: 18, background: C.text, marginLeft: 1 }} /> : null}
  </span>
);

/** An on/off switch. */
export const Switch: FC<{ on: boolean }> = ({ on }) => (
  <span
    style={{
      width: 40,
      height: 22,
      flex: 'none',
      borderRadius: 11,
      background: on ? C.spark : C.elev2,
      position: 'relative',
      display: 'inline-block',
    }}
  >
    <span
      style={{
        position: 'absolute',
        top: 3,
        left: on ? 21 : 3,
        width: 16,
        height: 16,
        borderRadius: 8,
        background: on ? '#1b1512' : C.muted,
      }}
    />
  </span>
);

export const Toggle: FC<{ title: string; desc?: string; on: boolean; children?: ReactNode }> = ({ title, desc, on, children }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
      <span style={{ fontSize: 15, fontWeight: 600 }}>{title}</span>
      {desc ? <span style={{ fontSize: 13, color: C.muted }}>{desc}</span> : null}
      {children}
    </div>
    <Switch on={on} />
  </div>
);

/** A row of chips, one of them on. */
export const Chips: FC<{ items: string[]; on: number; mono?: boolean }> = ({ items, on, mono }) => (
  <span style={{ display: 'flex', gap: 6 }}>
    {items.map((c, i) => (
      <span
        key={c}
        style={{
          height: 30,
          padding: '0 12px',
          display: 'flex',
          alignItems: 'center',
          borderRadius: 6,
          border: `1px solid ${i === on ? C.spark : C.line2}`,
          background: i === on ? soft(C.spark, 18) : 'transparent',
          color: i === on ? C.text : C.muted,
          fontFamily: mono ? MONO : 'inherit',
          fontSize: 13.5,
          fontWeight: 600,
        }}
      >
        {c}
      </span>
    ))}
  </span>
);

/** A context menu at (x, y) of the window. */
export const ContextMenu: FC<{
  x: number;
  y: number;
  items: (string | { label: string; hint?: string; danger?: boolean; colors?: string[]; selected?: number } | null)[];
  hover?: number;
  enter?: number;
}> = ({ x, y, items, hover, enter = 1 }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        minWidth: 250,
        padding: 5,
        borderRadius: 10,
        background: C.elev,
        border: `1px solid ${C.line2}`,
        boxShadow: '0 18px 50px rgba(0, 0, 0, 0.55)',
        zIndex: 45,
        opacity: Math.min(1, enter),
        transform: `scale(${0.95 + 0.05 * Math.min(1, enter)})`,
        transformOrigin: '0 0',
      }}
    >
      {items.map((it, i) => {
        if (it === null) return <div key={i} style={{ height: 1, background: C.line2, margin: '5px 6px' }} />;
        const o = typeof it === 'string' ? { label: it } : it;
        if ('colors' in o && o.colors)
          return (
            <div key={i} style={{ padding: '6px 12px 8px', borderRadius: 6, background: i === hover ? C.elev2 : 'transparent' }}>
              <div style={{ fontSize: 14.5, marginBottom: 7 }}>{o.label}</div>
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, maxWidth: 230 }}>
                {o.colors.map((c, k) => (
                  <span
                    key={k}
                    style={{
                      width: 18,
                      height: 18,
                      borderRadius: 5,
                      background: c,
                      boxShadow: k === o.selected ? `0 0 0 2px ${C.elev}, 0 0 0 3px ${c}` : 'none',
                    }}
                  />
                ))}
              </div>
            </div>
          );
        return (
          <div
            key={i}
            style={{
              display: 'flex',
              gap: 14,
              alignItems: 'baseline',
              padding: '8px 12px',
              borderRadius: 6,
              background: i === hover ? C.elev2 : 'transparent',
              fontSize: 14.5,
              color: o.danger ? C.del : C.text,
            }}
          >
            {o.label}
            <span style={{ flex: 1 }} />
            {o.hint ? <span style={{ fontSize: 12.5, color: C.dim }}>{o.hint}</span> : null}
          </div>
        );
      })}
    </div>
  );

/** The pointer, gliding from (x0, y0) to (x1, y1) over t (0 to 1), clicking with `click`. */
export const Pointer: FC<{ from: [number, number]; to: [number, number]; t: number; click?: number; visible?: boolean }> = ({
  from,
  to,
  t,
  click = 0,
  visible = true,
}) => {
  if (!visible) return null;
  const e = t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2;
  return <Cursor x={from[0] + (to[0] - from[0]) * e} y={from[1] + (to[1] - from[1]) * e} click={click} />;
};

/** The app's confirmation: a question, its consequence, an optional checkbox, then « Annuler » and the action. */
export const ConfirmModal: FC<{
  title: string;
  body: string;
  confirm: string;
  danger?: boolean;
  option?: { label: string; checked: boolean };
  enter: number;
  pressed?: number;
}> = ({ title, body, confirm, danger, option, enter, pressed }) => (
  <Modal
    title={title}
    width={560}
    enter={enter}
    footer={
      <>
        <Button>Annuler</Button>
        <Button primary={danger ? C.del : C.spark} pressed={pressed}>
          {confirm}
        </Button>
      </>
    }
  >
    <span style={{ fontSize: 15, lineHeight: 1.5, color: C.muted }}>{body}</span>
    {option ? (
      <span style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 14.5 }}>
        <span
          style={{
            width: 18,
            height: 18,
            borderRadius: 4,
            border: `1.5px solid ${option.checked ? C.spark : C.line2}`,
            background: option.checked ? C.spark : 'transparent',
            color: '#1b1512',
            fontSize: 13,
            fontWeight: 800,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          {option.checked ? '✓' : ''}
        </span>
        {option.label}
      </span>
    ) : null}
  </Modal>
);

import type { FC } from 'react';
import { C, soft, UI } from '../theme';
import { Logo } from './Logo';

/** A Windows notification, at the bottom right of the screen. `enter` slides it in (0 to 1). */
export const WinToast: FC<{ title: string; body: string; enter: number; bottom?: number }> = ({ title, body, enter, bottom = 36 }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        right: 36,
        bottom,
        width: 460,
        padding: '16px 18px',
        borderRadius: 10,
        background: '#2b2b2b',
        border: '1px solid rgba(255, 255, 255, 0.1)',
        boxShadow: '0 20px 60px rgba(0, 0, 0, 0.6)',
        transform: `translateX(${(1 - enter) * 540}px)`,
        fontFamily: UI,
        color: '#ffffff',
        zIndex: 70,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, color: '#bbbbbb' }}>
        <Logo size={18} /> Escouade
      </div>
      <div style={{ marginTop: 10, fontSize: 18, fontWeight: 700 }}>{title}</div>
      <div style={{ marginTop: 4, fontSize: 15, color: '#dddddd' }}>{body}</div>
    </div>
  );

const TONE = { error: C.del, ok: C.ok, info: C.info };

/** One of the app's own notifications, at the bottom right of its window. */
export const AppToast: FC<{ text: string; enter: number; tone?: keyof typeof TONE }> = ({ text, enter, tone = 'error' }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        right: 20,
        bottom: 50,
        padding: '12px 16px',
        borderRadius: 10,
        border: `1px solid ${TONE[tone]}`,
        background: `color-mix(in oklch, ${soft(TONE[tone], 16)}, ${C.panel})`,
        maxWidth: 520,
        whiteSpace: 'pre-wrap',
        fontSize: 15,
        fontWeight: 600,
        opacity: Math.min(1, enter),
        transform: `translateY(${(1 - Math.min(1, enter)) * 30}px)`,
        zIndex: 50,
      }}
    >
      {text}
    </div>
  );

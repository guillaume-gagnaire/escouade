import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { C, soft, UI } from '../theme';
import { Logo } from './Logo';
import { blink } from './Shell';

/** Windows' taskbar at the bottom of the stage; `flash` makes Escouade's button blink, `badge` puts a count on its tray icon. */
export const Taskbar: FC<{ enter: number; flash?: boolean; badge?: number; active?: boolean }> = ({ enter, flash, badge, active }) => {
  const frame = useCurrentFrame();
  const f = flash ? blink(frame * 1.4) : 0;
  if (enter <= 0) return null;
  const tile = (child: ReactNode, bg = 'transparent', key?: string) => (
    <span
      key={key}
      style={{
        width: 44,
        height: 44,
        borderRadius: 6,
        background: bg,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        position: 'relative',
      }}
    >
      {child}
    </span>
  );
  return (
    <div
      style={{
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: 0,
        height: 56,
        background: 'rgba(32, 32, 32, 0.96)',
        borderTop: '1px solid rgba(255,255,255,0.08)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 6,
        transform: `translateY(${(1 - Math.min(1, enter)) * 60}px)`,
        fontFamily: UI,
        zIndex: 65,
      }}
    >
      {tile(
        <svg width="20" height="20" viewBox="0 0 20 20">
          <rect x="0" y="0" width="9" height="9" fill="#4cc2ff" />
          <rect x="11" y="0" width="9" height="9" fill="#4cc2ff" />
          <rect x="0" y="11" width="9" height="9" fill="#4cc2ff" />
          <rect x="11" y="11" width="9" height="9" fill="#4cc2ff" />
        </svg>,
      )}
      {tile(<span style={{ width: 20, height: 16, borderRadius: 3, background: '#f2c94c' }} />)}
      {tile(
        <span
          style={{ width: 20, height: 20, borderRadius: '50%', background: 'conic-gradient(#e94235, #fbbc05, #34a853, #4285f4, #e94235)' }}
        />,
      )}
      {tile(<span style={{ width: 20, height: 20, borderRadius: 4, background: '#2f80ed' }} />)}
      {tile(
        <>
          <Logo size={26} />
          <span
            style={{
              position: 'absolute',
              bottom: 2,
              width: active || flash ? 16 : 6,
              height: 3,
              borderRadius: 2,
              background: flash ? C.spark : '#9a9a9a',
            }}
          />
        </>,
        flash ? soft(C.spark, 55 * f) : 'transparent',
        'escouade',
      )}
      <div style={{ position: 'absolute', right: 18, display: 'flex', alignItems: 'center', gap: 16, color: '#ffffff', fontSize: 13 }}>
        <span style={{ position: 'relative', display: 'flex' }}>
          <Logo size={18} />
          {badge ? (
            <span
              style={{
                position: 'absolute',
                right: -8,
                top: -7,
                minWidth: 15,
                height: 15,
                borderRadius: 8,
                background: C.wait,
                color: '#1b1512',
                fontSize: 10,
                fontWeight: 800,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              {badge}
            </span>
          ) : null}
        </span>
        <span style={{ color: '#dddddd' }}>FR</span>
        <span style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', lineHeight: 1.25 }}>
          <span>14:32</span>
          <span>04/10/2026</span>
        </span>
      </div>
    </div>
  );
};

/** Another app's window, in front of Escouade. */
export const OtherWindow: FC<{ enter: number }> = ({ enter }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        left: 330,
        top: 250,
        width: 1150,
        height: 700,
        borderRadius: 10,
        background: '#f3f3f3',
        boxShadow: '0 40px 120px rgba(0, 0, 0, 0.6)',
        opacity: Math.min(1, enter),
        transform: `translateY(${(1 - Math.min(1, enter)) * 40}px)`,
        overflow: 'hidden',
        zIndex: 30,
        fontFamily: UI,
      }}
    >
      <div
        style={{
          height: 40,
          background: '#e6e6e6',
          display: 'flex',
          alignItems: 'center',
          padding: '0 16px',
          gap: 10,
          color: '#333',
          fontSize: 13,
        }}
      >
        <span
          style={{ width: 220, height: 26, borderRadius: 6, background: '#fff', display: 'flex', alignItems: 'center', padding: '0 10px' }}
        >
          Spécifications — v2
        </span>
      </div>
      <div style={{ padding: '40px 60px', display: 'flex', flexDirection: 'column', gap: 18 }}>
        <span style={{ width: 380, height: 26, borderRadius: 4, background: '#d0d0d0' }} />
        {[0.9, 0.95, 0.7, 0.85, 0.6, 0.92, 0.8].map((w, i) => (
          <span key={i} style={{ width: `${w * 100}%`, height: 12, borderRadius: 3, background: '#dcdcdc' }} />
        ))}
      </div>
    </div>
  );

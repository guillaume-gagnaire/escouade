import type { FC } from 'react';
import { useDemo } from '../data';
import { useFmt } from '../lang';
import { C, MONO, UI } from '../theme';

export interface PhoneMsg {
  from: 'me' | 'claude';
  text: string;
}

/** A phone on claude.ai, showing an agent's session. `draft` is being typed in its input. */
export const Phone: FC<{ messages: PhoneMsg[]; draft: string; x: number }> = ({ messages, draft, x }) => {
  const { tr } = useFmt();
  const { refactor } = useDemo();
  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: 170,
        width: 400,
        height: 830,
        borderRadius: 58,
        background: '#0d0c0b',
        padding: 14,
        boxShadow: '0 40px 120px rgba(0, 0, 0, 0.7), inset 0 0 0 2px #3a3632',
        fontFamily: UI,
      }}
    >
      <div
        style={{
          width: '100%',
          height: '100%',
          borderRadius: 46,
          background: '#262320',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        <div style={{ height: 50, display: 'flex', justifyContent: 'center', alignItems: 'center' }}>
          <span style={{ width: 110, height: 30, borderRadius: 16, background: '#0d0c0b' }} />
        </div>
        <div style={{ padding: '6px 22px 14px', borderBottom: `1px solid ${C.line}` }}>
          <div style={{ fontSize: 13, color: C.muted, fontFamily: MONO }}>claude.ai</div>
          <div style={{ fontSize: 18, fontWeight: 700, marginTop: 2 }}>{`demo-api · ${refactor}`}</div>
        </div>
        <div style={{ flex: 1, padding: '18px 18px', display: 'flex', flexDirection: 'column', gap: 12, justifyContent: 'flex-end' }}>
          {messages.map((m, i) => (
            <div
              key={i}
              style={{
                alignSelf: m.from === 'me' ? 'flex-end' : 'flex-start',
                maxWidth: '85%',
                padding: m.from === 'me' ? '10px 14px' : '2px 2px',
                borderRadius: 14,
                background: m.from === 'me' ? '#3a3531' : 'transparent',
                fontSize: 16,
                lineHeight: 1.45,
              }}
            >
              {m.text}
            </div>
          ))}
        </div>
        <div
          style={{
            margin: 14,
            padding: '12px 14px',
            borderRadius: 18,
            background: '#302c28',
            border: `1px solid ${C.line2}`,
            display: 'flex',
            alignItems: 'center',
            gap: 10,
          }}
        >
          <span style={{ flex: 1, fontSize: 16, color: draft ? C.text : C.dim }}>{draft || tr('Répondre…', 'Reply…')}</span>
          <span
            style={{
              width: 32,
              height: 32,
              borderRadius: 10,
              background: '#D97757',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: '#1b1512',
              fontWeight: 800,
            }}
          >
            ↑
          </span>
        </div>
      </div>
    </div>
  );
};

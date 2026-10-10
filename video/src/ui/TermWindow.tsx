import type { CSSProperties, FC } from 'react';
import { useFmt } from '../lang';
import { C, MONO } from '../theme';

/** A plain terminal running Claude Code, as they pile up without the app. */
export const TermWindow: FC<{ project: string; style?: CSSProperties }> = ({ project, style }) => {
  const { tr } = useFmt();
  return (
    <div
      style={{
        width: 460,
        height: 270,
        borderRadius: 10,
        overflow: 'hidden',
        background: '#0c0c0c',
        border: '1px solid #3a3a3a',
        boxShadow: '0 20px 60px rgba(0, 0, 0, 0.6)',
        fontFamily: MONO,
        fontSize: 13,
        color: '#cccccc',
        ...style,
      }}
    >
      <div
        style={{
          height: 32,
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '0 12px',
          background: '#1f1f1f',
          fontSize: 12,
          color: '#aaaaaa',
        }}
      >
        <span style={{ color: C.spark }}>▲</span> claude — {project}
        <span style={{ flex: 1 }} />— ☐ ✕
      </div>
      <div style={{ padding: '10px 14px', display: 'flex', flexDirection: 'column', gap: 4, whiteSpace: 'pre' }}>
        <span style={{ color: C.spark }}>✻ Claude Code</span>
        <span>
          {'>'}
          {tr(' corrige le test qui échoue', ' fix the failing test')}
        </span>
        <span style={{ color: '#9bd8a9' }}>● Read(src/app.ts)</span>
        <span style={{ color: '#9bd8a9' }}>● Bash(npm test)</span>
        <span style={{ color: '#888888' }}> ⎿ 41 passed, 1 failed</span>
        <span>{'>'} ▌</span>
      </div>
    </div>
  );
};

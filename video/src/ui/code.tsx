import type { FC } from 'react';
import { C, MONO } from '../theme';

/** Colours of the app's editor theme (lib/editor/theme.ts). */
export const SYN = {
  keyword: '#D97757',
  string: 'oklch(0.8 0.1 145)',
  number: 'oklch(0.82 0.11 75)',
  comment: '#6f685f',
  fn: 'oklch(0.87 0.08 95)',
  type: 'oklch(0.82 0.08 215)',
  plain: '#ede7df',
};

const KEYWORDS = new Set(
  'import export from const let return async await function if else new type interface throw for of in await default'.split(' '),
);

/** TypeScript-ish tokens of a line, coloured. */
export function tokens(line: string): { text: string; color: string }[] {
  const out: { text: string; color: string }[] = [];
  const re = /(\/\/.*$)|('[^']*'|"[^"]*"|`[^`]*`)|(\b\d+(?:\.\d+)?\b)|([A-Za-z_$][\w$]*)|(\s+)|(.)/g;
  for (const m of line.matchAll(re)) {
    const [t, comment, str, num, word] = m;
    let color = SYN.plain;
    if (comment) color = SYN.comment;
    else if (str) color = SYN.string;
    else if (num) color = SYN.number;
    else if (word) {
      const next = line[(m.index ?? 0) + t.length];
      color = KEYWORDS.has(word) ? SYN.keyword : /^[A-Z]/.test(word) ? SYN.type : next === '(' ? SYN.fn : SYN.plain;
    }
    out.push({ text: t, color });
  }
  return out;
}

export const CodeLine: FC<{ line: string }> = ({ line }) => (
  <>
    {tokens(line).map((t, i) => (
      <span key={i} style={{ color: t.color }}>
        {t.text}
      </span>
    ))}
  </>
);

/** A code block of the conversation's markdown. */
export const CodeBlock: FC<{ lines: string[]; lang?: string; shown?: number }> = ({ lines, lang = 'ts', shown = lines.length }) => (
  <div style={{ borderRadius: 8, border: `1px solid ${C.line}`, background: C.term, overflow: 'hidden' }}>
    <div style={{ padding: '5px 12px', fontFamily: MONO, fontSize: 11.5, color: C.dim, borderBottom: `1px solid ${C.line}` }}>{lang}</div>
    <div style={{ padding: '10px 14px', fontFamily: MONO, fontSize: 14, lineHeight: 1.6, whiteSpace: 'pre' }}>
      {lines.slice(0, shown).map((l, i) => (
        <div key={i}>
          <CodeLine line={l} />
          {l ? null : ' '}
        </div>
      ))}
    </div>
  </div>
);

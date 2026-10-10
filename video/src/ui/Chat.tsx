import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { useFmt } from '../lang';
import { C, MONO, soft } from '../theme';
import { Cursor } from './Cursor';
import { Dot } from './Shell';
import { Button } from './Sidebar';

/** Text with `inline code` and **bold**. */
export const Rich: FC<{ text: string }> = ({ text }) => (
  <>
    {text.split(/(`[^`]*`|\*\*[^*]*\*\*)/).map((part, i) =>
      part.startsWith('`') ? (
        <code key={i} style={{ fontFamily: MONO, fontSize: '0.86em', background: C.elev2, padding: '1px 6px', borderRadius: 4 }}>
          {part.slice(1, -1)}
        </code>
      ) : part.startsWith('**') ? (
        <b key={i}>{part.slice(2, -2)}</b>
      ) : (
        <span key={i}>{part}</span>
      ),
    )}
  </>
);

const Metric: FC<{ k: string; v: ReactNode; box?: boolean }> = ({ k, v, box }) => (
  <div
    style={{
      display: 'flex',
      flexDirection: 'column',
      gap: 2,
      padding: box ? '3px 8px' : 0,
      margin: box ? '-3px -8px' : 0,
      borderRadius: 6,
      border: box ? `1px solid ${C.line2}` : 'none',
    }}
  >
    <span style={{ fontSize: 11.5, color: C.dim }}>{k}</span>
    <span style={{ fontFamily: MONO, fontSize: 13.5 }}>{v}</span>
  </div>
);

export interface HeadMetrics {
  model: string;
  context?: string;
  tokens: string;
  cost: string;
  files: number;
  duration: string;
}

const Layout: FC<{ split: boolean }> = ({ split }) => (
  <div style={{ display: 'flex', gap: 2, padding: 2, borderRadius: 6, background: C.bg, border: `1px solid ${C.line}` }}>
    {[false, true].map((s) => (
      <span
        key={String(s)}
        style={{
          width: 30,
          height: 24,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderRadius: 4,
          background: s === split ? C.elev2 : 'transparent',
        }}
      >
        <svg width="16" height="12" viewBox="0 0 16 12">
          <rect x="0.5" y="0.5" width="15" height="11" rx="1.5" fill="none" stroke={s === split ? C.text : C.dim} />
          <rect x={s ? 8 : 11} y="1" width={s ? 7 : 4} height="10" fill={s === split ? C.text : C.dim} opacity="0.45" />
        </svg>
      </span>
    ))}
  </div>
);

export const ConvHeader: FC<{
  name: string;
  status: 'running' | 'waiting' | 'done' | 'idle';
  sub: string;
  metrics?: HeadMetrics;
  test?: 'test' | 'prepare';
  split?: boolean;
  compact?: boolean;
  pressed?: { editor?: number; test?: number };
}> = ({ name, status, sub, metrics, test, split = false, compact, pressed }) => {
  const { tr, tok } = useFmt();
  return (
    <div
      style={{
        height: 64,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '0 24px',
        borderBottom: `1px solid ${C.line}`,
        whiteSpace: 'nowrap',
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 3, minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <span style={{ fontSize: 17, fontWeight: 700 }}>{name}</span>
          <span
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              fontSize: 13,
              color: status === 'waiting' ? C.wait : status === 'idle' ? C.dim : C.ok,
            }}
          >
            <Dot
              color={status === 'waiting' ? C.wait : status === 'idle' ? C.dim : C.ok}
              size={7}
              pulse={status === 'running' || status === 'waiting'}
            />
            {status === 'running'
              ? tr('En cours', 'Running')
              : status === 'waiting'
                ? 'Question'
                : status === 'idle'
                  ? tr('Prêt', 'Ready')
                  : tr('Terminé', 'Done')}
          </span>
        </div>
        <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>{sub}</span>
      </div>
      <div style={{ flex: 1 }} />
      {metrics ? (
        <>
          <Button pressed={pressed?.editor}>
            <span style={{ fontFamily: MONO, fontSize: 12, color: C.spark }}>{'</>'}</span>
            {compact ? null : tr('Éditeur', 'Editor')}
          </Button>
          {test ? (
            <Button pressed={pressed?.test}>
              <span style={{ fontSize: 11, color: C.spark }}>{test === 'test' ? '▶' : '▷'}</span>
              {compact ? null : test === 'test' ? tr('Tester', 'Test') : tr('Préparer le lancement', 'Prepare launch')}
            </Button>
          ) : null}
          <span style={{ fontFamily: MONO, fontSize: 12, padding: '4px 8px', borderRadius: 6, border: `1px solid ${C.line2}` }}>
            {metrics.model}
          </span>
          {compact ? null : (
            <>
              <Metric k={tr('Contexte', 'Context')} v={metrics.context ?? `${tok(76.2, 1)} / ${tok(200)}`} />
              <Metric k="Tokens" v={metrics.tokens} />
            </>
          )}
          <Metric k={tr('Coût', 'Cost')} v={metrics.cost} />
          <Metric k={split ? tr('Fichiers', 'Files') : tr('Fichiers ▸', 'Files ▸')} v={metrics.files} />
          {compact ? null : <Metric k={tr('Durée', 'Duration')} v={metrics.duration} />}
          <Layout split={split} />
        </>
      ) : null}
    </div>
  );
};

export const Conversation: FC<{ children: ReactNode; gap?: number; offset?: number }> = ({ children, gap = 14, offset = 0 }) => (
  <div style={{ flex: 1, minHeight: 0, overflow: 'hidden', position: 'relative' }}>
    <div
      style={{
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: offset,
        display: 'flex',
        flexDirection: 'column',
        gap,
        padding: '22px 36px',
      }}
    >
      {children}
    </div>
  </div>
);

/** Appearance of a message: fades and slides up. */
export const enterStyle = (e: number) => ({ opacity: Math.min(1, e), transform: `translateY(${(1 - Math.min(1, e)) * 16}px)` });

export const UserMsg: FC<{ text: string; tag?: string; enter?: number; children?: ReactNode }> = ({ text, tag, enter = 1, children }) =>
  enter <= 0 ? null : (
    <div
      style={{
        alignSelf: 'flex-end',
        maxWidth: '72%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'flex-end',
        gap: 5,
        ...enterStyle(enter),
      }}
    >
      {children}
      <div style={{ background: C.user, borderRadius: 14, padding: '11px 16px', fontSize: 17, lineHeight: 1.45 }}>
        <Rich text={text} />
      </div>
      {tag ? <span style={{ fontSize: 12.5, color: C.info }}>{tag}</span> : null}
    </div>
  );

/** Claude's text, after its avatar; `children` follow the text (a code block, a card). */
export const AssistantMsg: FC<{ text?: string; avatar?: boolean; enter?: number; children?: ReactNode }> = ({
  text,
  avatar = true,
  enter = 1,
  children,
}) =>
  (text || children) && enter > 0 ? (
    <div style={{ display: 'flex', gap: 12, ...enterStyle(enter) }}>
      <span
        style={{
          width: 24,
          height: 24,
          flex: 'none',
          marginTop: 2,
          borderRadius: 6,
          background: avatar ? C.spark : 'transparent',
          color: '#1b1512',
          fontFamily: MONO,
          fontSize: 12,
          fontWeight: 700,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        {avatar ? 'C' : ''}
      </span>
      <div
        style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 10, fontSize: 17, lineHeight: 1.55, maxWidth: 820 }}
      >
        {text ? (
          <div>
            <Rich text={text} />
          </div>
        ) : null}
        {children}
      </div>
    </div>
  ) : null;

export const Diffstat: FC<{ add: number; del: number }> = ({ add, del }) => (
  <span style={{ fontFamily: MONO, fontSize: 13 }}>
    <span style={{ color: C.ok }}>+{add}</span> <span style={{ color: C.del }}>−{del}</span>
  </span>
);

export const Spinner: FC = () => {
  const frame = useCurrentFrame();
  return (
    <span
      style={{
        width: 13,
        height: 13,
        flex: 'none',
        borderRadius: '50%',
        border: `2px solid ${C.elev2}`,
        borderTopColor: C.spark,
        transform: `rotate(${frame * 14}deg)`,
      }}
    />
  );
};

/** A tool call, compact; `detail` unfolds under it. */
export const ToolCall: FC<{
  tool: string;
  target: string;
  meta?: ReactNode;
  running?: boolean;
  enter?: number;
  detail?: ReactNode;
  link?: boolean;
}> = ({ tool, target, meta, running, enter = 1, detail, link }) =>
  enter <= 0 ? null : (
    <div
      style={{
        marginLeft: 36,
        borderRadius: 8,
        border: `1px solid ${C.line}`,
        background: soft(C.elev, 60),
        overflow: 'hidden',
        ...enterStyle(enter),
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 40, padding: '0 14px', fontSize: 14 }}>
        <span style={{ fontFamily: MONO, color: C.dim, width: 10 }}>{detail ? '▾' : '▸'}</span>
        <span style={{ padding: '2px 7px', borderRadius: 4, background: C.elev2, fontWeight: 600, fontSize: 12.5 }}>{tool}</span>
        <span style={{ fontFamily: MONO, fontSize: 13, color: link ? C.spark : C.muted, textDecoration: link ? 'underline' : 'none' }}>
          {target}
        </span>
        <div style={{ flex: 1 }} />
        {running ? <Spinner /> : meta}
      </div>
      {detail ? <div style={{ borderTop: `1px solid ${C.line}` }}>{detail}</div> : null}
    </div>
  );

export interface PatchLine {
  kind: '+' | '-' | ' ';
  text: string;
  n: number;
}

/** A unified diff, as unfolded under an Edit. */
export const PatchView: FC<{ lines: PatchLine[]; shown?: number; size?: number }> = ({ lines, shown = lines.length, size = 13 }) => (
  <div style={{ fontFamily: MONO, fontSize: size, lineHeight: 1.7, padding: '6px 0', background: C.term }}>
    {lines.slice(0, shown).map((l, i) => (
      <div
        key={i}
        style={{
          display: 'flex',
          whiteSpace: 'pre',
          background: l.kind === '+' ? soft(C.ok, 14) : l.kind === '-' ? soft(C.del, 14) : 'transparent',
        }}
      >
        <span style={{ width: 44, textAlign: 'right', paddingRight: 10, color: C.dim, flex: 'none' }}>{l.n}</span>
        <span style={{ width: 16, color: l.kind === '+' ? C.ok : l.kind === '-' ? C.del : C.dim, flex: 'none' }}>{l.kind}</span>
        <span>{l.text}</span>
      </div>
    ))}
  </div>
);

/** A command's output, as unfolded under a Bash call. */
export const Output: FC<{ lines: string[] }> = ({ lines }) => (
  <div
    style={{
      fontFamily: MONO,
      fontSize: 13,
      lineHeight: 1.6,
      padding: '8px 16px',
      background: C.term,
      color: '#d8d0c4',
      whiteSpace: 'pre',
    }}
  >
    {lines.map((l, i) => (
      <div key={i} style={{ color: /passed|✓/.test(l) ? '#9bd8a9' : undefined }}>
        {l}
      </div>
    ))}
  </div>
);

export const Thinking: FC<{ seconds?: number; enter?: number }> = ({ enter = 1 }) => {
  const { tr } = useFmt();
  return enter <= 0 ? null : (
    <div style={{ marginLeft: 36, fontSize: 14, color: C.dim, display: 'flex', gap: 6, ...enterStyle(enter) }}>
      <span>▸</span>
      {tr(' Réflexion', ' Thinking')}
    </div>
  );
};

const Pulse: FC = () => <Dot color={C.wait} size={8} pulse />;

/** A card asking you: Claude's question, its options; `cursor` moves the pointer onto option `target` (t: 0 to 1), then clicks. */
const AskCard: FC<{
  title: string;
  children: ReactNode;
  options: string[];
  picked?: number | null;
  answered?: string;
  enter?: number;
  cursor?: { target: number; t: number; click: number };
}> = ({ title, children, options, picked = null, answered, enter = 1, cursor }) =>
  enter <= 0 ? null : (
    <div
      style={{
        position: 'relative',
        marginLeft: 36,
        borderRadius: 12,
        border: `1px solid ${picked === null ? soft(C.wait, 60) : C.line}`,
        background: picked === null ? soft(C.wait, 7) : C.panel,
        padding: 18,
        display: 'flex',
        flexDirection: 'column',
        gap: 10,
        ...enterStyle(enter),
        opacity: (picked === null ? 1 : 0.75) * Math.min(1, enter),
      }}
    >
      <span
        style={{ display: 'flex', alignItems: 'center', gap: 9, fontSize: 13, fontWeight: 700, color: picked === null ? C.wait : C.muted }}
      >
        {picked === null ? <Pulse /> : null}
        {picked === null ? title : (answered ?? `→ ${options[picked]}`)}
      </span>
      {children}
      <div style={{ display: 'flex', gap: 10, marginTop: 4 }}>
        {options.map((o, i) => (
          <span
            key={o}
            style={{
              minWidth: 110,
              height: 40,
              padding: '0 16px',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: 8,
              border: `1px solid ${picked === i ? C.spark : C.line2}`,
              background: picked === i ? C.spark : cursor && cursor.target === i && cursor.t >= 1 ? C.elev2 : C.elev,
              color: picked === i ? '#1b1512' : C.text,
              fontSize: 15,
              fontWeight: 600,
            }}
          >
            {o}
          </span>
        ))}
      </div>
      {cursor ? (
        <Cursor x={18 + cursor.target * 122 + 64 + (1 - cursor.t) * 320} y={126 + (1 - cursor.t) * 140} click={cursor.click} />
      ) : null}
    </div>
  );

export const QuestionCard: FC<{
  question: string;
  options: string[];
  picked?: number | null;
  enter?: number;
  cursor?: { target: number; t: number; click: number };
}> = ({ question, ...rest }) => {
  const { tr } = useFmt();
  return (
    <AskCard title={tr('Claude attend ta réponse', 'Claude is waiting for your answer')} {...rest}>
      <span style={{ fontSize: 18, fontWeight: 600 }}>{question}</span>
    </AskCard>
  );
};

export const PermissionCard: FC<{
  tool: string;
  command: string;
  picked?: number | null;
  enter?: number;
  cursor?: { target: number; t: number; click: number };
}> = ({ tool, command, picked = null, ...rest }) => {
  const { tr } = useFmt();
  return (
    <AskCard
      title={tr('Claude demande une autorisation', 'Claude is asking for permission')}
      options={[tr('Autoriser', 'Allow'), tr('Toujours autoriser', 'Always allow'), tr('Refuser', 'Deny')]}
      picked={picked}
      answered={
        picked === 2
          ? tr('✕ Refusé', '✕ Denied')
          : picked === 1
            ? tr('✓ Toujours autorisé', '✓ Always allowed')
            : tr('✓ Autorisé', '✓ Allowed')
      }
      {...rest}
    >
      <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
        <span style={{ padding: '2px 7px', borderRadius: 4, background: C.elev2, fontWeight: 600, fontSize: 12.5 }}>{tool}</span>
        <span style={{ fontFamily: MONO, fontSize: 15 }}>{command}</span>
      </span>
    </AskCard>
  );
};

/** « Claude travaille… » under the last message. */
export const Working: FC<{ label?: string }> = ({ label }) => {
  const frame = useCurrentFrame();
  const { tr } = useFmt();
  return (
    <div style={{ marginLeft: 36, display: 'flex', alignItems: 'center', gap: 10, fontSize: 15, color: C.muted }}>
      <span style={{ display: 'inline-flex', gap: 4 }}>
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            style={{
              width: 6,
              height: 6,
              borderRadius: '50%',
              background: C.spark,
              opacity: 0.25 + 0.75 * Math.max(0, Math.sin(((frame - i * 4) / 36) * Math.PI * 2)),
            }}
          />
        ))}
      </span>
      {label ?? tr('Claude travaille…', 'Claude is working…')}
    </div>
  );
};

/** The card at the end of a task. */
export const TurnCard: FC<{
  duration: string;
  tokens: string;
  cost: string;
  files: { path: string; add: number; del: number }[];
  enter?: number;
  pressed?: { review?: number; commit?: number };
}> = ({ duration, tokens, cost, files, enter = 1, pressed }) => {
  const { tr, trx, plural } = useFmt();
  return enter <= 0 ? null : (
    <div
      style={{
        marginLeft: 36,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
        padding: '16px 18px',
        borderRadius: 10,
        border: `1px solid ${C.line2}`,
        background: C.panel,
        ...enterStyle(enter),
      }}
    >
      <span style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 14, fontWeight: 700, color: C.ok }}>
        {tr('✓ Tâche terminée', '✓ Task complete')}
      </span>
      <span style={{ display: 'flex', gap: 18, fontFamily: MONO, fontSize: 13.5, color: C.muted }}>
        <span>{duration}</span>
        <span>{tokens} tokens</span>
        <span>{cost}</span>
        <span>
          {trx(
            <>
              {files.length} fichier{files.length > 1 ? 's' : ''} modifié{files.length > 1 ? 's' : ''}
            </>,
            `${files.length} modified ${plural(files.length, '', '', 'file', 'files')}`,
          )}
        </span>
      </span>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 4, fontFamily: MONO, fontSize: 13.5 }}>
        {files.map((f) => (
          <span key={f.path} style={{ display: 'flex', gap: 10 }}>
            <span style={{ color: C.spark }}>{f.path}</span>
            <span style={{ color: C.ok }}>+{f.add}</span>
            <span style={{ color: C.del }}>−{f.del}</span>
          </span>
        ))}
      </div>
      <div style={{ display: 'flex', gap: 8 }}>
        <Button pressed={pressed?.review}>{tr('Revoir les fichiers', 'Review files')}</Button>
        <Button pressed={pressed?.commit}>Commit…</Button>
      </div>
    </div>
  );
};

export interface Criterion {
  text: string;
  ok: boolean;
  note?: string;
}

/** An agent's report, as the conversation shows its `escouade` block. */
export const CriteriaReport: FC<{
  criteria: Criterion[];
  progress?: string[];
  recipe?: { prepare: string[]; processes: { name: string; command: string; url?: string }[] };
  enter?: number;
}> = ({ criteria, progress, recipe, enter = 1 }) => {
  const { tr } = useFmt();
  const met = criteria.filter((c) => c.ok).length;
  const head = (t: string, n?: string) => (
    <span style={{ display: 'flex', gap: 8, fontSize: 14, fontWeight: 700 }}>
      {t}
      {n ? <span style={{ fontFamily: MONO, fontSize: 13, color: C.dim, fontWeight: 400 }}>{n}</span> : null}
    </span>
  );
  return enter <= 0 ? null : (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 8,
        padding: '14px 16px',
        borderRadius: 10,
        border: `1px solid ${C.line2}`,
        background: C.elev,
        ...enterStyle(enter),
      }}
    >
      {head(tr('Bilan des critères', 'Criteria report'), `${met}/${criteria.length}`)}
      {criteria.map((c) => (
        <span key={c.text} style={{ display: 'flex', flexWrap: 'wrap', gap: 8, fontSize: 15, lineHeight: 1.45 }}>
          <span style={{ width: 14, color: c.ok ? C.ok : C.dim }}>{c.ok ? '✓' : '○'}</span>
          <span>{c.text}</span>
          {c.note ? <span style={{ color: C.muted, fontSize: 14 }}>— {c.note}</span> : null}
        </span>
      ))}
      {progress ? (
        <>
          {head(tr('Ce qui a été fait', 'What was done'))}
          {progress.map((p) => (
            <span key={p} style={{ display: 'flex', gap: 8, fontSize: 15, color: C.muted }}>
              <span style={{ width: 14, textAlign: 'center', color: C.dim }}>·</span>
              {p}
            </span>
          ))}
        </>
      ) : null}
      {recipe ? (
        <>
          {head(tr('Lancement de test', 'Test launch'))}
          {recipe.prepare.map((p) => (
            <span key={p} style={{ display: 'flex', gap: 10, fontFamily: MONO, fontSize: 13 }}>
              <b style={{ fontWeight: 600 }}>{tr('Préparation', 'Setup')}</b>
              <span style={{ color: C.muted }}>{p}</span>
            </span>
          ))}
          {recipe.processes.map((p) => (
            <span key={p.name} style={{ display: 'flex', gap: 10, fontFamily: MONO, fontSize: 13 }}>
              <b style={{ fontWeight: 600 }}>{p.name}</b>
              <span style={{ color: C.muted }}>{p.command}</span>
              {p.url ? <span style={{ color: C.muted }}>{p.url}</span> : null}
            </span>
          ))}
          <span style={{ alignSelf: 'flex-start' }}>
            <Button>{tr('▶ Tester', '▶ Test')}</Button>
          </span>
        </>
      ) : null}
    </div>
  );
};

/** The line ending a short or interrupted turn. */
export const TurnSep: FC<{ text: string; enter?: number }> = ({ text, enter = 1 }) =>
  enter <= 0 ? null : (
    <div
      style={{
        marginLeft: 36,
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        fontFamily: MONO,
        fontSize: 13,
        color: C.dim,
        ...enterStyle(enter),
      }}
    >
      <span style={{ flex: 1, height: 1, background: C.line2 }} />
      {text}
      <span style={{ flex: 1, height: 1, background: C.line2 }} />
    </div>
  );

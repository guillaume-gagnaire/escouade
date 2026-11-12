import type { FC, ReactNode } from 'react';
import { useFmt, type Tr } from '../lang';
import { C, MONO, soft } from '../theme';
import { Icon } from './icons';
import { Kbd } from './Shell';

export interface MenuOption {
  label: string;
  detail?: string;
}

/** A composer picker; `options` open its menu upwards, `pick` highlighting one. */
const Dropdown: FC<{ caption: string; value: string; options?: MenuOption[]; pick?: number; current?: number; flash?: number }> = ({
  caption,
  value,
  options,
  pick,
  current,
  flash = 0,
}) => (
  <div style={{ position: 'relative' }}>
    <span
      style={{
        height: 30,
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '0 9px',
        borderRadius: 7,
        background: options ? 'rgba(255, 255, 255, 0.07)' : flash ? soft(C.spark, 22 * flash) : 'transparent',
        fontSize: 13.5,
        fontWeight: 600,
        whiteSpace: 'nowrap',
      }}
    >
      <span style={{ fontFamily: 'inherit', color: C.dim, fontWeight: 500 }}>{caption}</span>
      {value}
      <span style={{ color: C.muted }}>
        <Icon name="chevron" size={12} />
      </span>
    </span>
    {options ? (
      <div
        style={{
          position: 'absolute',
          left: 0,
          bottom: 'calc(100% + 6px)',
          minWidth: 230,
          display: 'flex',
          flexDirection: 'column',
          padding: 5,
          borderRadius: 10,
          background: C.elev2,
          boxShadow: `inset 0 0 0 1px ${C.line2}, 0 18px 50px rgba(0, 0, 0, 0.55)`,
          zIndex: 30,
        }}
      >
        {options.map((o, i) => (
          <span
            key={o.label}
            style={{
              display: 'flex',
              alignItems: 'baseline',
              gap: 10,
              padding: '8px 10px',
              borderRadius: 6,
              background: i === pick ? 'rgba(255, 255, 255, 0.07)' : 'transparent',
              fontSize: 14,
              whiteSpace: 'nowrap',
            }}
          >
            <span style={{ width: 12, color: C.text }}>{i === current ? '✓' : ''}</span>
            <span style={{ fontWeight: 600 }}>{o.label}</span>
            {o.detail ? <span style={{ fontSize: 12.5, color: C.dim }}>{o.detail}</span> : null}
          </span>
        ))}
      </div>
    ) : null}
  </div>
);

export interface Attachment {
  name: string;
  kind: 'image' | 'pdf' | 'text';
}

/** Suggestions above the field: `@` files or `/` commands. */
export const Suggestions: FC<{ items: { label: string; detail: string }[]; selected?: number }> = ({ items, selected = 0 }) => (
  <div
    style={{
      position: 'absolute',
      left: 0,
      right: 0,
      bottom: 'calc(100% + 8px)',
      padding: 5,
      borderRadius: 10,
      background: C.elev,
      border: `1px solid ${C.line2}`,
      boxShadow: '0 18px 50px rgba(0, 0, 0, 0.55)',
      display: 'flex',
      flexDirection: 'column',
      zIndex: 30,
    }}
  >
    {items.map((s, i) => (
      <span
        key={s.label}
        style={{
          display: 'flex',
          gap: 14,
          alignItems: 'baseline',
          padding: '8px 12px',
          borderRadius: 6,
          background: i === selected ? C.elev2 : 'transparent',
        }}
      >
        <span style={{ fontFamily: MONO, fontSize: 14, fontWeight: 600 }}>{s.label}</span>
        <span style={{ fontSize: 13, color: C.dim }}>{s.detail}</span>
      </span>
    ))}
  </div>
);

export type Menu = 'model' | 'effort' | 'mode';

const modelsOf = (tr: Tr): MenuOption[] => [
  { label: 'Fable 5.1', detail: tr('le plus capable', 'most capable') },
  { label: 'Opus 5.5', detail: tr('raisonnement profond', 'deep reasoning') },
  { label: 'Sonnet 5.5', detail: tr('rapide et précis', 'fast and accurate') },
  { label: 'Haiku 4.5', detail: tr('le plus rapide', 'fastest') },
];
const effortsOf = (tr: Tr): MenuOption[] => [
  { label: tr('Bas', 'Low'), detail: tr('Réponses rapides', 'Quick answers') },
  { label: tr('Moyen', 'Medium'), detail: tr('Équilibré', 'Balanced') },
  { label: tr('Élevé', 'High'), detail: tr('Réflexion approfondie', 'Deep thinking') },
  { label: tr('Très élevé', 'Very high'), detail: tr('Réflexion très approfondie', 'Very deep thinking') },
  { label: 'Max', detail: tr('Réflexion maximale', 'Maximum thinking') },
];
const modesOf = (tr: Tr): MenuOption[] => [
  { label: 'Auto', detail: tr('approuve les actions sûres', 'approves safe actions') },
  { label: tr('Demander', 'Ask'), detail: tr('ton accord avant chaque action', 'your approval before every action') },
  { label: 'Plan', detail: tr('propose un plan, sans modifier', 'proposes a plan, changes nothing') },
  { label: tr('Édits auto', 'Auto edits'), detail: tr('accepte les modifications', 'accepts edits') },
  { label: 'Bypass', detail: tr('aucune demande', 'no prompts') },
];

/** The field at the bottom of a conversation, with its pickers. */
export const Composer: FC<{
  text?: string;
  placeholder?: string;
  model?: string;
  effort?: string;
  mode?: string;
  /** The open picker, its highlighted option and the current one. */
  menu?: { which: Menu; pick: number; current: number };
  flash?: Partial<Record<Menu, number>>;
  attachments?: Attachment[];
  suggestions?: ReactNode;
  busy?: boolean;
  /** Half a panel wide (the split view): the pickers without their captions, no attach, dictation or voice. */
  narrow?: boolean;
  waiting?: boolean;
}> = ({
  text = '',
  placeholder,
  model = 'Opus 5.5',
  effort,
  mode = 'Auto',
  menu,
  flash = {},
  attachments = [],
  suggestions,
  busy,
  narrow,
  waiting,
}) => {
  const { tr } = useFmt();
  return (
    <div style={{ flex: 'none', padding: '6px 36px 20px' }}>
      <div
        style={{
          position: 'relative',
          borderRadius: 12,
          boxShadow: `inset 0 0 0 1px ${waiting ? C.wait : text ? C.pInk : C.line2}`,
          background: C.elev,
          padding: '12px 10px 8px 14px',
          display: 'flex',
          flexDirection: 'column',
          gap: 10,
        }}
      >
        {suggestions}
        {attachments.length ? (
          <div style={{ display: 'flex', gap: 8 }}>
            {attachments.map((a) =>
              a.kind === 'image' ? (
                <span
                  key={a.name}
                  style={{
                    width: 64,
                    height: 46,
                    borderRadius: 6,
                    border: `1px solid ${C.line2}`,
                    background: `linear-gradient(135deg, ${soft(C.info, 60)}, ${soft(C.spark, 50)})`,
                    position: 'relative',
                  }}
                >
                  <span
                    style={{
                      position: 'absolute',
                      left: 8,
                      top: 8,
                      width: 26,
                      height: 5,
                      borderRadius: 2,
                      background: 'rgba(255,255,255,0.6)',
                    }}
                  />
                  <span
                    style={{
                      position: 'absolute',
                      left: 8,
                      top: 18,
                      width: 42,
                      height: 16,
                      borderRadius: 3,
                      background: 'rgba(0,0,0,0.25)',
                    }}
                  />
                </span>
              ) : (
                <span
                  key={a.name}
                  style={{
                    height: 46,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '0 12px',
                    borderRadius: 6,
                    border: `1px solid ${C.line2}`,
                    background: C.panel,
                    fontSize: 13.5,
                  }}
                >
                  <span style={{ fontFamily: MONO, fontSize: 11, fontWeight: 700, color: a.kind === 'pdf' ? C.del : C.info }}>
                    {a.kind === 'pdf' ? 'PDF' : 'TXT'}
                  </span>
                  {a.name}
                  <span style={{ color: C.dim }}>×</span>
                </span>
              ),
            )}
          </div>
        ) : null}
        <div style={{ minHeight: 26, fontSize: 16, lineHeight: 1.5, color: text ? C.text : C.muted }}>
          {text ? <TextWithMentions text={text} /> : (placeholder ?? tr('Envoyer un message…', 'Send a message…'))}
          {text ? (
            <span style={{ display: 'inline-block', width: 2, height: 19, background: C.text, marginLeft: 1, verticalAlign: -3 }} />
          ) : null}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <Dropdown
            caption={narrow ? '' : tr('Modèle', 'Model')}
            value={model}
            flash={flash.model}
            {...(menu?.which === 'model' ? { options: modelsOf(tr), pick: menu.pick, current: menu.current } : {})}
          />
          <Dropdown
            caption={narrow ? '' : 'Effort'}
            value={effort ?? tr('Élevé', 'High')}
            flash={flash.effort}
            {...(menu?.which === 'effort' ? { options: effortsOf(tr), pick: menu.pick, current: menu.current } : {})}
          />
          <Dropdown
            caption={narrow ? '' : 'Mode'}
            value={mode}
            flash={flash.mode}
            {...(menu?.which === 'mode' ? { options: modesOf(tr), pick: menu.pick, current: menu.current } : {})}
          />
          <div style={{ flex: 1 }} />
          {narrow ? null : (
            <span style={{ display: 'flex', gap: 16, padding: '0 8px', color: C.muted }}>
              <Icon name="clip" size={15} />
              <Icon name="mic" size={15} />
              <svg width="15" height="15" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
                <path d="M2 7v2M5 5v6M8 3v10M11 5v6M14 7v2" />
              </svg>
            </span>
          )}
          {narrow ? null : <span style={{ width: 1, height: 20, background: C.line2 }} />}
          {busy ? (
            <span
              style={{
                height: 32,
                padding: '0 8px 0 12px',
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                borderRadius: 8,
                boxShadow: `inset 0 0 0 1px ${C.line2}`,
                fontSize: 14,
                fontWeight: 600,
              }}
            >
              <span style={{ fontSize: 9 }}>■</span> Stop <Kbd>{tr('Échap', 'Esc')}</Kbd>
            </span>
          ) : null}
          <span
            style={{
              height: 32,
              padding: '0 8px 0 13px',
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              whiteSpace: 'nowrap',
              borderRadius: 8,
              background: C.primary,
              color: C.onPrimary,
              fontSize: 14,
              fontWeight: 600,
              opacity: text ? 1 : 0.55,
            }}
          >
            {tr('Envoyer', 'Send')}
            <Kbd style={{ color: '#4a4e55', boxShadow: 'inset 0 0 0 1px rgba(0, 0, 0, 0.18)' }}>↵</Kbd>
          </span>
        </div>
      </div>
    </div>
  );
};

/** `@file` and `/command` in the field, coloured. */
const TextWithMentions: FC<{ text: string }> = ({ text }) => (
  <>
    {text.split(/(@[\w./-]+|^\/[\w-]+)/).map((p, i) =>
      p.startsWith('@') || p.startsWith('/') ? (
        <span key={i} style={{ color: C.spark, fontFamily: MONO, fontSize: '0.92em' }}>
          {p}
        </span>
      ) : (
        <span key={i}>{p}</span>
      ),
    )}
  </>
);

import type { FC, ReactNode } from 'react';
import { useCurrentFrame } from 'remotion';
import { C, MONO, soft } from '../theme';
import { Chips, Field, Modal, Switch } from './Modal';
import { Dot } from './Shell';
import { Button, STATUS_COLOR, type AgentStatus } from './Sidebar';

export type Column = 'todo' | 'doing' | 'review' | 'done';

export const COLUMNS: { id: Column; label: string; color: string; empty: string }[] = [
  { id: 'todo', label: 'À faire', color: C.dim, empty: "Ajoute un ticket : un agent le prendra dès qu'une place se libère." },
  { id: 'doing', label: 'En cours', color: C.spark, empty: 'Aucun agent en boucle' },
  { id: 'review', label: 'À tester', color: C.wait, empty: 'Rien à tester' },
  { id: 'done', label: 'Terminé', color: C.ok, empty: 'Aucun ticket terminé' },
];

export interface Ticket {
  key: string;
  title: string;
  column: Column;
  loop?: [number, number];
  criteria?: { text: string; ok: boolean }[];
  progress?: string[];
  activity?: string;
  waiting?: boolean;
  /** « Pris dès qu'une place se libère »… */
  wait?: string;
  canStart?: boolean;
  meta?: string;
  agent?: { name: string; status: AgentStatus };
  outcome?: string;
  doneMeta?: string;
  blocked?: string;
  step?: string;
  partial?: boolean;
  test?: boolean;
  /** Its test launch runs. */
  testing?: boolean;
  approve?: string;
  enter?: number;
  /** The « Renvoyer » form, with what is typed in it. */
  rejecting?: string;
  pressed?: { approve?: number; reject?: number; test?: number; resume?: number };
}

export const BoardHeader: FC<{
  project: string;
  count: number;
  looping: number;
  places: string;
  summary: string;
  autopilot: boolean;
  pressedCfg?: number;
}> = ({ project, count, looping, places, summary, autopilot, pressedCfg = 0 }) => (
  <div
    style={{
      height: 64,
      flex: 'none',
      display: 'flex',
      alignItems: 'center',
      gap: 16,
      padding: '0 20px 0 24px',
      borderBottom: `1px solid ${C.line}`,
      whiteSpace: 'nowrap',
    }}
  >
    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      <span style={{ fontSize: 17, fontWeight: 700 }}>Tableau</span>
      <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>
        {project} · {count} tickets · {looping} en boucle
      </span>
    </div>
    <div style={{ flex: 1 }} />
    <span style={{ fontSize: 13.5, color: C.muted }}>{places}</span>
    <span
      style={{
        height: 36,
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '0 12px',
        borderRadius: 6,
        border: `1px solid ${pressedCfg ? C.spark : C.line2}`,
        background: pressedCfg ? soft(C.spark, 20 * pressedCfg) : C.elev,
        fontSize: 13,
      }}
    >
      <span style={{ color: C.muted }}>⚙</span>
      <span style={{ color: C.muted }}>Après validation :</span>
      <b style={{ fontFamily: MONO, fontSize: 12.5 }}>{summary}</b>
    </span>
    <span
      style={{
        height: 36,
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '0 8px 0 12px',
        borderRadius: 6,
        border: `1px solid ${C.line2}`,
        background: C.elev,
      }}
    >
      <span style={{ fontSize: 13.5, fontWeight: 600 }}>{autopilot ? 'Pilote auto' : 'Pilote auto · off'}</span>
      <Switch on={autopilot} />
    </span>
  </div>
);

export const BoardColumn: FC<{ column: Column; count: number; children?: ReactNode; glow?: number; plusPressed?: number }> = ({
  column,
  count,
  children,
  glow = 0,
  plusPressed = 0,
}) => {
  const c = COLUMNS.find((x) => x.id === column)!;
  return (
    <section
      style={{
        minHeight: 0,
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 10,
        background: C.panel,
        border: `1px solid ${glow ? soft(c.color, 40 + 60 * glow) : C.line}`,
        boxShadow: glow ? `0 0 30px ${soft(c.color, 30 * glow)}` : 'none',
        overflow: 'hidden',
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '12px 10px 10px 14px' }}>
        <span style={{ width: 9, height: 9, borderRadius: '50%', background: c.color }} />
        <span style={{ fontSize: 14.5, fontWeight: 700 }}>{c.label}</span>
        <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>{count}</span>
        <div style={{ flex: 1 }} />
        {column === 'todo' ? (
          <span
            style={{
              width: 28,
              height: 28,
              borderRadius: 6,
              border: `1px solid ${plusPressed ? C.spark : C.line2}`,
              background: plusPressed ? soft(C.spark, 30) : C.elev,
              color: C.spark,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: 17,
            }}
          >
            +
          </span>
        ) : null}
      </div>
      <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', gap: 8, padding: '0 8px 10px' }}>
        {children}
        {count === 0 && !children ? (
          <div
            style={{
              padding: '18px 12px',
              borderRadius: 6,
              border: `1px dashed ${C.line2}`,
              textAlign: 'center',
              fontSize: 12.5,
              color: C.dim,
            }}
          >
            {c.empty}
          </div>
        ) : null}
      </div>
    </section>
  );
};

const Dots: FC = () => {
  const frame = useCurrentFrame();
  return (
    <span style={{ display: 'inline-flex', gap: 3 }}>
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          style={{
            width: 4,
            height: 4,
            borderRadius: '50%',
            background: C.spark,
            opacity: 0.2 + 0.8 * Math.max(0, Math.sin(((frame - i * 4) / 36) * Math.PI * 2)),
          }}
        />
      ))}
    </span>
  );
};

const Small: FC<{ children: ReactNode; pressed?: number; grow?: boolean; primary?: string }> = ({
  children,
  pressed = 0,
  grow,
  primary,
}) => (
  <span
    style={{
      flex: grow ? 1 : 'none',
      height: grow ? 32 : 28,
      padding: '0 11px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: 6,
      border: primary ? 'none' : `1px solid ${pressed ? C.spark : C.line2}`,
      background: primary ? primary : pressed ? soft(C.spark, 30 * pressed) : 'transparent',
      color: primary ? '#0f1a12' : C.text,
      fontSize: 13,
      fontWeight: primary ? 700 : 600,
      whiteSpace: 'nowrap',
      transform: `scale(${1 - 0.05 * pressed})`,
    }}
  >
    {children}
  </span>
);

export const TicketCard: FC<{ t: Ticket }> = ({ t }) => {
  const e = t.enter ?? 1;
  if (e <= 0) return null;
  const met = t.criteria?.filter((c) => c.ok).length ?? 0;
  const total = t.criteria?.length ?? 0;
  const steps = t.column === 'doing' ? (t.progress ?? []).slice(-3) : (t.progress ?? []);
  const hidden = (t.progress?.length ?? 0) - steps.length;
  const list = (items: string[]) => (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
      {items.map((p) => (
        <span key={p} style={{ display: 'flex', gap: 7, fontSize: 12.5, lineHeight: 1.4, color: C.muted }}>
          <span style={{ width: 10, textAlign: 'center', color: C.dim }}>·</span>
          {p}
        </span>
      ))}
    </div>
  );
  return (
    <div
      style={{
        flex: 'none',
        display: 'flex',
        flexDirection: 'column',
        gap: 9,
        padding: '11px 12px',
        borderRadius: 6,
        background: C.elev,
        border: `1px solid ${t.waiting ? C.wait : C.line}`,
        opacity: (t.column === 'done' ? 0.75 : 1) * Math.min(1, e),
        transform: `translateY(${(1 - Math.min(1, e)) * 18}px)`,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>{t.key}</span>
        <div style={{ flex: 1 }} />
        {t.column === 'doing' && t.loop ? (
          <span style={{ fontFamily: MONO, fontSize: 11.5, fontWeight: 600, color: C.spark }}>
            Boucle {t.loop[0]}/{t.loop[1]}
          </span>
        ) : null}
        {t.partial ? (
          <span style={{ fontSize: 11, fontWeight: 700, padding: '2px 6px', borderRadius: 3, background: soft(C.wait, 12), color: C.wait }}>
            Objectif partiel
          </span>
        ) : null}
      </div>
      <span style={{ fontSize: 14.5, fontWeight: 600, lineHeight: 1.35 }}>{t.title}</span>
      {t.column === 'done' && t.outcome ? (
        <span
          style={{
            alignSelf: 'flex-start',
            fontFamily: MONO,
            fontSize: 11.5,
            padding: '3px 7px',
            borderRadius: 3,
            background: C.elev2,
            color: C.ok,
          }}
        >
          {t.outcome}
        </span>
      ) : null}
      {t.column === 'review' && steps.length ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <span style={{ fontSize: 11, fontWeight: 700, letterSpacing: '0.04em', textTransform: 'uppercase', color: C.dim }}>
            Ce qui a été fait
          </span>
          {list(steps)}
        </div>
      ) : null}
      {(t.column === 'doing' || t.column === 'review') && t.criteria ? (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          {t.criteria.map((c) => (
            <span key={c.text} style={{ display: 'flex', gap: 7, fontSize: 12.5, lineHeight: 1.4, color: c.ok ? C.muted : C.text }}>
              <span style={{ width: 10, flex: 'none', color: c.ok ? C.ok : C.dim }}>{c.ok ? '✓' : '○'}</span>
              {c.text}
            </span>
          ))}
        </div>
      ) : null}
      {t.column === 'doing' ? (
        <>
          <div style={{ height: 5, borderRadius: 3, background: C.elev2, overflow: 'hidden' }}>
            <div style={{ width: `${total ? (met / total) * 100 : 0}%`, height: '100%', background: C.spark }} />
          </div>
          {steps.length ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              {list(steps)}
              {hidden > 0 ? <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>+{hidden}</span> : null}
            </div>
          ) : null}
          {!t.blocked && t.waiting ? (
            <span style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12.5, color: C.wait }}>
              <Dot color={C.wait} size={7} pulse />
              Question en attente de ta réponse
            </span>
          ) : !t.blocked && t.activity ? (
            <span style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12.5, color: C.muted }}>
              <Dots />
              {t.activity}
            </span>
          ) : null}
        </>
      ) : null}
      {t.column === 'todo' ? (
        <>
          <span style={{ fontSize: 12.5, color: C.muted }}>{t.meta ?? `${total} critères · max ${t.loop?.[1] ?? 5} boucles`}</span>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontSize: 12, color: C.dim }}>{t.wait}</span>
            <div style={{ flex: 1 }} />
            {t.canStart ? <Small>Lancer</Small> : null}
          </div>
        </>
      ) : null}
      {t.testing && (t.column === 'review' || t.column === 'doing') ? (
        <div style={{ display: 'flex', gap: 6 }}>
          <Small>■ Arrêter</Small>
          <Small>Ouvrir</Small>
        </div>
      ) : t.test && t.column === 'review' && !t.step ? (
        <div style={{ display: 'flex' }}>
          <Small pressed={t.pressed?.test}>▶ Tester</Small>
        </div>
      ) : null}
      {t.step ? (
        <span style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12.5, color: C.muted }}>
          <Dots />
          {t.step}
        </span>
      ) : null}
      {t.blocked ? (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 6,
            padding: '7px 9px',
            borderRadius: 6,
            background: soft(C.del, 12),
            color: C.del,
            fontSize: 12.5,
          }}
        >
          <span>{t.blocked}</span>
          {t.column === 'doing' ? (
            <div style={{ display: 'flex' }}>
              <Small pressed={t.pressed?.resume}>Reprendre</Small>
            </div>
          ) : null}
        </div>
      ) : null}
      {t.column !== 'todo' && (t.agent || t.doneMeta) ? (
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {t.agent ? (
            <span style={{ display: 'flex', alignItems: 'center', gap: 6, minWidth: 0, fontFamily: MONO, fontSize: 11.5 }}>
              <Dot color={STATUS_COLOR[t.agent.status]} size={7} pulse={t.agent.status === 'running'} />
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{t.agent.name}</span>
            </span>
          ) : null}
          <div style={{ flex: 1 }} />
          {t.column === 'review' ? (
            <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>
              {met}/{total} critères
            </span>
          ) : null}
          {t.column === 'done' && t.doneMeta ? <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>{t.doneMeta}</span> : null}
        </div>
      ) : null}
      {t.column === 'review' && !t.step ? (
        t.rejecting !== undefined ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <span
              style={{
                minHeight: 52,
                padding: '7px 9px',
                borderRadius: 6,
                border: `1px solid ${C.spark}`,
                background: C.bg,
                fontSize: 12.5,
                lineHeight: 1.4,
                color: t.rejecting ? C.text : C.dim,
              }}
            >
              {t.rejecting || 'Ce qui ne va pas'}
            </span>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
              <Small>Annuler</Small>
              <Button primary={C.spark} small pressed={t.pressed?.reject}>
                Renvoyer
              </Button>
            </div>
          </div>
        ) : (
          <div style={{ display: 'flex', gap: 6 }}>
            <Small grow primary={C.ok}>
              <span style={{ transform: `scale(${1 - 0.05 * (t.pressed?.approve ?? 0)})` }}>{t.approve ?? 'Valider et merger'}</span>
            </Small>
            <Small grow pressed={t.pressed?.reject}>
              Renvoyer
            </Small>
          </div>
        )
      ) : null}
    </div>
  );
};

/** A section title of a modal, as the app writes it (not in capitals). */
const Heading: FC<{ children: ReactNode }> = ({ children }) => <span style={{ fontSize: 14, fontWeight: 700 }}>{children}</span>;

/** A new ticket, being written. */
export const TicketForm: FC<{
  title: string;
  description: string;
  criteria: string[];
  loops: number | null;
  focus?: 'title' | 'description' | 'criteria';
  pressed?: number;
}> = ({ title, description, criteria, loops, focus, pressed = 0 }) => (
  <div
    style={{
      flex: 'none',
      display: 'flex',
      flexDirection: 'column',
      gap: 8,
      padding: 10,
      borderRadius: 6,
      background: C.elev,
      border: `1px solid ${C.spark}`,
    }}
  >
    <span
      style={{
        height: 34,
        display: 'flex',
        alignItems: 'center',
        padding: '0 10px',
        borderRadius: 6,
        border: `1px solid ${focus === 'title' ? C.spark : C.line2}`,
        background: C.bg,
        fontSize: 14,
        fontWeight: 600,
        color: title ? C.text : C.dim,
      }}
    >
      {title || 'Titre du ticket'}
    </span>
    <span
      style={{
        minHeight: 46,
        padding: '7px 10px',
        borderRadius: 6,
        border: `1px solid ${focus === 'description' ? C.spark : C.line2}`,
        background: C.bg,
        fontSize: 12.5,
        lineHeight: 1.45,
        color: description ? C.text : C.dim,
      }}
    >
      {description || 'Description (facultative)'}
    </span>
    <span
      style={{
        minHeight: 74,
        padding: '7px 10px',
        borderRadius: 6,
        border: `1px solid ${focus === 'criteria' ? C.spark : C.line2}`,
        background: C.bg,
        fontSize: 12.5,
        lineHeight: 1.45,
        color: criteria.length ? C.text : C.dim,
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      {criteria.length ? criteria.map((c, i) => <span key={i}>{c}</span>) : "Critères d'acceptation, un par ligne"}
    </span>
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <span style={{ fontSize: 12, color: C.dim, marginRight: 4 }}>Boucles max</span>
      {[3, 5, 8].map((n) => (
        <span
          key={n}
          style={{
            height: 26,
            minWidth: 30,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: 6,
            border: `1px solid ${loops === n ? C.spark : C.line2}`,
            background: loops === n ? C.elev2 : 'transparent',
            fontFamily: MONO,
            fontSize: 12,
            fontWeight: 600,
            color: loops === n ? C.text : C.muted,
          }}
        >
          {n}
        </span>
      ))}
    </div>
    <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
      <Small>Annuler</Small>
      <Button primary={C.spark} small pressed={pressed}>
        Ajouter
      </Button>
    </div>
  </div>
);

const ACTIONS = [
  { label: 'Merger dans une branche', desc: "Fusionne le worktree de l'agent dans la branche cible, puis libère l'agent." },
  { label: 'Ouvrir une pull request', desc: 'Pousse ticket/<clé> et ouvre une PR vers la branche cible pour relecture.' },
  { label: 'Pousser la branche du ticket', desc: 'Commit et push sur ticket/<clé>, sans merge ni PR.' },
  { label: "Laisser en l'état", desc: 'Les modifications restent non commitées dans le worktree.' },
];

/** « Réglages du tableau »; `hover` lights an action, `glow` one of the lower blocks. */
export const BoardSettings: FC<{ enter: number; action: number; hover?: number; glow?: 'conflicts' | 'agents'; parallel: number }> = ({
  enter,
  action,
  hover,
  glow,
  parallel,
}) => (
  <Modal title="Réglages du tableau" sub="demo-api" width={720} enter={enter} footer={<Button primary={C.spark}>Terminé</Button>}>
    <Heading>Quand je valide un ticket « À tester »</Heading>
    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
      {ACTIONS.map((a, i) => (
        <div
          key={a.label}
          style={{
            display: 'flex',
            gap: 10,
            padding: '10px 12px',
            borderRadius: 8,
            border: `1px solid ${i === action ? C.spark : i === hover ? C.line2 : C.line}`,
            background: i === action ? soft(C.spark, 10) : i === hover ? C.elev : 'transparent',
          }}
        >
          <span
            style={{
              width: 16,
              height: 16,
              marginTop: 2,
              flex: 'none',
              borderRadius: '50%',
              border: `2px solid ${i === action ? C.spark : C.dim}`,
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            {i === action ? <span style={{ width: 7, height: 7, borderRadius: '50%', background: C.spark }} /> : null}
          </span>
          <span style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            <span style={{ fontSize: 14.5, fontWeight: 600 }}>{a.label}</span>
            <span style={{ fontSize: 12.5, color: C.muted, lineHeight: 1.35 }}>{a.desc}</span>
          </span>
        </div>
      ))}
    </div>
    <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
      <span style={{ fontSize: 13, color: C.muted, width: 110 }}>Branche cible</span>
      <Chips items={['⎇ main', '⎇ develop']} on={0} mono />
      <span style={{ fontSize: 13, color: C.muted, marginLeft: 12 }}>Stratégie</span>
      <Chips items={['Merge commit', 'Squash', 'Rebase']} on={1} />
    </div>
    <div
      style={{ display: 'flex', flexDirection: 'column', gap: 10, padding: '12px 14px', borderRadius: 8, border: `1px solid ${C.line}` }}
    >
      {[
        ['Relancer les tests avant', "Bloque l'action si un test échoue et renvoie le ticket à l'agent.", true],
        ['Supprimer le worktree après merge', "Libère l'espace disque et repart d'une branche propre.", true],
        ['Message de commit généré', 'Format Conventional Commits, avec la clé du ticket.', true],
      ].map(([l, d, on], i) => (
        <div key={String(l)} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <span style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 2 }}>
            <span style={{ fontSize: 14, fontWeight: 600 }}>{l}</span>
            <span style={{ fontSize: 12.5, color: C.muted }}>{d}</span>
            {/* The tests' command, under its switch (BoardSettingsModal.svelte). */}
            {i === 0 ? (
              <span style={{ marginTop: 4, display: 'flex' }}>
                <Field value="npm test" mono grow={false} />
              </span>
            ) : null}
          </span>
          <Switch on={Boolean(on)} />
        </div>
      ))}
      <span style={{ fontFamily: MONO, fontSize: 12.5, color: C.dim }}>feat: limiter les tentatives de connexion [DEM-42]</span>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: 6,
          margin: -6,
          borderRadius: 8,
          boxShadow: glow === 'conflicts' ? `0 0 0 2px ${C.spark}` : 'none',
        }}
      >
        <span style={{ flex: 1, fontSize: 14, fontWeight: 600 }}>En cas de conflit</span>
        <Chips items={['Me demander', "L'agent résout", 'Annuler']} on={0} />
      </div>
    </div>
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: 6,
        margin: -6,
        borderRadius: 8,
        boxShadow: glow === 'agents' ? `0 0 0 2px ${C.spark}` : 'none',
      }}
    >
      <Heading>Agents</Heading>
      <span style={{ fontSize: 13, color: C.muted, marginLeft: 8 }}>En parallèle</span>
      <Chips items={['1', '2', '3', '4', '5', '6']} on={parallel - 1} mono />
      <span style={{ fontSize: 13, color: C.muted, marginLeft: 8 }}>Modèle</span>
      <Field value="Comme les réglages (Opus 5.5)" grow={false} />
    </div>
    <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginTop: -4 }}>
      <span style={{ fontSize: 13, color: C.muted, marginLeft: 70 }}>Effort</span>
      <Field value="Comme les réglages" grow={false} />
      <span style={{ fontSize: 13, color: C.muted }}>Mode</span>
      <Field value="Comme les réglages" grow={false} />
    </div>
  </Modal>
);

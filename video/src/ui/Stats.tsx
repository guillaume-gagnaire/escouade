import type { FC } from 'react';
import { fr } from '../anim';
import { C, hue, MONO, soft } from '../theme';

/** The three series of the app's chart. */
const SERIES = [
  { key: 'input', label: 'Entrée', color: 'oklch(0.74 0.12 235)' },
  { key: 'cache', label: 'Cache', color: 'oklch(0.76 0.12 150)' },
  { key: 'output', label: 'Sortie', color: '#D97757' },
] as const;
/** Millions of tokens per day over 14 days: input, cache, output. */
const DAYS: [number, number, number][] = [
  [0.6, 1.4, 0.3],
  [0.8, 2.1, 0.4],
  [0.4, 1.1, 0.2],
  [0.9, 2.6, 0.5],
  [1.1, 3.0, 0.6],
  [0.2, 0.5, 0.1],
  [0.3, 0.6, 0.1],
  [0.9, 2.2, 0.4],
  [1.2, 3.1, 0.6],
  [1.0, 2.4, 0.5],
  [1.3, 3.4, 0.7],
  [1.5, 3.9, 0.8],
  [0.4, 0.9, 0.2],
  [1.1, 2.8, 0.6],
];
/** Jour (14 days), Semaine (12 weeks), Mois (12 months), as the app's ranges. */
const LABELS = [
  ['21/09', '22/09', '23/09', '24/09', '25/09', '26/09', '27/09', '28/09', '29/09', '30/09', '01/10', '02/10', '03/10', '04/10'],
  ['S29', 'S30', 'S31', 'S32', 'S33', 'S34', 'S35', 'S36', 'S37', 'S38', 'S39', 'S40'],
  ['nov.', 'déc.', 'janv.', 'févr.', 'mars', 'avr.', 'mai', 'juin', 'juil.', 'août', 'sept.', 'oct.'],
];
const SPANS = ['14 derniers jours', '12 dernières semaines', '12 derniers mois'];
const UNITS = ['jour', 'semaine', 'mois'];
/** How much more a range holds than the 14 days. */
const SCALE = [1, 5.4, 9.2];
const PROJECTS = [
  { name: 'demo-api', hue: 48, cost: 21.4 },
  { name: 'mobile-app', hue: 200, cost: 11.2 },
  { name: 'studio-web', hue: 300, cost: 6.9 },
  { name: 'infra', hue: 150, cost: 2.6 },
];
const MODELS = [
  { name: 'Opus 5.5', cost: 26.3, color: '#D97757' },
  { name: 'Sonnet 5.5', cost: 11.9, color: 'oklch(0.74 0.12 235)' },
  { name: 'Fable 5.1', cost: 3.1, color: 'oklch(0.74 0.12 300)' },
  { name: 'Haiku 4.5', cost: 0.8, color: 'oklch(0.76 0.12 150)' },
];

const Bars: FC<{ title: string; rows: { name: string; cost: number; color: string }[]; grow: number; glow: number }> = ({
  title,
  rows,
  grow,
  glow,
}) => {
  const max = Math.max(...rows.map((r) => r.cost));
  return (
    <div
      style={{
        flex: 1,
        padding: '14px 18px',
        borderRadius: 12,
        background: C.elev,
        border: `1px solid ${glow ? C.spark : C.line}`,
        boxShadow: glow ? `0 0 30px ${soft(C.spark, 30 * glow)}` : 'none',
        display: 'flex',
        flexDirection: 'column',
        gap: 9,
      }}
    >
      <span style={{ fontSize: 13, fontWeight: 700, color: C.muted }}>{title}</span>
      {rows.map((r) => (
        <div key={r.name} style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 13.5 }}>
          <span style={{ width: 96, display: 'flex', alignItems: 'center', gap: 7 }}>
            <span style={{ width: 8, height: 8, borderRadius: 2, background: r.color }} />
            {r.name}
          </span>
          <span style={{ flex: 1, height: 8, borderRadius: 4, background: C.elev2, overflow: 'hidden' }}>
            <span style={{ display: 'block', width: `${(r.cost / max) * 100 * grow}%`, height: '100%', background: r.color }} />
          </span>
          <span style={{ width: 62, textAlign: 'right', fontFamily: MONO, fontSize: 12.5 }}>{fr(r.cost)} $</span>
        </div>
      ))}
    </div>
  );
};

/** The stats page: range, four KPIs, tokens per day by series, cost by project and by model. */
export const StatsView: FC<{
  grow: number;
  count: number;
  range?: number;
  glow?: { series?: number; projects?: number; models?: number; range?: number };
}> = ({ grow, count, range = 0, glow = {} }) => {
  const max = 5.5;
  return (
    <div style={{ flex: 1, padding: '22px 30px', display: 'flex', flexDirection: 'column', gap: 16, minHeight: 0 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
        <span style={{ fontSize: 24, fontWeight: 700 }}>Statistiques</span>
        <span style={{ fontSize: 13.5, color: C.dim }}>Agents lancés depuis l'app · 4 projets, 12 agents</span>
        <div style={{ flex: 1 }} />
        <div
          style={{
            display: 'flex',
            gap: 2,
            padding: 3,
            borderRadius: 8,
            background: C.bg,
            border: `1px solid ${glow.range ? C.spark : C.line}`,
          }}
        >
          {['Jour', 'Semaine', 'Mois'].map((r, i) => (
            <span
              key={r}
              style={{
                padding: '5px 14px',
                borderRadius: 6,
                fontSize: 13.5,
                fontWeight: 600,
                background: i === range ? C.elev2 : 'transparent',
                color: i === range ? C.text : C.muted,
              }}
            >
              {r}
            </span>
          ))}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 14 }}>
        {[
          ['Tokens', `${fr(28.1 * SCALE[range] * count, 2)} M`, `${SPANS[range]} · +12 %`],
          ['Coût global', `${fr(42.17 * SCALE[range] * count)} $`, '412,80 $ depuis le 14 août 2025'],
          ['Coût moyen / prompt', `${fr(0.31 * count)} $`, '≈ 206,6 k tokens / prompt'],
          [
            'Prompts',
            Math.round(136 * SCALE[range] * count).toLocaleString('fr-FR'),
            `${Math.round((136 * SCALE[range]) / LABELS[range].length)} par ${UNITS[range]} en moyenne`,
          ],
        ].map(([k, v, s]) => (
          <div key={k} style={{ flex: 1, padding: '12px 16px', borderRadius: 12, background: C.elev, border: `1px solid ${C.line}` }}>
            <div style={{ fontSize: 12.5, color: C.muted }}>{k}</div>
            <div style={{ marginTop: 4, fontFamily: MONO, fontSize: 26, fontWeight: 600 }}>{v}</div>
            <div style={{ marginTop: 2, fontSize: 12, color: C.dim }}>{s}</div>
          </div>
        ))}
      </div>
      <div
        style={{
          flex: 1,
          minHeight: 0,
          padding: '12px 16px',
          borderRadius: 12,
          background: C.elev,
          border: `1px solid ${C.line}`,
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <span style={{ fontSize: 13, fontWeight: 700, color: C.muted }}>Tokens par {UNITS[range]}</span>
          <div style={{ flex: 1 }} />
          {SERIES.map((s, i) => (
            <span
              key={s.key}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 6,
                fontSize: 13,
                padding: '2px 8px',
                borderRadius: 5,
                background: glow.series === i + 1 ? soft(s.color, 25) : 'transparent',
                color: glow.series === i + 1 ? C.text : C.muted,
              }}
            >
              <span style={{ width: 10, height: 10, borderRadius: 2, background: s.color }} />
              {s.label}
            </span>
          ))}
        </div>
        <div style={{ flex: 1, display: 'flex', alignItems: 'flex-end', gap: 12, borderBottom: `1px solid ${C.line2}` }}>
          {DAYS.slice(DAYS.length - LABELS[range].length).map((d, i) => {
            const g = Math.max(0, Math.min(1, grow * 1.6 - i * 0.04));
            return (
              <div key={i} style={{ flex: 1, height: '100%', display: 'flex', flexDirection: 'column-reverse' }}>
                {d.map((v, k) => (
                  <div
                    key={k}
                    style={{
                      height: `${(v / max) * 100 * g}%`,
                      background: SERIES[k].color,
                      opacity: glow.series && glow.series !== k + 1 ? 0.35 : 1,
                      borderRadius: k === 2 ? '3px 3px 0 0' : 0,
                    }}
                  />
                ))}
              </div>
            );
          })}
        </div>
        <div style={{ display: 'flex', gap: 12 }}>
          {LABELS[range].map((l) => (
            <span key={l} style={{ flex: 1, textAlign: 'center', fontFamily: MONO, fontSize: 11, color: C.dim }}>
              {l}
            </span>
          ))}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 14 }}>
        <Bars
          title="Par projet"
          rows={PROJECTS.map((p) => ({ name: p.name, cost: p.cost * SCALE[range], color: hue(p.hue) }))}
          grow={grow}
          glow={glow.projects ?? 0}
        />
        <Bars title="Par modèle" rows={MODELS.map((m) => ({ ...m, cost: m.cost * SCALE[range] }))} grow={grow} glow={glow.models ?? 0} />
      </div>
    </div>
  );
};

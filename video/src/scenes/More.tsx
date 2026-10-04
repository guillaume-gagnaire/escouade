import type { FC, ReactNode } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { C, MONO, soft } from '../theme';
import { Switch } from '../ui/Modal';
import { Stage, Title } from '../ui/Stage';

const Tile: FC<{ title: string; enter: number; glow: number; children: ReactNode }> = ({ title, enter, glow, children }) => (
  <div
    style={{
      width: 700,
      height: 330,
      padding: '22px 26px',
      borderRadius: 16,
      background: C.panel,
      border: `1px solid ${glow ? C.spark : C.line2}`,
      boxShadow: glow ? `0 0 50px ${soft(C.spark, 30 * glow)}` : '0 30px 80px rgba(0,0,0,0.45)',
      display: 'flex',
      flexDirection: 'column',
      gap: 14,
      opacity: Math.min(1, enter),
      transform: `translateY(${(1 - Math.min(1, enter)) * 30}px) scale(${1 + 0.03 * glow})`,
    }}
  >
    <span style={{ fontSize: 24, fontWeight: 700 }}>{title}</span>
    {children}
  </div>
);

const Row: FC<{ k: string; children: ReactNode }> = ({ k, children }) => (
  <div style={{ display: 'flex', alignItems: 'center', gap: 14, fontSize: 16 }}>
    <span style={{ width: 210, color: C.muted }}>{k}</span>
    {children}
  </div>
);
const Val: FC<{ children: ReactNode }> = ({ children }) => (
  <span
    style={{
      flex: 1,
      padding: '6px 10px',
      borderRadius: 6,
      border: `1px solid ${C.line2}`,
      background: C.bg,
      fontFamily: MONO,
      fontSize: 14,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}
  >
    {children}
  </span>
);
const Key: FC<{ children: ReactNode }> = ({ children }) => (
  <span
    style={{
      padding: '3px 8px',
      borderRadius: 5,
      border: `1px solid ${C.line2}`,
      borderBottomWidth: 3,
      background: C.elev,
      fontFamily: MONO,
      fontSize: 14,
    }}
  >
    {children}
  </span>
);

const Platform: FC<{ name: string; detail: string; enter: number; logo: ReactNode }> = ({ name, detail, enter, logo }) => (
  <div
    style={{
      width: 560,
      padding: '30px 34px',
      borderRadius: 18,
      background: C.panel,
      border: `1px solid ${C.line2}`,
      display: 'flex',
      alignItems: 'center',
      gap: 26,
      opacity: Math.min(1, enter),
      transform: `translateY(${(1 - Math.min(1, enter)) * 40}px)`,
      boxShadow: '0 30px 80px rgba(0,0,0,0.45)',
    }}
  >
    {logo}
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      <span style={{ fontSize: 38, fontWeight: 800 }}>{name}</span>
      <span style={{ fontSize: 19, color: C.muted }}>{detail}</span>
    </div>
  </div>
);

/** Settings, proxy, shortcuts, updates; then Windows and macOS. */
export const More: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const w = (word: string) => c.word('extras', word);
  const glow = (from: number, to: number) => ramp(frame, from - 4, 8) - ramp(frame, to - 4, 8);
  const tilesOut = ramp(frame, c.at('os') - 10, 14);
  const os = (word: string) => pop(frame, fps, c.word('os', word) - 6, 16);
  return (
    <Stage>
      <Title />
      <div
        style={{
          position: 'absolute',
          left: 0,
          right: 0,
          top: 190,
          display: 'grid',
          gridTemplateColumns: '700px 700px',
          justifyContent: 'center',
          gap: 30,
          opacity: 1 - tilesOut,
          transform: `scale(${1 - 0.08 * tilesOut})`,
        }}
      >
        <Tile title="Réglages" enter={pop(frame, fps, w('réglages') - 6)} glow={glow(w('réglages'), w('proxy'))}>
          <Row k="Chemin de claude">
            <Val>C:\Users\lea\.local\bin\claude.exe</Val>
          </Row>
          <Row k="Modèle, effort, mode">
            <Val>Opus · Élevé · Auto</Val>
          </Row>
          <Row k="Notifications Windows">
            <Switch on />
          </Row>
          <Row k="Arrêt des agents inactifs">
            <Val>après 30 min</Val>
          </Row>
        </Tile>
        <Tile title="Proxy réseau" enter={pop(frame, fps, w('proxy') - 6)} glow={glow(w('proxy'), w('raccourcis'))}>
          <Row k="Proxy HTTP(S)">
            <Val>http://proxy.entreprise.fr:8080</Val>
          </Row>
          <Row k="Exclusions (NO_PROXY)">
            <Val>localhost, *.interne</Val>
          </Row>
          <Row k="Aussi dans les terminaux">
            <Switch on />
          </Row>
          <span style={{ fontSize: 14, color: C.dim }}>Pour Claude, les quotas, les mises à jour et, si tu veux, les terminaux.</span>
        </Tile>
        <Tile title="Raccourcis clavier" enter={pop(frame, fps, w('raccourcis') - 6)} glow={glow(w('raccourcis'), w('mises'))}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px 18px' }}>
            {[
              [['Ctrl', '1…9'], 'projet n'],
              [['Ctrl', 'Tab'], 'agent suivant'],
              [['Ctrl', 'N'], 'nouvel agent'],
              [['Ctrl', 'T'], 'nouveau terminal'],
              [['Ctrl', 'J'], 'agent qui attend'],
              [['Ctrl', 'Maj', 'B'], 'fichiers'],
              [['Ctrl', 'Maj', 'L'], 'disposition'],
              [['Ctrl', ','], 'réglages'],
              [['Échap'], 'interrompre'],
              [['↑'], 'dernier message'],
            ].map(([keys, label]) => (
              <div key={String(label)} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 15 }}>
                <span style={{ width: 140, display: 'flex', gap: 4 }}>
                  {(keys as string[]).map((k) => (
                    <Key key={k}>{k}</Key>
                  ))}
                </span>
                <span style={{ color: C.muted }}>{label as string}</span>
              </div>
            ))}
          </div>
        </Tile>
        <Tile title="Mises à jour automatiques" enter={pop(frame, fps, w('mises') - 6)} glow={glow(w('mises'), c.at('os'))}>
          <span
            style={{
              alignSelf: 'flex-start',
              padding: '8px 14px',
              borderRadius: 8,
              background: soft(C.ok, 18),
              color: C.ok,
              fontSize: 16,
              fontWeight: 700,
            }}
          >
            Mise à jour 1.4.0 disponible → installer
          </span>
          <span style={{ fontSize: 16, color: C.muted, lineHeight: 1.5 }}>
            L'app cherche une nouvelle version toutes les cinq minutes. Les versions sont signées, et leurs notes viennent du journal des
            versions.
          </span>
        </Tile>
      </div>
      <div style={{ position: 'absolute', left: 0, right: 0, top: 380, display: 'flex', justifyContent: 'center', gap: 40 }}>
        <Platform
          name="Windows"
          detail="10 et 11 · installeur .exe"
          enter={os('windows')}
          logo={
            <svg width="96" height="96" viewBox="0 0 20 20">
              <rect x="0" y="0" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="10.6" y="0" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="0" y="10.6" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="10.6" y="10.6" width="9.4" height="9.4" fill="#4cc2ff" />
            </svg>
          }
        />
        <Platform
          name="macOS"
          detail="11 et plus · .dmg universel"
          enter={os('mac')}
          logo={
            <svg width="96" height="96" viewBox="0 0 24 24" fill={C.text}>
              <path d="M16.4 12.6c0-2.4 2-3.5 2-3.6-1.1-1.6-2.8-1.8-3.4-1.8-1.4-.1-2.8.9-3.5.9-.7 0-1.9-.9-3.1-.8-1.6 0-3 .9-3.8 2.3-1.6 2.8-.4 7 1.2 9.3.8 1.1 1.7 2.4 2.9 2.3 1.2 0 1.6-.7 3-.7s1.8.7 3.1.7c1.3 0 2.1-1.1 2.8-2.3.9-1.3 1.3-2.6 1.3-2.6s-2.5-1-2.5-3.7zM14.2 5.6c.6-.8 1.1-1.9 1-3-1 0-2.1.7-2.8 1.4-.6.7-1.1 1.8-1 2.9 1.1.1 2.1-.6 2.8-1.3z" />
            </svg>
          }
        />
      </div>
    </Stage>
  );
};

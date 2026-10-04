import type { FC } from 'react';
import { C, MONO, UI } from '../theme';
import { Modal } from './Modal';
import { Button } from './Sidebar';

export type StepState = 'waiting' | 'running' | 'ready' | 'failed';
const MARK: Record<StepState, string> = { waiting: '·', running: '…', ready: '✓', failed: '✕' };

export interface Step {
  label: string;
  detail: string;
  state: StepState;
}

/** « Tester DEM-6 »: each step of the recipe, then where the browser was opened. */
export const TestLaunchModal: FC<{ title: string; steps: Step[]; opened?: string; enter: number; pressedLogs?: number }> = ({
  title,
  steps,
  opened,
  enter,
  pressedLogs,
}) => (
  <Modal
    title={title}
    width={640}
    enter={enter}
    footer={
      <>
        {opened ? <Button>Rouvrir</Button> : null}
        <Button pressed={pressedLogs}>Voir les logs</Button>
        <Button>
          <span style={{ color: C.del }}>Tout arrêter</span>
        </Button>
        <Button primary={C.spark}>Fermer</Button>
      </>
    }
  >
    <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
      {steps.map((s) => (
        <div
          key={s.label}
          style={{ display: 'flex', alignItems: 'baseline', gap: 12, fontSize: 15, opacity: s.state === 'waiting' ? 0.5 : 1 }}
        >
          <span
            style={{
              width: 16,
              textAlign: 'center',
              color: s.state === 'ready' ? C.ok : s.state === 'failed' ? C.del : C.dim,
              fontWeight: 700,
            }}
          >
            {MARK[s.state]}
          </span>
          <span style={{ fontWeight: 600, width: 110, flex: 'none' }}>{s.label}</span>
          <span style={{ fontFamily: MONO, fontSize: 13, color: C.muted }}>{s.detail}</span>
        </div>
      ))}
    </div>
    {opened ? (
      <span style={{ fontSize: 14, color: C.muted }}>
        Ouvert dans le navigateur : <span style={{ fontFamily: MONO, color: C.text }}>{opened}</span>
      </span>
    ) : null}
  </Modal>
);

/** A browser window on the feature being tested: the login page, refusing a sixth try. */
export const Browser: FC<{ x: number; url: string; tries: number }> = ({ x, url, tries }) => (
  <div
    style={{
      position: 'absolute',
      left: x,
      top: 200,
      width: 980,
      height: 700,
      borderRadius: 12,
      overflow: 'hidden',
      background: '#f6f5f2',
      boxShadow: '0 40px 120px rgba(0, 0, 0, 0.65)',
      fontFamily: UI,
      zIndex: 80,
    }}
  >
    <div style={{ height: 44, background: '#e7e5e0', display: 'flex', alignItems: 'center', gap: 12, padding: '0 16px' }}>
      <span style={{ display: 'flex', gap: 7 }}>
        {['#ec6a5e', '#f4bf4f', '#61c554'].map((c) => (
          <span key={c} style={{ width: 12, height: 12, borderRadius: 6, background: c }} />
        ))}
      </span>
      <span
        style={{
          flex: 1,
          height: 28,
          borderRadius: 7,
          background: '#ffffff',
          display: 'flex',
          alignItems: 'center',
          padding: '0 12px',
          fontFamily: MONO,
          fontSize: 14,
          color: '#333',
        }}
      >
        {url}
      </span>
    </div>
    <div
      style={{
        height: 'calc(100% - 44px)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'linear-gradient(160deg, #fbfaf7, #efece6)',
      }}
    >
      <div
        style={{
          width: 400,
          padding: 32,
          borderRadius: 14,
          background: '#ffffff',
          boxShadow: '0 10px 40px rgba(0,0,0,0.08)',
          display: 'flex',
          flexDirection: 'column',
          gap: 14,
          color: '#1d1b18',
        }}
      >
        <span style={{ fontSize: 24, fontWeight: 800 }}>Connexion</span>
        <span
          style={{
            height: 40,
            borderRadius: 8,
            border: '1px solid #d9d6cf',
            display: 'flex',
            alignItems: 'center',
            padding: '0 12px',
            fontSize: 15,
            color: '#555',
          }}
        >
          lea@exemple.fr
        </span>
        <span
          style={{
            height: 40,
            borderRadius: 8,
            border: '1px solid #d9d6cf',
            display: 'flex',
            alignItems: 'center',
            padding: '0 12px',
            fontSize: 18,
            color: '#555',
          }}
        >
          ••••••••
        </span>
        {tries >= 6 ? (
          <span style={{ padding: '10px 12px', borderRadius: 8, background: '#fdecea', color: '#b3261e', fontSize: 14, fontWeight: 600 }}>
            Trop de tentatives. Réessaie dans 15 min.
          </span>
        ) : (
          <span style={{ fontSize: 13, color: '#b3261e' }}>Mot de passe incorrect ({tries}/5)</span>
        )}
        <span
          style={{
            height: 42,
            borderRadius: 8,
            background: tries >= 6 ? '#bdb8ae' : '#1d1b18',
            color: '#ffffff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontWeight: 700,
          }}
        >
          Se connecter
        </span>
      </div>
    </div>
  </div>
);

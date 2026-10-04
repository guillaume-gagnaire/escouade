import type { FC } from 'react';
import { C, hue, MONO } from '../theme';
import { Field, Label, Modal, Toggle } from './Modal';
import { Button } from './Sidebar';

/** The app's 14 project colours (lib/theme.ts). */
export const COLORS = [48, 300, 200, 150, 250, 0, 25, 70, 110, 175, 225, 275, 325].map(hue).concat(['oklch(0.8 0.02 60)']);

/** « Nouveau projet »: folder, git, name, colour, first agent, worktrees. */
export const NewProjectModal: FC<{
  enter: number;
  path: string;
  git: number;
  name: string;
  color: number | null;
  firstAgent: boolean;
  worktrees: boolean;
  pressed?: number;
}> = ({ enter, path, git, name, color, firstAgent, worktrees, pressed }) => {
  const col = COLORS[color ?? 0];
  return (
    <Modal
      title="Nouveau projet"
      width={660}
      enter={enter}
      footer={
        <>
          <Button>Annuler</Button>
          <Button primary={col} pressed={pressed}>
            Créer le projet
          </Button>
        </>
      }
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <Label>Dossier</Label>
        <div style={{ display: 'flex', gap: 8 }}>
          <Field value={path} mono placeholder="C:\chemin\vers\le\projet" focus={git === 0 && path.length > 0} />
          <Button>Parcourir…</Button>
        </div>
        <span
          style={{ display: 'flex', alignItems: 'center', gap: 8, fontFamily: MONO, fontSize: 13, color: git ? C.ok : C.dim, opacity: git }}
        >
          <span style={{ width: 7, height: 7, borderRadius: '50%', background: C.ok }} />
          Dépôt git détecté · branche main
        </span>
      </div>
      <div style={{ display: 'flex', gap: 16 }}>
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <Label>Nom</Label>
          <Field value={name} />
        </div>
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 8 }}>
          <Label>Aperçu de l'onglet</Label>
          <span
            style={{
              height: 38,
              alignSelf: 'flex-start',
              display: 'flex',
              alignItems: 'center',
              gap: 9,
              padding: '0 15px',
              borderRadius: '9px 9px 0 0',
              border: `1px solid ${C.line2}`,
              borderBottom: 'none',
              background: C.bg,
              fontSize: 15,
              fontWeight: 600,
            }}
          >
            <span style={{ width: 9, height: 9, borderRadius: 3, background: col }} />
            {name || 'nouveau-projet'}
          </span>
        </div>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        <Label>Couleur</Label>
        <div style={{ display: 'flex', gap: 9 }}>
          {COLORS.map((h, i) => (
            <span
              key={h}
              style={{
                width: 30,
                height: 30,
                borderRadius: 8,
                background: h,
                boxShadow: i === color ? `0 0 0 2px ${C.panel}, 0 0 0 4px ${h}` : 'none',
              }}
            />
          ))}
        </div>
      </div>
      <div style={{ height: 1, background: C.line }} />
      <Toggle title="Créer un premier agent" desc="Ouvre directement une conversation dans ce projet" on={firstAgent}>
        {firstAgent ? (
          <span style={{ display: 'flex', gap: 6, marginTop: 6 }}>
            {['Fable', 'Opus', 'Sonnet', 'Haiku'].map((m, i) => (
              <span
                key={m}
                style={{
                  fontFamily: MONO,
                  fontSize: 12.5,
                  padding: '3px 9px',
                  borderRadius: 5,
                  border: `1px solid ${i === 1 ? col : C.line2}`,
                  color: i === 1 ? C.text : C.muted,
                }}
              >
                {m}
              </span>
            ))}
          </span>
        ) : null}
      </Toggle>
      <Toggle
        title="Un worktree git par agent"
        desc="Isole le travail de chaque agent et permet de voir ses fichiers modifiés séparément"
        on={worktrees}
      />
    </Modal>
  );
};

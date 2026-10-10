// Pieces several scenes share: the demo conversations and an agent's empty state.

import type { FC } from 'react';
import { useFmt, type Tr } from '../lang';
import { C, MONO } from '../theme';
import { AssistantMsg, Diffstat, ToolCall, UserMsg } from '../ui/Chat';
import type { ContextMenu } from '../ui/Modal';

/** The menu of an agent's card (right click). */
export const agentMenu = (tr: Tr): Parameters<typeof ContextMenu>[0]['items'] => [
  tr('Renommer', 'Rename'),
  { label: tr('Archiver', 'Archive'), hint: tr('garde la conversation', 'keeps the conversation') },
  tr('Ouvrir dans l’éditeur', 'Open in editor'),
  null,
  { label: tr('Activer le remote control', 'Turn on remote control'), hint: 'claude.ai, mobile' },
  null,
  { label: tr('Supprimer…', 'Delete…'), danger: true },
];

export const RefactoConv: FC = () => {
  const { tr } = useFmt();
  return (
    <>
      <UserMsg
        text={tr(
          "Refactore l'auth pour passer aux tokens JWT, sans casser les sessions existantes.",
          'Refactor auth to switch to JWT tokens without breaking existing sessions.',
        )}
      />
      <AssistantMsg
        text={tr(
          "D'accord. Je cartographie l'existant : middleware, routes et stockage des sessions.",
          'Sure. I’m mapping what exists: middleware, routes and session storage.',
        )}
      />
      <ToolCall tool="Read" target="src/auth/middleware.ts" meta={<span style={{ color: C.ok }}>✓</span>} />
      <ToolCall tool="Grep" target="session" meta={<span style={{ fontSize: 13, color: C.muted }}>{tr('14 fichiers', '14 files')}</span>} />
      <ToolCall tool="Edit" target="src/auth/jwt.ts" meta={<Diffstat add={58} del={0} />} />
      <AssistantMsg
        text={tr(
          'Je remplace le middleware de session par une vérification du JWT, avec une période de transition.',
          'I’m replacing the session middleware with JWT verification, with a transition period.',
        )}
      />
    </>
  );
};

export const EmptyConv: FC<{ cwd: string }> = ({ cwd }) => {
  const { tr } = useFmt();
  return (
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10 }}>
      <span style={{ fontSize: 22, fontWeight: 700 }}>{tr('Agent prêt', 'Agent ready')}</span>
      <span style={{ fontSize: 16, color: C.muted }}>
        {tr("Décris la tâche à confier à Claude. L'agent travaille dans ", 'Describe the task to give Claude. The agent works in ')}
        <span style={{ fontFamily: MONO }}>{cwd}</span>.
      </span>
    </div>
  );
};

// Pieces several scenes share: the demo conversations and an agent's empty state.

import type { FC } from 'react';
import { C, MONO } from '../theme';
import { AssistantMsg, Diffstat, ToolCall, UserMsg } from '../ui/Chat';

export const RefactoConv: FC = () => (
  <>
    <UserMsg text="Refactore l'auth pour passer aux tokens JWT, sans casser les sessions existantes." />
    <AssistantMsg text="D'accord. Je cartographie l'existant : middleware, routes et stockage des sessions." />
    <ToolCall tool="Read" target="src/auth/middleware.ts" meta={<span style={{ color: C.ok }}>✓</span>} />
    <ToolCall tool="Grep" target="session" meta={<span style={{ fontSize: 13, color: C.muted }}>14 fichiers</span>} />
    <ToolCall tool="Edit" target="src/auth/jwt.ts" meta={<Diffstat add={58} del={0} />} />
    <AssistantMsg text="Je remplace le middleware de session par une vérification du JWT, avec une période de transition." />
  </>
);

export const EmptyConv: FC<{ cwd: string }> = ({ cwd }) => (
  <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10 }}>
    <span style={{ fontSize: 22, fontWeight: 700 }}>Agent prêt</span>
    <span style={{ fontSize: 16, color: C.muted }}>
      Décris la tâche à confier à Claude. L'agent travaille dans <span style={{ fontFamily: MONO }}>{cwd}</span>.
    </span>
  </div>
);

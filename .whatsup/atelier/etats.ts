// The atelier's states: made-up projects and agents only (no real customer, person, path or token).
import type { Agent, GitInfo, Project, Settings, Ticket, UiEvent, UiState } from '../../src/lib/types';
import { agent, gitInfo, project } from '../../src/test/ipc';

export interface Etat {
  projects: Project[];
  agents?: Agent[];
  tickets?: Ticket[];
  ui?: Partial<UiState>;
  settings?: Partial<Settings>;
  git?: Record<string, GitInfo>;
  conversations?: Record<string, unknown[]>;
  handlers?: Record<string, (args: any) => unknown>;
  /** Gestures once the window is drawn: clicks, backend events. */
  apres?: (o: { emit: (e: UiEvent) => void }) => Promise<void> | void;
}

const atelier = project({ id: 'p1', name: 'Atelier', path: '/demo/atelier' });
const agents = [
  agent({ id: 'a1', name: 'front-1', projectId: 'p1', cwd: '/demo/atelier' }),
  agent({ id: 'a2', name: 'testeur', projectId: 'p1', cwd: '/demo/atelier' }),
  agent({ id: 'a3', name: 'rédacteur', projectId: 'p1', cwd: '/demo/atelier' }),
];

export const ETATS: Record<string, Etat> = {
  defaut: {
    projects: [atelier],
    agents,
    ui: { selectedAgent: { p1: 'a1' } },
    git: { p1: gitInfo() },
  },
};

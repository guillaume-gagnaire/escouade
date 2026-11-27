// The atelier's states: made-up projects and agents only (no real customer, person, path or token).
import type { Agent, GitInfo, Project, Settings, Ticket, UiEvent, UiState } from '../../src/lib/types';
import { agent, gitInfo, project, ticket } from '../../src/test/ipc';
import { gsd, passage, scanOf, track } from '../../src/test/workflows';
import { app } from '../../src/lib/state.svelte';

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

// ---------- 1. The agents' pages (AP-3, AP-4): a page beside its agent's conversation, its choice band. ----------

const theme = () => (new URLSearchParams(location.search).get('theme') === 'sombre' ? 'dark' : 'light');
const attendre = async (sel: string) => {
  for (let i = 0; i < 100; i++) {
    const el = document.querySelector<HTMLElement>(sel);
    if (el) return el;
    await new Promise((r) => setTimeout(r, 50));
  }
  throw new Error(`atelier : ${sel} introuvable`);
};
/** Draws page `fichier` where the app's own webview would be (its room), and waits for it. */
async function dessinePage(salle: string, fichier: string, extra = '', place = 'inset:0;width:100%') {
  const room = await attendre(salle);
  const f = document.createElement('iframe');
  f.src = `/.whatsup/atelier/pages/${fichier}?theme=${theme()}${extra}`;
  f.style.cssText = `position:absolute;${place};height:100%;top:0;border:0`;
  await new Promise((r) => {
    f.onload = r;
    room.append(f);
  });
}

const DEMANDE = {
  id: 'a-7',
  projectId: 'p1',
  ticketId: null,
  author: { agent: 'a1', name: 'front-1', chef: false },
  kind: 'choice',
  title: 'Quelle mise en page pour le tableau de bord ?',
  body: 'Deux mises en page, rendues sur la page.',
  options: [
    { id: 'a', label: 'Colonnes de chiffres', description: '' },
    { id: 'b', label: 'Cartes empilées', description: '' },
  ],
  multi: false,
  steps: [],
  recommend: { option: 'b', text: null, why: 'Les retards passent en tête.' },
  unblocks: 'ATL-14 démarre sur la mise en page choisie',
  tickets: [],
  urgency: 'normal',
  expiresAt: Date.now() + 7 * 86_400_000,
  decision: false,
  wake: true,
  yesLabel: '',
  noLabel: '',
  gate: false,
  files: [],
  state: 'open',
  createdAt: Date.now() - 120_000,
  updatedAt: Date.now() - 120_000,
  snoozedUntil: null,
  woke: false,
  delegatedTo: null,
  answer: null,
  readAt: null,
  closedAt: null,
  reason: null,
  filesErasedAt: null,
  seen: true,
  history: [],
  projectName: 'Atelier',
  private: false,
  ticketKey: null,
  delegatedName: null,
  delegates: [],
};

function pageRow(choisi: boolean) {
  return {
    page: 'p-3',
    projectId: 'p1',
    agentId: 'a1',
    agentName: 'front-1',
    ticketId: null,
    title: 'Deux mises en page du tableau de bord',
    template: 'compare',
    versions: [{ n: 1, at: Date.now() - 120_000 }],
    shown: 1,
    open: true,
    quiet: false,
    loaded: true,
    ask: 'a-7',
    proposal: choisi ? { ask: 'a-7', options: ['b'], note: null, at: Date.now() - 5_000 } : null,
  };
}

const CONV_PAGE = [
  { kind: 'user', id: 'u1', text: 'Propose-moi deux mises en page pour le tableau de bord, je choisis.', images: 0, ts: Date.now() - 300_000, queued: false },
  { kind: 'text', id: 'x1', text: 'J’ai rendu les deux mises en page et je te les montre côte à côte.', streaming: false },
  {
    kind: 'tool',
    id: 't1',
    ts: Date.now() - 120_000,
    name: 'mcp__escouade__show_page',
    input: { title: 'Deux mises en page du tableau de bord', template: 'compare', data: {} },
    status: 'ok',
    result: { text: JSON.stringify({ browser: 'page:p-3', page: 'p-3', version: 1, opened: 'beside', ask: 'a-7', bytes: 1, files: 1 }), isError: false },
  },
  { kind: 'text', id: 'x2', text: 'Choisis dans la page : je reprends dès que ta réponse arrive.', streaming: false },
];

function etatPage(choisi: boolean): Etat {
  return {
    projects: [atelier],
    agents,
    ui: { selectedAgent: { p1: 'a1' } },
    git: { p1: gitInfo() },
    settings: { mcpEnabled: true },
    conversations: { a1: CONV_PAGE },
    handlers: {
      agentpage_info: () => pageRow(choisi),
      inbox_list: () => ({ enabled: true, asks: [DEMANDE], open: 1 }),
    },
    apres: async ({ emit }) => {
      emit({ type: 'agentPage', page: { page: 'p-3', phase: 'shown' } } as UiEvent);
      await attendre('.agentpage');
      app.setPaneRatio('', 0.34);
      await dessinePage('.agentpage .room', 'comparer.html', choisi ? '&choisi=b' : '');
      // « Envoyer » is armed a moment after the proposal.
      if (choisi) await new Promise((r) => setTimeout(r, 1200));
    },
  };
}

ETATS.page = etatPage(false);
ETATS['page-choisie'] = etatPage(true);

// ---------- 2. The Workspace (#45, ESC-689): the canvas of the agents, each with its workflow step; « Workflows ». ----------

const agentsAuTravail = [
  agent({ id: 'a1', name: 'front-1', projectId: 'p1', cwd: '/demo/atelier', status: 'running' }),
  agent({ id: 'a2', name: 'testeur', projectId: 'p1', cwd: '/demo/atelier', status: 'waiting' }),
  agent({ id: 'a3', name: 'rédacteur', projectId: 'p1', cwd: '/demo/atelier', status: 'running', ticketId: 't1' }),
];
const PISTES = () => [
  track('a1', {
    step: 'gsd:plan-phase',
    passages: [
      passage(1, 'gsd:discuss-phase', 0, 4, { door: '/gsd:discuss-phase 3' }),
      passage(2, 'gsd:plan-phase', 4, 9),
      passage(3, 'gsd:execute-phase', 13, 21),
      passage(4, 'gsd:verify-work', 34, 3, { outcome: 'back', reason: '2 écarts sur le filtre', backTo: 'gsd:plan-phase' }),
      passage(5, 'gsd:plan-phase', 37, 4, { pass: 2, outcome: 'running' }),
    ],
  }),
  track('a2', { step: 'gsd:execute-phase', passages: [passage(1, 'gsd:execute-phase', 0, 3, { outcome: 'running' })] }),
];
function etatWorkspace(liste: boolean): Etat {
  return {
    projects: [atelier],
    agents: agentsAuTravail,
    tickets: [ticket({ id: 't1', key: 'ATL-5', title: 'Écrire l’aide de l’export', column: 'doing', agentId: 'a3', iteration: 1, projectId: 'p1' })],
    ui: { selectedAgent: { p1: 'a1' } },
    git: { p1: gitInfo() },
    handlers: {
      get_workflows: () => scanOf(gsd()),
      effective_workflows: () => scanOf(gsd()).workflows,
      get_workflow_tracks: () => PISTES(),
    },
    apres: async () => {
      app.openWorkspace('p1');
      await attendre('[data-testid="card-step"]');
      const bouton = (re: RegExp) => [...document.querySelectorAll('button')].find((x) => re.test(x.textContent?.trim() ?? ''));
      if (!liste) return;
      bouton(/^Workflows/)?.click();
      await new Promise((r) => setTimeout(r, 200));
      (document.activeElement as HTMLElement | null)?.blur();
    },
  };
}
ETATS.workspace = etatWorkspace(false);
ETATS['workspace-workflows'] = etatWorkspace(true);

// ---------- 3. « Navigateurs » in the sidebar (ESC-660): « + », an address, its tab beside the agent. ----------

/** Draws the made-up project's page in a browser's room, where its webview would be, at the width the room shows. */
async function dessineOnglet(extra = '') {
  await new Promise((r) => setTimeout(r, 150));
  const room = await attendre('.devpage .room');
  const frame = room.querySelector<HTMLElement>('.frame');
  const place = frame ? `left:${frame.offsetLeft + 1}px;width:${frame.offsetWidth - 2}px` : undefined;
  await dessinePage('.devpage .room', 'connexion.html', extra, place);
  // Ready once its own page has laid out (the contour is measured there).
  await new Promise((r) => setTimeout(r, 100));
}

function etatNavigateur(ouvert: boolean): Etat {
  return {
    projects: [atelier],
    agents,
    ui: { selectedAgent: { p1: 'a1' } },
    git: { p1: gitInfo() },
    conversations: { a1: [] },
    handlers: {
      devpage_open: (a: any) => ({ agentId: a.agentId, projectId: 'p1', title: 'Atelier · Connexion', url: a.url, loaded: true }),
    },
    apres: async () => {
      (await attendre('button[aria-label="Nouveau navigateur"]')).click();
      const champ = (await attendre('input[aria-label="Adresse du nouveau navigateur"]')) as HTMLInputElement;
      champ.value = 'localhost:5173/connexion';
      champ.dispatchEvent(new Event('input', { bubbles: true }));
      if (!ouvert) return;
      champ.closest('form')!.requestSubmit();
      for (let i = 0; i < 40 && !app.pageShown; i++) await new Promise((r) => setTimeout(r, 50));
      await dessineOnglet();
    },
  };
}
ETATS.navigateur = etatNavigateur(false);
ETATS['navigateur-ouvert'] = etatNavigateur(true);

// ---------- 4. The agent drives the tab it opened for you (AB-7): scroll, highlight, a phone's width. ----------

const CONV_PILOTE = [
  { kind: 'user', id: 'u1', text: 'Le bouton de connexion passe sous le clavier sur téléphone, regarde.', images: 0, ts: Date.now() - 200_000, queued: false },
  { kind: 'tool', id: 't1', ts: Date.now() - 150_000, name: 'mcp__escouade__open_browser', input: { url: 'http://localhost:5173/connexion', where: 'beside' }, status: 'ok', result: { text: 'Ouvert à côté', isError: false } },
  { kind: 'tool', id: 't2', ts: Date.now() - 120_000, name: 'mcp__escouade__drive_browser', input: { browser: 'browser:b7', action: 'mode', mode: 'mobile' }, status: 'ok', result: { text: 'Largeur mobile, 390 px', isError: false } },
  { kind: 'tool', id: 't3', ts: Date.now() - 90_000, name: 'mcp__escouade__drive_browser', input: { browser: 'browser:b7', action: 'highlight', target: '#entrer', label: 'Passe sous le clavier' }, status: 'ok', result: { text: 'Mis en évidence', isError: false } },
  { kind: 'text', id: 'x1', text: 'Je te l’ai mis en largeur mobile et j’ai entouré le bouton. Je remonte le formulaire de 40 px ?', streaming: false },
];

ETATS['pilote-avant'] = {
  projects: [atelier],
  agents,
  ui: { selectedAgent: { p1: 'a1' } },
  git: { p1: gitInfo() },
  settings: { mcpEnabled: true },
  conversations: { a1: CONV_PILOTE.slice(0, 2) },
  apres: async ({ emit }) => {
    const view = { agentId: 'browser:b7', projectId: 'p1', title: 'Atelier · Connexion', url: 'http://localhost:5173/connexion', loaded: true, docTitle: 'Atelier · Connexion', opener: 'a1', driver: 'a1' };
    emit({ type: 'browserOpened', browser: { view, agent: 'a1', wish: 'beside', place: 'beside' } } as UiEvent);
    app.setPaneRatio('', 0.5);
    await dessineOnglet();
  },
};

ETATS.pilote = {
  projects: [atelier],
  agents,
  ui: { selectedAgent: { p1: 'a1' } },
  git: { p1: gitInfo() },
  settings: { mcpEnabled: true },
  conversations: { a1: CONV_PILOTE },
  apres: async ({ emit }) => {
    const view = { agentId: 'browser:b7', projectId: 'p1', title: 'Atelier · Connexion', url: 'http://localhost:5173/connexion', loaded: true, docTitle: 'Atelier · Connexion', opener: 'a1', driver: 'a1' };
    emit({ type: 'browserOpened', browser: { view, agent: 'a1', wish: 'beside', place: 'beside' } } as UiEvent);
    emit({ type: 'devPage', page: { agentId: 'browser:b7', phase: 'mode', url: 'mobile' } } as UiEvent);
    app.setPaneRatio('', 0.5);
    await dessineOnglet('&marque=Passe%20sous%20le%20clavier');
  },
};

// ---------- 5. « Page à soi » (AP-7): Réglages › Projets › Lancement. ----------

ETATS['page-a-soi'] = {
  projects: [project({ id: 'p1', name: 'Atelier', path: '/demo/atelier', customPage: 'design/revue-maison.html' })],
  agents,
  ui: { selectedAgent: { p1: 'a1' } },
  git: { p1: gitInfo() },
  handlers: { save_settings: () => [] },
  apres: async () => {
    app.modal = { kind: 'settings', page: 'launch', projectId: 'p1' } as typeof app.modal;
    const titre = await attendre('[data-testid="custom-page-from"]');
    titre.closest('section, .group, fieldset')?.scrollIntoView({ block: 'center' });
  },
};

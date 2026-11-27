// The atelier's states: made-up projects and agents only (no real customer, person, path or token).
import type { Agent, GitInfo, Project, Settings, Ticket, UiEvent, UiState } from '../../src/lib/types';
import { agent, gitInfo, project, ticket } from '../../src/test/ipc';
import { gsd, passage, scanOf, track } from '../../src/test/workflows';
import { app } from '../../src/lib/state.svelte';
import { t } from '../../src/lib/i18n';

/** The atelier's language (?lang=en): the made-up words of a state in that language too. */
const EN = new URLSearchParams(location.search).get('lang') === 'en';
const L = (fr: string, en: string) => (EN ? en : fr);

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
  agent({ id: 'a2', name: L('testeur', 'tester'), projectId: 'p1', cwd: '/demo/atelier' }),
  agent({ id: 'a3', name: L('rédacteur', 'writer'), projectId: 'p1', cwd: '/demo/atelier' }),
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
  f.src = `/.whatsup/atelier/pages/${fichier}?theme=${theme()}&lang=${EN ? 'en' : 'fr'}${extra}`;
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
  title: L('Quelle mise en page pour le tableau de bord ?', 'Which layout for the dashboard?'),
  body: L('Deux mises en page, rendues sur la page.', 'Two layouts, rendered on the page.'),
  options: [
    { id: 'a', label: L('Colonnes de chiffres', 'Columns of figures'), description: '' },
    { id: 'b', label: L('Cartes empilées', 'Stacked cards'), description: '' },
  ],
  multi: false,
  steps: [],
  recommend: { option: 'b', text: null, why: L('Les retards passent en tête.', 'Delays come first.') },
  unblocks: L('ATL-14 démarre sur la mise en page choisie', 'ATL-14 starts on the chosen layout'),
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
    title: L('Deux mises en page du tableau de bord', 'Two dashboard layouts'),
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
  {
    kind: 'user',
    id: 'u1',
    text: L(
      'Propose-moi deux mises en page pour le tableau de bord, je choisis.',
      'Suggest two layouts for the dashboard, I will pick one.',
    ),
    images: 0,
    ts: Date.now() - 300_000,
    queued: false,
  },
  {
    kind: 'text',
    id: 'x1',
    text: L(
      'J’ai rendu les deux mises en page et je te les montre côte à côte.',
      'I rendered both layouts and I am showing them side by side.',
    ),
    streaming: false,
  },
  {
    kind: 'tool',
    id: 't1',
    ts: Date.now() - 120_000,
    name: 'mcp__escouade__show_page',
    input: { title: L('Deux mises en page du tableau de bord', 'Two dashboard layouts'), template: 'compare', data: {} },
    status: 'ok',
    result: {
      text: JSON.stringify({ browser: 'page:p-3', page: 'p-3', version: 1, opened: 'beside', ask: 'a-7', bytes: 1, files: 1 }),
      isError: false,
    },
  },
  {
    kind: 'text',
    id: 'x2',
    text: L('Choisis dans la page : je reprends dès que ta réponse arrive.', 'Pick in the page: I carry on as soon as your answer comes.'),
    streaming: false,
  },
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
  agent({ id: 'a2', name: L('testeur', 'tester'), projectId: 'p1', cwd: '/demo/atelier', status: 'waiting' }),
  agent({ id: 'a3', name: L('rédacteur', 'writer'), projectId: 'p1', cwd: '/demo/atelier', status: 'running', ticketId: 't1' }),
];
const PISTES = () => [
  track('a1', {
    step: 'gsd:plan-phase',
    passages: [
      passage(1, 'gsd:discuss-phase', 0, 4, { door: '/gsd:discuss-phase 3' }),
      passage(2, 'gsd:plan-phase', 4, 9),
      passage(3, 'gsd:execute-phase', 13, 21),
      passage(4, 'gsd:verify-work', 34, 3, {
        outcome: 'back',
        reason: L('2 écarts sur le filtre', '2 gaps in the filter'),
        backTo: 'gsd:plan-phase',
      }),
      passage(5, 'gsd:plan-phase', 37, 4, { pass: 2, outcome: 'running' }),
    ],
  }),
  track('a2', { step: 'gsd:execute-phase', passages: [passage(1, 'gsd:execute-phase', 0, 3, { outcome: 'running' })] }),
];
function etatWorkspace(liste: boolean): Etat {
  return {
    projects: [atelier],
    agents: agentsAuTravail,
    tickets: [
      ticket({
        id: 't1',
        key: 'ATL-5',
        title: L('Écrire l’aide de l’export', 'Write the export help'),
        column: 'doing',
        agentId: 'a3',
        iteration: 1,
        projectId: 'p1',
      }),
    ],
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

const SIGNIN_TITLE = L('Atelier · Connexion', 'Atelier · Sign in');

// ---------- 3. « Navigateurs » in the sidebar (ESC-660): « + », an address, its tab beside the agent. ----------

/** Draws the made-up project's page in a browser's room, where its webview would be, at the width the room shows. */
async function dessineOnglet(extra = '', fichier = 'connexion.html') {
  await new Promise((r) => setTimeout(r, 150));
  const room = await attendre('.devpage .room');
  const frame = room.querySelector<HTMLElement>('.frame');
  const place = frame ? `left:${frame.offsetLeft + 1}px;width:${frame.offsetWidth - 2}px` : undefined;
  await dessinePage('.devpage .room', fichier, extra, place);
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
      devpage_open: (a: any) => ({ agentId: a.agentId, projectId: 'p1', title: SIGNIN_TITLE, url: a.url, loaded: true }),
    },
    apres: async () => {
      (await attendre(`button[aria-label="${t('devpage.browsers.new')}"]`)).click();
      const champ = (await attendre(`input[aria-label="${t('devpage.browsers.field')}"]`)) as HTMLInputElement;
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
  {
    kind: 'user',
    id: 'u1',
    text: L(
      'Le bouton de connexion passe sous le clavier sur téléphone, regarde.',
      'The sign-in button goes under the keyboard on a phone, have a look.',
    ),
    images: 0,
    ts: Date.now() - 200_000,
    queued: false,
  },
  {
    kind: 'tool',
    id: 't1',
    ts: Date.now() - 150_000,
    name: 'mcp__escouade__open_browser',
    input: { url: 'http://localhost:5173/connexion', where: 'beside' },
    status: 'ok',
    result: { text: L('Ouvert à côté', 'Opened beside'), isError: false },
  },
  {
    kind: 'tool',
    id: 't2',
    ts: Date.now() - 120_000,
    name: 'mcp__escouade__drive_browser',
    input: { browser: 'browser:b7', action: 'mode', mode: 'mobile' },
    status: 'ok',
    result: { text: L('Largeur mobile, 390 px', 'Mobile width, 390 px'), isError: false },
  },
  {
    kind: 'tool',
    id: 't3',
    ts: Date.now() - 90_000,
    name: 'mcp__escouade__drive_browser',
    input: { browser: 'browser:b7', action: 'highlight', target: '#entrer', label: L('Passe sous le clavier', 'Goes under the keyboard') },
    status: 'ok',
    result: { text: L('Mis en évidence', 'Highlighted'), isError: false },
  },
  {
    kind: 'text',
    id: 'x1',
    text: L(
      'Je te l’ai mis en largeur mobile et j’ai entouré le bouton. Je remonte le formulaire de 40 px ?',
      'I put it in mobile width and circled the button. Shall I move the form up by 40 px?',
    ),
    streaming: false,
  },
];

ETATS['pilote-avant'] = {
  projects: [atelier],
  agents,
  ui: { selectedAgent: { p1: 'a1' } },
  git: { p1: gitInfo() },
  settings: { mcpEnabled: true },
  conversations: { a1: CONV_PILOTE.slice(0, 2) },
  apres: async ({ emit }) => {
    const view = {
      agentId: 'browser:b7',
      projectId: 'p1',
      title: SIGNIN_TITLE,
      url: 'http://localhost:5173/connexion',
      loaded: true,
      docTitle: SIGNIN_TITLE,
      opener: 'a1',
      driver: 'a1',
    };
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
    const view = {
      agentId: 'browser:b7',
      projectId: 'p1',
      title: SIGNIN_TITLE,
      url: 'http://localhost:5173/connexion',
      loaded: true,
      docTitle: SIGNIN_TITLE,
      opener: 'a1',
      driver: 'a1',
    };
    emit({ type: 'browserOpened', browser: { view, agent: 'a1', wish: 'beside', place: 'beside' } } as UiEvent);
    emit({ type: 'devPage', page: { agentId: 'browser:b7', phase: 'mode', url: 'mobile' } } as UiEvent);
    app.setPaneRatio('', 0.5);
    await dessineOnglet(`&marque=${encodeURIComponent(L('Passe sous le clavier', 'Goes under the keyboard'))}`);
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

// ---------- 6. 2.2.18: sign in to a site in a tab (ESC-695), the agent never takes the screen (ESC-684), lend (AB-9). ----------

/**
 * The window a page opens to sign in (Exemple ID, made up): a window of Escouade's own over the app, its origin first in
 * its title (devpage.rs, popup_title), drawn where it would stand.
 */
async function dessineFenetre() {
  const sombre = theme() === 'dark';
  const c = sombre
    ? { barre: '#202020', fond: '#2b2b2b', texte: '#f3f3f3', doux: '#a6a6a6', bord: '#3d3d3d', ligne: '#353535' }
    : { barre: '#f3f3f3', fond: '#ffffff', texte: '#1b1b1b', doux: '#616161', bord: '#d6d6d6', ligne: '#ececec' };
  const w = document.createElement('div');
  w.style.cssText = `position:fixed;z-index:9999;left:960px;top:170px;width:460px;height:560px;border:1px solid ${c.bord};border-radius:8px;overflow:hidden;background:${c.fond};color:${c.texte};font:14px/1.45 'Segoe UI',system-ui,sans-serif;box-shadow:0 24px 64px rgb(0 0 0 / ${sombre ? 0.6 : 0.28})`;
  const compte = (nom: string, mail: string, init: string, teinte: string) =>
    `<div style="display:flex;align-items:center;gap:12px;padding:12px 4px;border-bottom:1px solid ${c.ligne}"><span style="width:34px;height:34px;border-radius:50%;background:${teinte};color:#fff;display:grid;place-items:center;font-weight:600">${init}</span><span><b style="display:block;font-weight:600">${nom}</b><span style="color:${c.doux};font-size:13px">${mail}</span></span></div>`;
  w.innerHTML = `<div style="height:32px;display:flex;align-items:center;justify-content:space-between;padding-left:12px;background:${c.barre};font-size:12px"><span>https://id.exemple.test — ${L('Connexion à Exemple ID', 'Sign in to Exemple ID')}</span><span style="display:flex;color:${c.doux}"><span style="width:46px;text-align:center">─</span><span style="width:46px;text-align:center">☐</span><span style="width:46px;text-align:center">✕</span></span></div>
<div style="padding:34px 36px"><div style="display:flex;align-items:center;gap:8px;font-weight:700;font-size:17px"><span style="width:22px;height:22px;border-radius:6px;background:#4b6bdc"></span>Exemple ID</div>
<h2 style="margin:26px 0 4px;font-size:22px;font-weight:600">${L('Choisis un compte', 'Choose an account')}</h2>
<p style="margin:0 0 18px;color:${c.doux}">${L('pour continuer vers Atelier', 'to continue to Atelier')}</p>
${compte('Camille Martin', 'camille@exemple.test', 'C', '#2f6f5e')}${compte('Camille · Studio', 'studio@exemple.test', 'S', '#8a5cd0')}
<div style="padding:14px 4px;color:#4b6bdc;font-weight:600">${L('Utiliser un autre compte', 'Use another account')}</div>
<p style="margin:26px 0 0;color:${c.doux};font-size:12px">${L('Exemple ID partagera ton nom et ton adresse avec Atelier.', 'Exemple ID will share your name and address with Atelier.')}</p></div>`;
  document.body.append(w);
}

function etatConnexion(fenetre: boolean): Etat {
  return {
    ...etatNavigateur(false),
    apres: async () => {
      (await attendre(`button[aria-label="${t('devpage.browsers.new')}"]`)).click();
      const champ = (await attendre(`input[aria-label="${t('devpage.browsers.field')}"]`)) as HTMLInputElement;
      champ.value = 'localhost:5173/connexion';
      champ.dispatchEvent(new Event('input', { bubbles: true }));
      champ.closest('form')!.requestSubmit();
      for (let i = 0; i < 40 && !app.pageShown; i++) await new Promise((r) => setTimeout(r, 50));
      await dessineOnglet('&sso=1');
      if (fenetre) await dessineFenetre();
    },
  };
}
ETATS.connexion = etatConnexion(false);
ETATS['connexion-fenetre'] = etatConnexion(true);

/** front-1 opened the tab for you: in its conversation, the card of open_browser. */
const CONV_ECRAN = [
  {
    kind: 'user',
    id: 'u1',
    text: L('Corrige la marge du formulaire de connexion, puis montre-moi.', 'Fix the sign-in form’s margin, then show me.'),
    images: 0,
    ts: Date.now() - 240_000,
    queued: false,
  },
  {
    kind: 'text',
    id: 'x1',
    text: L('C’est corrigé : je t’ouvre la page pour que tu voies.', 'Fixed: I am opening the page so you can see.'),
    streaming: false,
  },
  {
    kind: 'tool',
    id: 't1',
    ts: Date.now() - 60_000,
    name: 'mcp__escouade__open_browser',
    input: { url: 'http://localhost:5173/connexion', where: 'beside' },
    status: 'ok',
    result: { text: L('Ouvert à côté', 'Opened beside'), isError: false },
  },
];
/** What you were doing meanwhile: testeur's conversation. */
const CONV_AILLEURS = [
  {
    kind: 'user',
    id: 'u1',
    text: L('Relance les tests de l’export et dis-moi ce qui casse.', 'Run the export tests again and tell me what breaks.'),
    images: 0,
    ts: Date.now() - 180_000,
    queued: false,
  },
  {
    kind: 'text',
    id: 'x1',
    text: L(
      '42 tests passent, 1 échoue : la date de l’export sort en UTC. Je corrige ?',
      '42 tests pass, 1 fails: the export’s date comes out in UTC. Shall I fix it?',
    ),
    streaming: false,
  },
];
const VUE_ECRAN = {
  agentId: 'browser:b8',
  projectId: 'p1',
  title: SIGNIN_TITLE,
  url: 'http://localhost:5173/connexion',
  loaded: true,
  docTitle: SIGNIN_TITLE,
  opener: 'a1',
  driver: 'a1',
};
function etatEcran(revenu: boolean): Etat {
  return {
    projects: [atelier],
    agents,
    ui: { selectedAgent: { p1: revenu ? 'a1' : 'a2' } },
    git: { p1: gitInfo() },
    settings: { mcpEnabled: true },
    conversations: { a1: CONV_ECRAN, a2: CONV_AILLEURS },
    apres: async ({ emit }) => {
      emit({
        type: 'browserOpened',
        browser: { view: VUE_ECRAN, agent: 'a1', wish: 'beside', place: revenu ? 'beside' : 'waiting' },
      } as UiEvent);
      if (!revenu) {
        await attendre('.row-item.browser.waits');
        return;
      }
      app.setPaneRatio('', 0.5);
      await dessineOnglet();
    },
  };
}
ETATS.ecran = etatEcran(false);
ETATS['ecran-revenu'] = etatEcran(true);

/** « Laisser piloter par… » in the menu of a browser you opened: the project's agents. */
ETATS['laisser-piloter'] = {
  ...etatNavigateur(false),
  apres: async () => {
    await ETATS.connexion.apres!({ emit: () => {} });
    const row = await attendre('.row-item.browser');
    const r = row.getBoundingClientRect();
    row.dispatchEvent(
      new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 60, clientY: r.top + r.height / 2 }),
    );
    (await attendre('.menus [aria-haspopup="menu"]')).click();
    await new Promise((res) => setTimeout(res, 250));
  },
};

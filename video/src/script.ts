// The script: the scenes in order, what each shows and what the voice says over it.
// The voice's length makes the timeline (src/timeline.ts): a scene lasts as long as its lines.
// The tone is a product launch, not a tutorial: short lines, one idea each, and the picture does the explaining.

import type { FeatureId } from './features';

export type SceneId =
  | 'intro'
  | 'chaos'
  | 'projects'
  | 'agents'
  | 'chat'
  | 'composer'
  | 'notify'
  | 'git'
  | 'editor'
  | 'board'
  | 'loop'
  | 'test'
  | 'validate'
  | 'launch'
  | 'terminals'
  | 'stats'
  | 'remote'
  | 'more'
  | 'outro';

export interface Line {
  /** Unique within its scene: the scene's animations start on it (its cue). */
  id: string;
  /** As written in the subtitles. */
  text: string;
  /** As the voice says it, when the pronunciation table (SAY) is not enough. */
  say?: string;
  /** Seconds left after the line for the picture to finish what it shows. */
  hold?: number;
}

export interface SceneScript {
  id: SceneId;
  /** The scene's text on screen. */
  title: string;
  shows: FeatureId[];
  lines: Line[];
  /** Seconds before the first line (its entry), and after the last one. */
  lead?: number;
  tail?: number;
}

export const SCRIPT: SceneScript[] = [
  {
    id: 'intro',
    title: 'Tous tes Claude Code, dans une seule fenêtre.',
    shows: [],
    lead: 1,
    tail: 1.2,
    lines: [{ id: 'hello', text: 'Voici Escouade : le poste de pilotage de tous tes agents Claude Code.' }],
  },
  {
    id: 'chaos',
    title: '5 projets. 12 agents. 30 terminaux ?',
    shows: [],
    lines: [
      { id: 'mess', text: 'Cinq projets. Douze agents. Trente terminaux ouverts… Sérieusement ?', hold: 0.3 },
      { id: 'one', text: 'Escouade réunit tout ça dans une seule fenêtre.', hold: 0.2 },
    ],
  },
  {
    id: 'projects',
    title: 'Un onglet par projet.',
    shows: ['tabs', 'tab-indicators', 'tab-menu', 'new-project', 'git-detect', 'worktree-option'],
    lines: [
      {
        id: 'tabs',
        text: "Un projet, un onglet, une couleur : toute l'interface suit. Un point vert ? Un agent bosse. Et le chiffre ? Le nombre de fichiers modifiés.",
      },
      { id: 'menu', text: 'Glisse les onglets pour les ranger. Clic droit : renomme, recolore, ferme.', hold: 0.3 },
      { id: 'add', text: 'Nouveau projet ? Choisis son dossier : Escouade détecte le dépôt git… ou le crée.' },
      { id: 'options', text: 'Une couleur, un premier agent, et si tu veux, un worktree git par agent.', hold: 0.8 },
    ],
  },
  {
    id: 'agents',
    title: 'Autant d’agents que tu veux.',
    shows: ['parallel-agents', 'agent-card', 'auto-name', 'archive', 'agent-manage'],
    lines: [
      { id: 'many', text: 'Des agents ? Autant que tu veux, chacun avec sa conversation.' },
      { id: 'card', text: 'Sa carte dit tout : statut, modèle, temps de travail, tokens, coût, fichiers modifiés.' },
      { id: 'name', text: 'Ta première demande ? Haiku lui trouve un nom.', hold: 0.4 },
      { id: 'archive', text: 'Il a fini ? Archive-le : sa conversation reste à un clic.' },
      { id: 'manage', text: 'Double-clic pour le renommer. Et Supprimer arrête son processus et retire son worktree.', hold: 0.4 },
    ],
  },
  {
    id: 'chat',
    title: 'Un vrai chat, pas un terminal.',
    shows: [
      'markdown',
      'thinking',
      'tools',
      'diff-in-chat',
      'live-activity',
      'live-cost',
      'questions',
      'permissions',
      'mid-turn',
      'interrupt',
    ],
    lines: [
      { id: 'md', text: "Ici, c'est un vrai chat : markdown, code coloré, réflexion de Claude repliée, appels d'outils compacts." },
      { id: 'diff', text: "Déplie-les : le diff d'une modification, la sortie d'une commande.", hold: 0.4 },
      { id: 'live', text: "Pendant le tour, tu vois ce que fait l'agent. Ses tokens et son coût montent en direct." },
      { id: 'ask', text: 'Une question ? Une autorisation ? Tu réponds en un clic.', hold: 0.5 },
      { id: 'during', text: "Écris-lui pendant qu'il travaille : il en tient compte à l'étape suivante. Et Échap l'arrête net." },
    ],
  },
  {
    id: 'composer',
    title: 'Modèle, effort, mode : quand tu veux.',
    shows: ['model-effort-mode', 'model-version', 'attachments', 'mentions', 'slash', 'turn-card', 'agent-commit'],
    lines: [
      {
        id: 'menus',
        text: "Modèle, effort, mode : change quand tu veux. Fable, Opus, Sonnet ou Haiku, dans la version qu'utilise Claude Code.",
        hold: 0.3,
      },
      {
        id: 'attach',
        text: 'Glisse une image, un PDF, un fichier. Cite un fichier avec arobase, lance une commande avec slash.',
        hold: 0.3,
      },
      {
        id: 'done',
        text: 'Fini ? Une carte résume la tâche : durée, coût, fichiers. Un clic pour les revoir, un autre pour demander le commit.',
      },
    ],
  },
  {
    id: 'notify',
    title: 'Il t’attend ? Tu le sais tout de suite.',
    shows: ['blink', 'chime', 'turn-end-notif', 'sound-toggle', 'system-notif', 'ctrl-j', 'tray', 'resume-sessions'],
    lines: [
      {
        id: 'ask',
        text: "Un agent t'attend, ou a fini ? Son onglet et sa carte clignotent, et un carillon sonne, que tu peux couper.",
        hold: 0.3,
      },
      { id: 'toast', text: 'Escouade est en arrière-plan ? Une notification arrive, la barre des tâches clignote.', hold: 0.3 },
      { id: 'jump', text: "Contrôle J : direct au prochain agent qui t'attend.", hold: 0.4 },
      {
        id: 'tray',
        text: "Tu fermes la fenêtre ? Rien ne s'arrête : Escouade file dans la zone de notification. Et au redémarrage, chaque agent reprend sa session.",
      },
    ],
  },
  {
    id: 'git',
    title: 'Chaque modif, chaque branche, sous tes yeux.',
    shows: [
      'split-layout',
      'uncommitted',
      'diff-views',
      'full-diff',
      'git-graph',
      'commit-diff',
      'worktree-merge',
      'merge-strategy',
      'remote-sync',
    ],
    lines: [
      {
        id: 'split',
        text: "En moitié-moitié : la conversation à gauche, à droite les fichiers modifiés et leur diff, en direct pendant que l'agent écrit.",
      },
      { id: 'scope', text: 'Cet agent ou tout le projet. Côte à côte ou unifié. Et même en plein écran.', hold: 0.4 },
      {
        id: 'history',
        text: "L'historique ? Le graphe du dépôt, avec la branche de l'agent mise en avant. Un clic sur un commit, et voilà son diff.",
        hold: 0.4,
      },
      {
        id: 'merge',
        text: "Un worktree par agent ? Quand c'est prêt, un clic pour fusionner sa branche dans main, en merge ou en squash.",
        // The confirmation, then git's summary in a toast: time to read it.
        hold: 1.5,
      },
      { id: 'sync', text: 'Et la barre de statut suit ta branche : pull, push et fetch, en un clic.', hold: 0.4 },
    ],
  },
  {
    id: 'editor',
    title: 'Un éditeur, sans quitter l’app.',
    shows: ['editor', 'editor-marks', 'editor-save', 'editor-disk'],
    lines: [
      { id: 'open', text: "Un fichier à retoucher ? L'éditeur intégré s'ouvre sur le projet, ou sur le worktree d'un agent." },
      { id: 'look', text: 'Arborescence, onglets, coloration, recherche, et les lignes modifiées, marquées dans la marge.', hold: 0.3 },
      { id: 'save', text: "Contrôle S, c'est enregistré. Et si l'agent modifie ton fichier ouvert, l'éditeur te prévient.", hold: 0.4 },
    ],
  },
  {
    id: 'board',
    title: 'Des tickets, que des agents prennent seuls.',
    shows: ['board', 'tickets', 'max-loops', 'ticket-menu', 'manual-start', 'autopilot', 'parallel'],
    lines: [
      { id: 'switch', text: 'Et maintenant, le tableau ! À faire, en cours, à tester, terminé.', hold: 0.3 },
      {
        id: 'ticket',
        text: "Un ticket : un titre, une description, des critères d'acceptation… et un nombre maximum de boucles.",
        hold: 0.4,
      },
      { id: 'manage', text: 'Clic droit : modifie, passe en tête, supprime.', hold: 0.3 },
      {
        id: 'auto',
        text: "Lance un ticket à la main… ou passe en pilote auto : les agents prennent les tickets seuls, dans l'ordre, jusqu'au nombre d'agents en parallèle que tu as choisi.",
        hold: 0.6,
      },
    ],
  },
  {
    id: 'loop',
    title: 'Chaque agent boucle jusqu’au but.',
    shows: ['ticket-worktree', 'env-copy', 'loop', 'ticket-card', 'progress', 'blocked', 'blocked-notif', 'criteria-report', 'sidebar-tag'],
    lines: [
      {
        id: 'setup',
        text: 'Chaque ticket a son agent, sa branche, son worktree avec tes fichiers .env, et ses propres ports pour les tests.',
      },
      {
        id: 'loop',
        text: "Et l'agent boucle jusqu'au but : à chaque tour, il fait le bilan de ses critères. Il en manque ? Il repart.",
        hold: 0.4,
      },
      {
        id: 'card',
        text: "Sur la carte : la boucle en cours, les critères atteints, ce qui est déjà en place, et ce que fait l'agent en ce moment.",
      },
      { id: 'blocked', text: 'Un ticket bloqué libère sa place et te prévient. Un clic, et il repart.', hold: 0.4 },
      {
        id: 'report',
        text: 'Son bilan s’affiche en carte dans la conversation. Et dans la barre latérale, sa carte porte le ticket et sa boucle.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'test',
    title: 'Teste la fonctionnalité en un clic.',
    shows: ['review', 'ticket-notif', 'partial', 'test-launch', 'open-feature', 'test-logs', 'prepare-launch'],
    lines: [
      {
        id: 'ready',
        text: 'Tous les critères sont atteints ? Le ticket passe à tester, et tu es prévenu. Dernière boucle sans tout atteindre ? Il y passe aussi, en « Objectif partiel ».',
        hold: 0.4,
      },
      {
        id: 'run',
        text: "Un clic sur Tester : Escouade prépare le projet, lance ses serveurs sur les ports du ticket, attend qu'ils répondent…",
        hold: 0.3,
      },
      { id: 'open', text: 'Et ouvre ton navigateur directement sur la fonctionnalité !', hold: 0.6 },
      { id: 'logs', text: "Les logs ? Dans la section Lancement, sous le nom de l'agent." },
      { id: 'prepare', text: 'Et pour tout agent à worktree, Préparer le lancement lui demande comment le tester.' },
    ],
  },
  {
    id: 'validate',
    title: 'Valide, ou renvoie.',
    shows: [
      'validate-merge',
      'validate-tests',
      'commit-message',
      'cleanup',
      'pr-push',
      'pr-gh',
      'tests-fail-back',
      'board-settings',
      'conflicts',
      'reject',
      'done',
    ],
    lines: [
      {
        id: 'ok',
        text: 'Ça te va ? Valide ! Escouade relance les tests, commite avec un message généré, fusionne dans ta branche et retire le worktree.',
        hold: 0.4,
      },
      {
        id: 'settings',
        text: "Plutôt une pull request, ouverte avec gh ? Ou pousser la branche ? Tout se règle par projet, jusqu'aux conflits et aux agents en parallèle.",
        hold: 0.3,
      },
      {
        id: 'reject',
        text: 'Un souci ? Renvoie le ticket à son agent, en lui disant ce qui ne va pas. Et si un test échoue à la validation, il y repart tout seul.',
        hold: 0.4,
      },
      { id: 'done', text: 'Et les tickets terminés gardent leur issue, leurs boucles et leur coût.' },
    ],
  },
  {
    id: 'launch',
    title: 'Lance ton projet d’un clic.',
    shows: ['launch-commands', 'launch-all', 'launch-crash'],
    lines: [
      {
        id: 'config',
        text: "Ton projet, en un clic : configure ses commandes, le front, l'API, un worker, chacune avec son shell et son dossier.",
      },
      { id: 'run', text: 'Tout lancer, et tout démarre : chaque commande a son log et son statut en direct.', hold: 0.3 },
      { id: 'crash', text: "L'une plante ? Tu le sais tout de suite, avec son code de sortie.", hold: 0.4 },
    ],
  },
  {
    id: 'terminals',
    title: 'De vrais terminaux.',
    shows: ['terminals', 'mac-shells'],
    lines: [
      {
        id: 'shells',
        text: "Un terminal ? PowerShell, Git Bash ou WSL, et zsh, bash et fish sur Mac. Avec l'autocomplétion de ton shell, l'historique, et même tes programmes plein écran.",
        hold: 0.5,
      },
    ],
  },
  {
    id: 'stats',
    title: 'Tokens, coût, quotas : en direct.',
    shows: ['status-counts', 'quotas', 'today-cost', 'processes', 'usage-resume', 'stats', 'stats-ranges'],
    lines: [
      {
        id: 'quota',
        text: 'En bas, la barre de statut compte tes agents actifs, en attente et terminés. Et elle suit tes quotas : la session de cinq heures, la semaine, et quand ils repartent à zéro.',
      },
      { id: 'cost', text: 'Le coût du jour monte en direct, avec la mémoire et le processeur que prend Claude.', hold: 0.3 },
      { id: 'resume', text: "Limite d'usage atteinte ? L'agent reprend tout seul dès que ton quota revient.", hold: 0.3 },
      {
        id: 'page',
        text: 'Et les statistiques : tes tokens en entrée, cache et sortie, ton coût, par jour, semaine ou mois, par projet et par modèle.',
        hold: 0.5,
      },
    ],
  },
  {
    id: 'remote',
    title: 'Et depuis ton téléphone.',
    shows: ['remote-control'],
    lines: [
      {
        id: 'enable',
        text: "Tu t'éloignes ? Active le remote control : la session de l'agent s'ouvre sur claude.ai, et dans l'app Claude sur ton téléphone.",
      },
      { id: 'reply', text: 'Ce que tu y envoies s’affiche aussi dans Escouade.', hold: 0.5 },
    ],
  },
  {
    id: 'more',
    title: 'Et tout le reste.',
    shows: ['settings', 'idle-stop', 'proxy', 'shortcuts', 'updates', 'platforms'],
    lines: [
      {
        id: 'extras',
        text: "Et aussi : des réglages pour tout, jusqu'à l'arrêt des agents inactifs, un proxy réseau, des raccourcis clavier, et des mises à jour automatiques.",
        hold: 0.3,
      },
      { id: 'os', text: 'Escouade tourne sur Windows et sur macOS.', hold: 0.4 },
    ],
  },
  {
    id: 'outro',
    title: 'Gratuit, open source, pour Windows et macOS.',
    shows: ['open-source'],
    tail: 3,
    lines: [{ id: 'bye', text: 'Escouade est gratuit et open source. Télécharge-le sur GitHub, et donne une escouade à Claude !' }],
  },
];

/** The file and manifest key of a line. */
export const lineKey = (scene: SceneId, line: string) => `${scene}.${line}`;

/**
 * Words the voice says wrong as they are written, and how to write them for it to say them right: « git » with a
 * hard g (not « jit »), a chat room (not the cat), letters and dots read out. Whole words only.
 */
const SAY: [RegExp, string | ((m: string) => string)][] = [
  [/\bGitHub\b/g, 'Guite-Heub'],
  [/\b[Gg]it\b/g, (m) => (m[0] === 'G' ? 'Guite' : 'guite')],
  [/\bchat\b/g, 'tchatte'],
  [/(^|\s)\.env\b/g, (m) => m.replace('.env', 'point E N V')],
  [/\bclaude\.ai\b/g, 'claude point A I'],
  [/\bgh\b/g, 'G H'],
  [/\bmacOS\b/g, 'Mac'],
];

/** What the voice says for a line. */
export const spoken = (l: Line) =>
  l.say ?? SAY.reduce((t, [re, to]) => (typeof to === 'string' ? t.replace(re, to) : t.replace(re, to)), l.text);

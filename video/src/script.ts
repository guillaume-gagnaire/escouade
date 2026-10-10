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
  | 'integrations'
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
  /** The English subtitles: same moments, short and punchy; the voice keeps saying `text`. */
  en: string;
  /** As the voice says it, when the pronunciation table (SAY) is not enough. */
  say?: string;
  /** Seconds left after the line for the picture to finish what it shows. */
  hold?: number;
}

export interface SceneScript {
  id: SceneId;
  /** The scene's text on screen. */
  title: string;
  /** The same, for the English pictures (the website's English page). */
  titleEn: string;
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
    titleEn: 'All your Claude Code agents, in one window.',
    shows: [],
    lead: 1,
    tail: 1.2,
    lines: [
      {
        id: 'hello',
        text: 'Voici Escouade : le poste de pilotage de tous tes agents Claude Code.',
        en: 'Meet Escouade: the cockpit for all your Claude Code agents.',
      },
    ],
  },
  {
    id: 'chaos',
    title: '5 projets. 12 agents. 30 terminaux ?',
    titleEn: '5 projects. 12 agents. 30 terminals?',
    shows: [],
    lines: [
      {
        id: 'mess',
        text: 'Cinq projets. Douze agents. Trente terminaux ouverts… Sérieusement ?',
        en: 'Five projects. Twelve agents. Thirty terminals open… Seriously?',
        hold: 0.3,
      },
      {
        id: 'one',
        text: 'Escouade réunit tout ça dans une seule fenêtre.',
        en: 'Escouade brings it all together in a single window.',
        hold: 0.2,
      },
    ],
  },
  {
    id: 'projects',
    title: 'Un onglet par projet.',
    titleEn: 'One tab per project.',
    shows: ['tabs', 'tab-indicators', 'tab-menu', 'new-project', 'git-detect', 'worktree-option'],
    lines: [
      {
        id: 'tabs',
        text: "Un projet, un onglet, une couleur : toute l'interface suit. Un point vert ? Un agent bosse. Et le chiffre ? Le nombre de fichiers modifiés.",
        en: 'One project, one tab, one color: the whole interface follows. A green dot? An agent is at work. The number? Modified files.',
      },
      {
        id: 'menu',
        text: 'Glisse les onglets pour les ranger. Clic droit : renomme, recolore, ferme.',
        en: 'Drag tabs to sort them. Right-click: rename, recolor, close.',
        hold: 0.3,
      },
      {
        id: 'add',
        text: 'Nouveau projet ? Choisis son dossier : Escouade détecte le dépôt git… ou le crée.',
        en: 'New project? Pick its folder: Escouade detects the git repository… or creates it.',
      },
      {
        id: 'options',
        text: 'Une couleur, un premier agent, et si tu veux, un worktree git par agent.',
        en: 'A color, a first agent, and if you want, a git worktree per agent.',
        hold: 0.8,
      },
    ],
  },
  {
    id: 'agents',
    title: 'Autant d’agents que tu veux.',
    titleEn: 'As many agents as you want.',
    shows: ['parallel-agents', 'agent-card', 'auto-name', 'archive', 'agent-manage'],
    lines: [
      {
        id: 'many',
        text: 'Des agents ? Autant que tu veux, chacun avec sa conversation.',
        en: 'Agents? As many as you want, each with its own conversation.',
      },
      {
        id: 'card',
        text: 'Sa carte dit tout : statut, modèle, temps de travail, tokens, coût, fichiers modifiés.',
        en: 'Its card says it all: status, model, working time, tokens, cost, modified files.',
      },
      { id: 'name', text: 'Ta première demande ? Haiku lui trouve un nom.', en: 'Your first request? Haiku finds it a name.', hold: 0.4 },
      {
        id: 'archive',
        text: 'Il a fini ? Archive-le : sa conversation reste à un clic.',
        en: 'It’s done? Archive it: its conversation stays one click away.',
      },
      {
        id: 'manage',
        text: 'Double-clic pour le renommer. Et Supprimer arrête son processus et retire son worktree.',
        en: 'Double-click to rename it. And Delete stops its process and removes its worktree.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'chat',
    title: 'Un vrai chat, pas un terminal.',
    titleEn: 'A real chat, not a terminal.',
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
      {
        id: 'md',
        text: "Ici, c'est un vrai chat : markdown, code coloré, réflexion de Claude repliée, appels d'outils compacts.",
        en: 'This is a real chat: markdown, colored code, Claude’s thinking folded away, compact tool calls.',
      },
      {
        id: 'diff',
        text: "Déplie-les : le diff d'une modification, la sortie d'une commande.",
        en: 'Unfold them: a change’s diff, a command’s output.',
        hold: 0.4,
      },
      {
        id: 'live',
        text: "Pendant le tour, tu vois ce que fait l'agent. Ses tokens et son coût montent en direct.",
        en: 'During the turn, you see what the agent does. Its tokens and cost climb live.',
      },
      {
        id: 'ask',
        text: 'Une question ? Une autorisation ? Tu réponds en un clic.',
        en: 'A question? A permission? Answer in one click.',
        hold: 0.5,
      },
      {
        id: 'during',
        text: "Écris-lui pendant qu'il travaille : il en tient compte à l'étape suivante. Et Échap l'arrête net.",
        en: 'Write to it while it works: it takes it into account at the next step. And Esc stops it cold.',
      },
    ],
  },
  {
    id: 'composer',
    title: 'Modèle, effort, mode : quand tu veux.',
    titleEn: 'Model, effort, mode: whenever you want.',
    shows: ['model-effort-mode', 'model-version', 'attachments', 'mentions', 'slash', 'turn-card', 'agent-commit'],
    lines: [
      {
        id: 'menus',
        text: "Modèle, effort, mode : change quand tu veux. Fable, Opus, Sonnet ou Haiku, dans la version qu'utilise Claude Code.",
        en: 'Model, effort, mode: change them whenever you want. Fable, Opus, Sonnet or Haiku, in the version Claude Code uses.',
        hold: 0.3,
      },
      {
        id: 'attach',
        text: 'Glisse une image, un PDF, un fichier. Cite un fichier avec arobase, lance une commande avec slash.',
        en: 'Drop in an image, a PDF, a file. Mention a file with @, run a command with /.',
        hold: 0.3,
      },
      {
        id: 'done',
        text: 'Fini ? Une carte résume la tâche : durée, coût, fichiers. Un clic pour les revoir, un autre pour demander le commit.',
        en: 'Done? A card sums up the task: time, cost, files. One click to review them, another to ask for the commit.',
      },
    ],
  },
  {
    id: 'notify',
    title: 'Il t’attend ? Tu le sais tout de suite.',
    titleEn: 'It’s waiting for you? You know right away.',
    shows: ['blink', 'chime', 'turn-end-notif', 'sound-toggle', 'system-notif', 'ctrl-j', 'tray', 'resume-sessions'],
    lines: [
      {
        id: 'ask',
        text: "Un agent t'attend, ou a fini ? Son onglet et sa carte clignotent, et un carillon sonne, que tu peux couper.",
        en: 'An agent is waiting for you, or finished? Its tab and card blink, and a chime rings, which you can mute.',
        hold: 0.3,
      },
      {
        id: 'toast',
        text: 'Escouade est en arrière-plan ? Une notification arrive, la barre des tâches clignote.',
        en: 'Escouade in the background? A notification arrives, the taskbar flashes.',
        hold: 0.3,
      },
      {
        id: 'jump',
        text: "Contrôle J : direct au prochain agent qui t'attend.",
        en: 'Ctrl+J: straight to the next agent waiting for you.',
        hold: 0.4,
      },
      {
        id: 'tray',
        text: "Tu fermes la fenêtre ? Rien ne s'arrête : Escouade file dans la zone de notification. Et au redémarrage, chaque agent reprend sa session.",
        en: 'You close the window? Nothing stops: Escouade slips into the notification area. And on restart, every agent picks up its session.',
      },
    ],
  },
  {
    id: 'git',
    title: 'Chaque modif, chaque branche, sous tes yeux.',
    titleEn: 'Every change, every branch, in plain sight.',
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
        en: 'Split view: the conversation on the left, the modified files and their diff on the right, live while the agent writes.',
      },
      {
        id: 'scope',
        text: 'Cet agent ou tout le projet. Côte à côte ou unifié. Et même en plein écran.',
        en: 'This agent or the whole project. Side by side or unified. Even full screen.',
        hold: 0.4,
      },
      {
        id: 'history',
        text: "L'historique ? Le graphe du dépôt, avec la branche de l'agent mise en avant. Un clic sur un commit, et voilà son diff.",
        en: 'History? The repository graph, with the agent’s branch highlighted. Click a commit, and there’s its diff.',
        hold: 0.4,
      },
      {
        id: 'merge',
        text: "Un worktree par agent ? Quand c'est prêt, un clic pour fusionner sa branche dans main, en merge ou en squash.",
        en: 'A worktree per agent? When it’s ready, one click merges its branch into main, as a merge or a squash.',
        // The confirmation, then git's summary in a toast: time to read it.
        hold: 1.5,
      },
      {
        id: 'sync',
        text: 'Et la barre de statut suit ta branche : pull, push et fetch, en un clic.',
        en: 'And the status bar follows your branch: pull, push and fetch, in one click.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'editor',
    title: 'Un éditeur, sans quitter l’app.',
    titleEn: 'An editor, without leaving the app.',
    shows: ['editor', 'editor-marks', 'editor-save', 'editor-disk'],
    lines: [
      {
        id: 'open',
        text: "Un fichier à retoucher ? L'éditeur intégré s'ouvre sur le projet, ou sur le worktree d'un agent.",
        en: 'A file to tweak? The built-in editor opens on the project, or on an agent’s worktree.',
      },
      {
        id: 'look',
        text: 'Arborescence, onglets, coloration, recherche, et les lignes modifiées, marquées dans la marge.',
        en: 'File tree, tabs, syntax highlighting, search, and modified lines, marked in the margin.',
        hold: 0.3,
      },
      {
        id: 'save',
        text: "Contrôle S, c'est enregistré. Et si l'agent modifie ton fichier ouvert, l'éditeur te prévient.",
        en: 'Ctrl+S, and it’s saved. And if the agent changes your open file, the editor warns you.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'board',
    title: 'Des tickets, que des agents prennent seuls.',
    titleEn: 'Tickets that agents pick up on their own.',
    shows: ['board', 'tickets', 'max-loops', 'ticket-menu', 'manual-start', 'autopilot', 'parallel'],
    lines: [
      {
        id: 'switch',
        text: 'Et maintenant, le Kanban ! À faire, en cours, à tester, terminé.',
        en: 'And now, the Kanban! To do, in progress, to review, done.',
        hold: 0.3,
      },
      {
        id: 'ticket',
        text: "Un ticket : un titre, une description, des critères d'acceptation… et un nombre maximum de boucles.",
        en: 'A ticket: a title, a description, acceptance criteria… and a maximum number of loops.',
        hold: 0.4,
      },
      {
        id: 'manage',
        text: 'Clic droit : modifie, passe en tête, supprime.',
        en: 'Right-click: edit, move to the top, delete.',
        hold: 0.3,
      },
      {
        id: 'auto',
        text: "Lance un ticket à la main… ou passe en pilote auto : les agents prennent les tickets seuls, dans l'ordre, jusqu'au nombre d'agents en parallèle que tu as choisi.",
        en: 'Start a ticket by hand… or switch on autopilot: agents take tickets on their own, in order, up to the number of parallel agents you chose.',
        hold: 0.6,
      },
    ],
  },
  {
    id: 'loop',
    title: 'Chaque agent boucle jusqu’au but.',
    titleEn: 'Each agent loops until it gets there.',
    shows: ['ticket-worktree', 'env-copy', 'loop', 'ticket-card', 'progress', 'blocked', 'blocked-notif', 'criteria-report', 'sidebar-tag'],
    lines: [
      {
        id: 'setup',
        text: 'Chaque ticket a son agent, sa branche, son worktree avec tes fichiers .env, et ses propres ports pour les tests.',
        en: 'Each ticket gets its own agent, its branch, its worktree with your .env files, and its own ports for tests.',
      },
      {
        id: 'loop',
        text: "Et l'agent boucle jusqu'au but : à chaque tour, il fait le bilan de ses critères. Il en manque ? Il repart.",
        en: 'And the agent loops until it gets there: every turn, it checks its criteria. Some missing? It goes again.',
        hold: 0.4,
      },
      {
        id: 'card',
        text: "Sur la carte : la boucle en cours, les critères atteints, ce qui est déjà en place, et ce que fait l'agent en ce moment.",
        en: 'On the card: the current loop, the criteria met, what’s already in place, and what the agent is doing right now.',
      },
      {
        id: 'blocked',
        text: 'Un ticket bloqué libère sa place et te prévient. Un clic, et il repart.',
        en: 'A blocked ticket frees its slot and lets you know. One click, and it goes again.',
        hold: 0.4,
      },
      {
        id: 'report',
        text: 'Son bilan s’affiche en carte dans la conversation. Et dans la barre latérale, sa carte porte le ticket et sa boucle.',
        en: 'Its report shows as a card in the conversation. And in the sidebar, its card carries the ticket and its loop.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'test',
    title: 'Teste la fonctionnalité en un clic.',
    titleEn: 'Test the feature in one click.',
    shows: ['review', 'ticket-notif', 'partial', 'test-launch', 'open-feature', 'test-logs', 'prepare-launch'],
    lines: [
      {
        id: 'ready',
        text: 'Tous les critères sont atteints ? Le ticket passe à tester, et tu es prévenu. Dernière boucle sans tout atteindre ? Il y passe aussi, en « Objectif partiel ».',
        en: 'All criteria met? The ticket moves to To review, and you’re notified. Last loop without meeting them all? It goes there too, as “Partial goal”.',
        hold: 0.4,
      },
      {
        id: 'run',
        text: "Un clic sur Tester : Escouade prépare le projet, lance ses serveurs sur les ports du ticket, attend qu'ils répondent…",
        en: 'One click on Test: Escouade sets up the project, starts its servers on the ticket’s ports, waits for them to answer…',
        hold: 0.3,
      },
      {
        id: 'open',
        text: 'Et ouvre ton navigateur directement sur la fonctionnalité !',
        en: 'And opens your browser right on the feature!',
        hold: 0.6,
      },
      {
        id: 'logs',
        text: "Les logs ? Dans la section Lancement, sous le nom de l'agent.",
        en: 'The logs? In the Launch section, under the agent’s name.',
      },
      {
        id: 'prepare',
        text: 'Et pour tout agent à worktree, Préparer le lancement lui demande comment le tester.',
        en: 'And for any worktree agent, Prepare launch asks it how to test it.',
      },
    ],
  },
  {
    id: 'validate',
    title: 'Valide, ou renvoie.',
    titleEn: 'Approve, or send back.',
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
        en: 'Looks good? Approve! Escouade reruns the tests, commits with a generated message, merges into your branch and removes the worktree.',
        hold: 0.4,
      },
      {
        id: 'settings',
        text: "Plutôt une pull request, ouverte avec gh ? Ou pousser la branche ? Tout se règle par projet, jusqu'aux conflits et aux agents en parallèle.",
        en: 'Rather a pull request, opened with gh? Or push the branch? It’s all set per project, down to conflicts and parallel agents.',
        hold: 0.3,
      },
      {
        id: 'reject',
        text: 'Un souci ? Renvoie le ticket à son agent, en lui disant ce qui ne va pas. Et si un test échoue à la validation, il y repart tout seul.',
        en: 'Something’s off? Send the ticket back to its agent, telling it what’s wrong. And if a test fails at approval, it goes back by itself.',
        hold: 0.4,
      },
      {
        id: 'done',
        text: 'Et les tickets terminés gardent leur issue, leurs boucles et leur coût.',
        en: 'And finished tickets keep their outcome, their loops and their cost.',
      },
    ],
  },
  {
    id: 'integrations',
    title: 'Jira, Trello, GitHub : importés et synchronisés.',
    titleEn: 'Jira, Trello, GitHub: imported and synced.',
    shows: [
      'integrations-accounts',
      'integrations-link',
      'import',
      'criteria-extract',
      'external-card',
      'status-sync',
      'sync-comments',
      'auto-import',
    ],
    lines: [
      {
        id: 'connect',
        text: 'Tes tickets sont dans Jira, Trello ou GitHub ? Connecte ton compte, et lie une source à ton projet.',
        en: 'Your tickets are in Jira, Trello or GitHub? Connect your account, and link a source to your project.',
      },
      {
        id: 'import',
        text: "Un clic sur Importer : cherche, filtre, coche. Et leurs critères d'acceptation sont détectés tout seuls.",
        en: 'One click on Import: search, filter, tick. And their acceptance criteria are detected on their own.',
        hold: 0.3,
      },
      {
        id: 'arrive',
        text: "Les voilà dans le Kanban, avec leur clé d'origine.",
        en: 'There they are in the Kanban, with their original key.',
        hold: 0.3,
      },
      {
        id: 'sync',
        text: 'Escouade tient leur statut à jour, et commente le ticket quand il est prêt à tester, puis validé.',
        en: 'Escouade keeps their status up to date, and comments when it’s ready for review, then approved.',
        hold: 0.5,
      },
      {
        id: 'auto',
        text: 'Encore mieux : étiquette un ticket, et il arrive tout seul dans ton Kanban.',
        en: 'Even better: label a ticket, and it lands in your Kanban on its own.',
        hold: 0.6,
      },
    ],
  },
  {
    id: 'launch',
    title: 'Lance ton projet d’un clic.',
    titleEn: 'Launch your project in one click.',
    shows: ['launch-commands', 'launch-all', 'launch-crash'],
    lines: [
      {
        id: 'config',
        text: "Ton projet, en un clic : configure ses commandes, le front, l'API, un worker, chacune avec son shell et son dossier.",
        en: 'Your project, in one click: set up its commands, the front end, the API, a worker, each with its own shell and folder.',
      },
      {
        id: 'run',
        text: 'Tout lancer, et tout démarre : chaque commande a son log et son statut en direct.',
        en: 'Run all, and everything starts: each command has its log and live status.',
        hold: 0.3,
      },
      {
        id: 'crash',
        text: "L'une plante ? Tu le sais tout de suite, avec son code de sortie.",
        en: 'One crashes? You know right away, with its exit code.',
        hold: 0.4,
      },
    ],
  },
  {
    id: 'terminals',
    title: 'De vrais terminaux.',
    titleEn: 'Real terminals.',
    shows: ['terminals', 'mac-shells'],
    lines: [
      {
        id: 'shells',
        text: "Un terminal ? PowerShell, Git Bash ou WSL, et zsh, bash et fish sur Mac. Avec l'autocomplétion de ton shell, l'historique, et même tes programmes plein écran.",
        en: 'A terminal? PowerShell, Git Bash or WSL, and zsh, bash and fish on Mac. With your shell’s autocomplete, history, and even your full-screen programs.',
        hold: 0.5,
      },
    ],
  },
  {
    id: 'stats',
    title: 'Tokens, coût, quotas : en direct.',
    titleEn: 'Tokens, cost, quotas: live.',
    shows: ['status-counts', 'quotas', 'today-cost', 'processes', 'usage-resume', 'stats', 'stats-ranges'],
    lines: [
      {
        id: 'quota',
        text: 'En bas, la barre de statut compte tes agents actifs, en attente et terminés. Et elle suit tes quotas : la session de cinq heures, la semaine, et quand ils repartent à zéro.',
        en: 'At the bottom, the status bar counts your active, waiting and finished agents. And it tracks your quotas: the 5-hour window, the week, and when they reset.',
      },
      {
        id: 'cost',
        text: 'Le coût du jour monte en direct, avec la mémoire et le processeur que prend Claude.',
        en: 'Today’s cost climbs live, with the memory and CPU Claude takes.',
        hold: 0.3,
      },
      {
        id: 'resume',
        text: "Limite d'usage atteinte ? L'agent reprend tout seul dès que ton quota revient.",
        en: 'Usage limit reached? The agent resumes by itself as soon as your quota is back.',
        hold: 0.3,
      },
      {
        id: 'page',
        text: 'Et les statistiques : tes tokens en entrée, cache et sortie, ton coût, par jour, semaine ou mois, par projet et par modèle.',
        en: 'And the stats: your input, cache and output tokens, your cost, by day, week or month, by project and by model.',
        hold: 0.5,
      },
    ],
  },
  {
    id: 'remote',
    title: 'Et depuis ton téléphone.',
    titleEn: 'And from your phone.',
    shows: ['remote-control'],
    lines: [
      {
        id: 'enable',
        text: "Tu t'éloignes ? Active le remote control : la session de l'agent s'ouvre sur claude.ai, et dans l'app Claude sur ton téléphone.",
        en: 'Stepping away? Turn on remote control: the agent’s session opens on claude.ai, and in the Claude app on your phone.',
      },
      {
        id: 'reply',
        text: 'Ce que tu y envoies s’affiche aussi dans Escouade.',
        en: 'What you send there shows up in Escouade too.',
        hold: 0.5,
      },
    ],
  },
  {
    id: 'more',
    title: 'Et tout le reste.',
    titleEn: 'And everything else.',
    shows: ['settings', 'idle-stop', 'proxy', 'shortcuts', 'updates', 'platforms'],
    lines: [
      {
        id: 'extras',
        text: "Et aussi : des réglages pour tout, jusqu'à l'arrêt des agents inactifs, un proxy réseau, des raccourcis clavier, et des mises à jour automatiques.",
        en: 'And more: settings for everything, down to stopping idle agents, a network proxy, keyboard shortcuts, and automatic updates.',
        hold: 0.3,
      },
      { id: 'os', text: 'Escouade tourne sur Windows et sur macOS.', en: 'Escouade runs on Windows and macOS.', hold: 0.4 },
    ],
  },
  {
    id: 'outro',
    title: 'Gratuit, open source, pour Windows et macOS.',
    titleEn: 'Free, open source, for Windows and macOS.',
    shows: ['open-source'],
    tail: 3,
    lines: [
      {
        id: 'bye',
        text: 'Escouade est gratuit et open source. Télécharge-le sur GitHub, et donne une escouade à Claude !',
        en: 'Escouade is free and open source. Download it on GitHub, and give Claude a squad!',
      },
    ],
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
  [/\bJira\b/g, 'Djira'],
  [/\bmacOS\b/g, 'Mac'],
];

/** What the voice says for a line. */
export const spoken = (l: Line) =>
  l.say ?? SAY.reduce((t, [re, to]) => (typeof to === 'string' ? t.replace(re, to) : t.replace(re, to)), l.text);

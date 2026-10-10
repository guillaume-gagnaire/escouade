import type { Tree } from '../types';

// The launch commands, their logs, and the test launches.
export default {
  /** What a launch command is doing, in the list, above its log and on the lines of a test. */
  status: {
    ready: 'prêt',
    stopping: 'arrêt…',
    running: 'en cours',
    stopped: 'arrêté',
    done: 'terminé',
    crashed: 'planté',
    crashedCode: 'planté (code {code})',
  },
  /** The buttons of a command (« Stopper » and « Tout arrêter » are `common.stop` and `common.stopAll`). */
  action: { run: 'Lancer', restart: 'Relancer', runAll: 'Tout lancer' },
  /** What the log of a command and the toasts about it say. */
  launch: {
    wontStop: "« {name} » ne s'arrête pas",
    removed: '« {name} » a été retiré',
    restartedAt: '— relancé à {time} —',
    /** `{why}`: the reason the backend gave, as received. */
    couldNotStart: "« {name} » n'a pas pu démarrer : {why}",
    crashed: "« {name} » s'est arrêté en erreur (code {code})",
  },
  /** A shell is needed to open a terminal; `{expected}` lists the ones the system is expected to have. */
  term: { noShell: 'Aucun shell détecté ({expected}). Vérifie les réglages.' },
  /** The launch section of the sidebar. */
  section: {
    title: 'Lancement',
    configure: 'Commandes de lancement…',
    none: 'Aucune commande.',
    set: 'Configurer',
    suggest: 'Proposer des commandes',
  },
  /** The head and the empty state of the log of a command. */
  log: {
    searchKey: 'Rechercher ({key})',
    since: 'depuis {time}',
    notRun: 'Pas encore lancée.',
  },
  /** The terminal view. */
  terminal: {
    close: 'Fermer le terminal',
    exited: 'Processus terminé.',
    exitedCode: 'Processus terminé (code {code}).',
  },
  /** The steps of an agent's recipe, as launch commands, and its hidden characters spelled out. */
  recipe: {
    prepStep: 'Préparation {n}',
    process: 'processus {n}',
    /** The counts are 24 and over (spaces) or 2 and over (lines): always a plural. */
    spaces: '⟨{n} espaces⟩',
    spacesInvisible: '⟨{n} espaces ou invisibles⟩',
    emptyLines: '⟨{n} lignes vides⟩',
    emptyLinesInvisible: '⟨{n} lignes vides ou invisibles⟩',
  },
  /** The lines and the error of a test under way, written as it goes. */
  flow: {
    prepare: 'Préparation : {command}',
    running: 'en cours…',
    starting: 'démarrage…',
    started: 'démarré',
    waitingFor: 'en attente de {host}…',
    readyIn: 'prêt · {seconds} s',
    noAnswer: 'Pas de réponse de {url} après 3 min',
    notWaited: 'non attendu',
    code: 'code {code}',
    prepStopped: 'arrêtée',
    prepDone: 'terminée',
    prepFailed: 'Préparation en échec (code {code})',
    isolaFailed: 'isola up en échec (code {code})',
    stopped: 'Arrêté',
    couldNotStart: "« {name} » n'a pas pu démarrer",
    exited: "{name} s'est arrêté",
    notRunning: '{name} ne tourne pas',
  },
  /** The window of « ▶ Tester ». */
  testLaunch: {
    title: 'Tester {name}',
    steps: 'Étapes du lancement',
    viewLog: 'Voir le log',
    viewLogs: 'Voir les logs',
    reopen: 'Rouvrir',
    /** `{address}`: the address opened, in code style. */
    opened: 'Ouvert dans le navigateur : {address}',
    reading: 'Lecture du .isola.toml…',
    changed: 'La recette vient de changer : relis-la avant de lancer.',
    /** `{name}`: the agent. */
    isolaNotice:
      'isola lance les commandes de ce fichier dans ton shell, hors du mode de permission de Claude Code. {name} peut l’avoir écrit ou modifié.',
    isolaConfig: 'Configuration isola (.isola.toml)',
    recipeNotice: 'Ces commandes ont été écrites par {name}. Elles tournent dans ton shell, hors du mode de permission de Claude Code.',
    prepare: 'Préparation',
    launch: 'Lancement',
    root: 'la racine du worktree',
    variable: 'Variable',
    address: 'Adresse',
    opening: 'Ouverture',
    recipeChanged: 'La recette a changé : relance ▶ Tester.',
  },
} as const satisfies Tree;

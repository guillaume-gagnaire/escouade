import type { Tree } from '../types';

// The « Workspace »: the canvas of a project's agents, its compasses and its controls. The names of the
// agents and what they do are their own words: shown as they come, never translated.
export default {
  /** The entry of the menu of the « Éditeur » button, and that button when the editor is off. */
  open: 'Workspace',
  exit: 'Quitter le workspace',
  label: 'Toile des agents',
  empty: 'Aucun agent dans ce projet',
  /** The canvas of every project, when no project has an agent. */
  emptyAll: 'Aucun agent',
  count: { one: '{count} agent', other: '{count} agents' },
  questions: { one: '{count} question', other: '{count} questions' },
  zoomIn: 'Zoomer',
  zoomOut: 'Dézoomer',
  zoom: '{percent} %',
  fit: 'Ajuster',
  /** The line of a card whose agent waits for an answer. */
  waiting: 'attend ta réponse',
  /** The landing zone, south of a compass: what the agent delivered, its remote CI, its deployments. */
  land: {
    title: 'Zone d’atterrissage · {landed}, {coming}',
    landed: { one: '{count} posé', other: '{count} posés' },
    coming: '{count} en approche',
    empty: 'Rien n’a encore atterri',
    artifact: 'Artefact · version {n} · {ago}',
    open: 'Ouvrir ↗',
    /** The name of a page's button (or a ribbon card's address) for a screen reader. */
    openPage: 'Ouvrir {title}',
  },
  /** A card with a ribbon: a test run, a CI, a deployment. */
  ribbon: {
    label: '{kind}, {state}',
    output: 'Sortie',
    /** Its name for a screen reader: the card it opens. */
    outputOf: 'Sortie de {title}',
    checks: { one: '{count} contrôle, {failed} en échec', other: '{count} contrôles, {failed} en échec' },
    kind: { tests: 'Tests', ci: 'CI', deploy: 'Déploiement' },
    state: { running: 'en cours', success: 'réussi', failure: 'en échec', stopped: 'arrêté' },
  },
  /** The word on the dotted link from an agent to one it launched. */
  launched: 'a lancé',
  /** The same, said in text on the agent launched (the link is drawn alone). */
  launchedBy: 'lancé par {name}',
  /** A skill or a plugin, on its card above the agent's, and as a tag on a subagent's card. */
  tool: {
    label: '{kind} {name}, {calls}',
    skill: 'Skill',
    plugin: 'Plugin',
    calls: { one: '{count} appel', other: '{count} appels' },
    running: 'en cours',
    /** The tags of a subagent's card: its tools, named for a screen reader. */
    tags: 'Outils de {name}',
    /** The pile of the tools past the most recent, and its button once unfolded. */
    more: { one: '+ {count} outil', other: '+ {count} outils' },
    fold: 'Replier les outils',
  },
  /** A side of a compass folded at mid-height, a button that unfolds it. */
  pile: {
    west: 'Outils ×{count}',
    east: 'Équipe ×{count}',
    south: 'Livrables ×{count}',
    /** Its name for a screen reader: what a click does. */
    unfold: { west: 'Déplier les outils ({count})', east: 'Déplier l’équipe ({count})', south: 'Déplier les livrables ({count})' },
  },
  /** The team, east of the card: its subagents and background tasks. */
  team: {
    task: 'tâche {n}',
    write: 'Écrire à {name}',
    folded: { one: '+ {count} terminé', other: '+ {count} terminés' },
    /** The plan keeps 30 subagents at most: more may have ended. */
    foldedCapped: '+ 30+ terminés',
    failed: { one: '{count} échec', other: '{count} échecs' },
  },
  /** The card of an agent near (its compass's centre) and its bar to write to it. */
  card: {
    write: 'Écrire à {name}',
    settings: 'Paramètres',
    model: 'Modèle',
    effort: 'Effort',
    mode: 'Mode',
    attach: 'Joindre un fichier',
    allModels: 'Tous les modèles…',
    files: { one: '{count} fichier', other: '{count} fichiers' },
    label: '{name}, {state}',
  },
  /** The requests of an agent that waits, in its card. `fromSub`: on the agent's card, a subagent's question is on its own. */
  ask: { loading: 'Chargement de la question…', fromSub: 'question de {name}' },
  /**
   * The workflows in the canvas (ESC-689): the step on an agent's card, which opens its diagram with its path; and
   * « Workflows » of the floating bar, the project's workflows with the agents in each.
   */
  wf: {
    outcome: { running: 'en cours', done: 'faite', back: 'renvoyée', failed: 'en échec', stopped: 'arrêtée', left: 'quittée' },
    back: 'Renvoyé par {from} : {reason}',
    backPlain: 'Renvoyé par {from}',
    open: 'Voir le schéma de {workflow} et le chemin de {name}',
    entry: 'Workflows',
    entryTip: 'Les workflows de {project} et les agents qui les suivent',
    title: 'Workflows',
    close: 'Fermer (Échap)',
    diagram: 'Schéma',
    diagramTip: 'Le schéma de {name}, depuis une porte d’entrée au choix',
    following: { one: '{count} agent le suit', other: '{count} agents le suivent' },
    nobody: 'Aucun agent ne le suit',
    outside: 'hors du workflow',
    between: 'entre deux étapes',
    pathOf: 'Le schéma de {workflow} et le chemin de {name}',
  },
} as const satisfies Tree;

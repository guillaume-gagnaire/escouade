import type { Tree } from '../types';

// The plan an agent follows: the banner of its conversation, the line of its card. The titles of the tasks, the
// names of the subagents and what they do are the agent's own words: shown as they come, never translated.
export default {
  /** The tag at the head of the banner. */
  badge: 'Plan',
  /** The banner of an agent with no task list that has subagents running. */
  badgeSubs: 'Sous-agents',
  /** « 2/7 tâches »: `count` is the number of tasks in all. */
  tasks: { one: '{done}/{count} tâche', other: '{done}/{count} tâches' },
  percent: '{percent} %',
  subsRunning: { one: '{count} sous-agent actif', other: '{count} sous-agents actifs' },
  /** The button that folds and unfolds the banner, for a screen reader. */
  toggle: 'Plan : {title}',
  /** The tooltip of the title when the tasks come from a plan file. */
  source: 'Plan : {file}',
  /** The list of the tasks, for a screen reader. */
  list: 'Tâches du plan',
  /** The state of a task, said in words next to its icon. */
  state: { done: 'Terminé', inProgress: 'En cours', todo: 'À faire', blocked: 'Bloqué' },
  /** « ↳ visuel-produit »: the subagent working on a task. The model follows, in its own tag. */
  subOn: '↳ {name}',
  /** « après 03, 05 »: the ranks of the tasks a task to do waits for. */
  after: 'après {list}',
  /** What a blocked task says: the agent asked a question or a permission. */
  waiting: 'En attente de ta réponse',
  /** The steps of a task a plan file counts: `count` is the number of steps in all. */
  stepsOf: { one: '{done}/{count} étape', other: '{done}/{count} étapes' },
  /** The handle under the list: its tooltip, and its name for a screen reader. */
  grip: 'Glisser pour redimensionner · double-clic pour réinitialiser',
  gripLabel: 'Hauteur de la liste des tâches',
  /** The line of the agent’s card in the sidebar. */
  mini: 'Plan {done}/{total}',
  miniSubs: { one: 'Plan {done}/{total} · {count} sous-agent', other: 'Plan {done}/{total} · {count} sous-agents' },
} as const satisfies Tree;

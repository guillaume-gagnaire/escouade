import type { Tree } from '../types';

// « Continuer sur <harness> » (ESC-88): an agent goes on on another harness by a handover, when the quota of its own is
// spent or when the user asks. `{harness}`: a harness's name (`engineName`).
export default {
  /** The button of the card of a turn stopped by the limit, and the items of the agent's menu. */
  continueOn: 'Continuer sur {harness}',
  /** The button that opens the menu of the other harnesses. */
  another: 'Continuer sur un autre harness',
  /** The item of the agent's menu that opens the list of harnesses. */
  menu: 'Continuer sur un autre harness…',
  /** Its quota came back: the way back, never taken by itself. */
  back: 'Revenir sur {harness}',
  backHint: 'Le quota {harness} est revenu : l’agent continue sur son harness actuel tant que tu ne le ramènes pas.',
  /** The item of the agent's menu while a turn runs. */
  waitTurn: 'Attends la fin du tour',
  /** The separator in the conversation. `{time}`: when the quota resets (« 18:00 »). */
  separator: {
    limitedUntil: 'Suite sur {harness} · quota {from} épuisé jusqu’à {time}',
    limited: 'Suite sur {harness} · quota {from} épuisé',
    asked: 'Suite sur {harness} · l’agent était sur {from}',
  },
  /** The first message of the agent on its new harness, folded. */
  file: {
    title: 'Dossier de passation',
    hint: 'Écrit par Escouade depuis la conversation : le premier message de l’agent sur {harness}.',
  },
  /** The mark of the harness on a turn, once the conversation went from one to another. */
  turnOn: 'sur {harness}',
  /** The settings of Réglages › Comptes et harness. */
  settings: {
    onLimit: 'Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés',
    onLimitDesc:
      'L’agent y passe seul, et le pilote auto le suit pour les tickets. Sinon il attend la remise à zéro. Le harness quitté n’est pas repris avant sa remise à zéro, et l’agent n’y revient jamais seul.',
    model: 'Modèle de passation · {harness}',
    /** The row's name in the register (« Avancé » names it, the search finds it). */
    modelName: 'Modèle de passation',
    modelDesc: 'Le modèle d’un agent qui passe sur ce harness.',
    best: 'Le meilleur ({model})',
  },
} as const satisfies Tree;

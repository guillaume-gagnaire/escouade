import type { Tree } from '../types';

// The Kanban: its columns, tickets and criteria.
export default {
  // The four columns (glossary): their names and what one says while it holds nothing.
  columns: { todo: 'À faire', doing: 'En cours', review: 'À tester', done: 'Terminé' },
  empty: {
    todo: "Ajoute un ticket : un agent le prendra dès qu'une place se libère.",
    doing: 'Aucun agent en boucle',
    review: 'Rien à tester',
    done: 'Aucun ticket terminé',
  },

  column: { newTicket: 'Nouveau ticket' },

  // The header of the board.
  header: {
    sub: {
      one: '{name} · {count} ticket · {looping} en boucle',
      other: '{name} · {count} tickets · {looping} en boucle',
    },
    claudeMissing: 'Claude Code introuvable — aucun ticket ne démarre',
    importTitle: 'Importer des tickets de Jira, Trello ou GitHub Issues',
    settingsTitle: 'Réglages du Kanban — {summary}',
    afterApproval: 'Après validation :',
    autopilotTitle: "Les tickets « À faire » partent seuls dès qu'une place se libère",
    autopilot: 'Pilote auto',
    autopilotOff: 'Pilote auto · off',
    resumeNow: 'Reprendre maintenant',
  },

  // A ticket's card.
  card: {
    openExternal: 'Ouvrir {key} dans {service}',
    syncError: 'Synchro avec {service} : {error}',
    resync: 'Resynchroniser',
    loop: 'Boucle {iteration}/{max}',
    partial: 'Objectif partiel',
    doneHeading: 'Ce qui a été fait',
    progressLabel: 'Avancement',
    moreSteps: { one: '{count} autre élément', other: '{count} autres éléments' },
    criteriaLabel: 'Critères',
    criteriaMetLabel: 'Critères atteints',
    criteriaMet: { one: '{met}/{total} critère', other: '{met}/{total} critères' },
    todoMeta: { one: '{count} critère · max {max} boucles', other: '{count} critères · max {max} boucles' },
    questionWaiting: 'Question en attente de ta réponse',
    resumes: 'Reprise {when}',
    settingUp: 'Prépare le worktree · {step}',
    thinking: 'Réfléchit',
    costLabel: 'Coût du ticket',
    launch: 'Lancer',
    launchTitle: 'Lancer {key} ?',
    launchAnyway: 'Lancer quand même',
    stopTests: 'Arrêter',
    test: 'Tester',
    resume: 'Reprendre',
    agentResolves: "L'agent résout",
    sendBack: 'Renvoyer',
    prioritize: 'Passer en tête',
    openAgent: "Ouvrir l'agent",
    removeTitle: 'Supprimer {key} ?',
    removeBody: 'Le ticket et sa description sont supprimés.',
    removeBodyImported: 'Le ticket et sa description sont supprimés. Il ne sera plus importé depuis {service}.',
    removeRunningTitle: 'Supprimer le ticket {key} ?',
    removeRunningBody: 'Son agent est archivé, avec son worktree.',
    approveTitle: 'Valider {key} ?',
    approveBody: 'Le worktree de son agent est supprimé après le merge.',
  },

  // The form of a ticket, new or edited.
  form: {
    discardTitle: 'Abandonner les modifications ?',
    discardBody: 'Ce que tu as saisi dans ce ticket ne sera pas enregistré.',
    discard: 'Abandonner',
    title: 'Titre du ticket',
    descriptionPlaceholder: 'Description (facultative)',
    description: 'Description',
    criteriaPlaceholder: "Critères d'acceptation, un par ligne",
    criteria: "Critères d'acceptation",
    maxLoops: 'Boucles max',
    after: 'Après',
    searchKey: 'Rechercher une clé',
    noMatch: 'Aucun ticket pour cette clé',
  },

  // The form that sends a ticket back to its agent.
  reject: { comment: 'Ce qui ne va pas', submit: 'Renvoyer' },

  // The report an agent gives when it hands a ticket over: its criteria, its progress, its test launch.
  report: {
    title: 'Bilan des critères',
    criterion: 'Critère {n}',
    doneHeading: 'Ce qui a été fait',
    progressLabel: 'Avancement',
    testLaunch: 'Lancement de test',
    prepare: 'Préparation',
    process: 'processus {n}',
    test: 'Tester',
  },
} as const satisfies Tree;

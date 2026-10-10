import type { Tree } from '../types';

// Jira, Trello and GitHub: the integrations tab and the import of tickets.
export default {
  // What each service calls the place its tickets are in (the project, the board, the repository).
  container: { jira: 'Projet', trello: 'Tableau', github: 'Dépôt' },
  // The search field of the import, by service.
  searchHint: {
    jira: 'Rechercher par clé ou par texte…',
    trello: 'Rechercher une carte…',
    github: 'Rechercher une issue…',
  },
  // The toast after an import: « 3 tickets importés depuis Jira et Trello ».
  imported: { one: '{count} ticket importé depuis {from}', other: '{count} tickets importés depuis {from}' },

  // The accounts group of the settings tab.
  accounts: {
    title: 'Comptes connectés',
    note: "Les jetons restent sur cette machine, à part des réglages, et ne servent qu'aux appels de ces services.",
    connected: 'Connecté · {label}',
    notConnected: 'Non connecté',
    keychainUnavailable: 'Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json.',
    keychainUnreadable: 'Trousseau du système illisible : relance Escouade ou reconnecte le compte.',
    disconnect: 'Déconnecter',
    connectEllipsis: 'Connecter…',
    connect: 'Connecter',
    checking: 'Vérification…',
    formLabel: 'Connexion à {service}',
    site: 'Site',
    email: 'E-mail',
    apiToken: "Jeton d'API",
    createAtlassianToken: "Créer un jeton d'API Atlassian",
    apiKey: "Clé d'API",
    token: 'Jeton',
    getApiKey: "Obtenir une clé d'API",
    getTokenForKey: 'Obtenir un jeton pour cette clé',
    githubToken: 'Jeton {hint}',
    githubTokenHint: '(vide : celui de gh)',
    createGithubToken: 'Créer un jeton GitHub',
  },

  // The project's sources.
  sources: {
    title: 'Sources liées à {project}',
    connectFirst: 'Connecte un compte ci-dessus pour lier une source à ce projet.',
    none: 'Aucun',
    // The choice of the source, by service: its container, then the service.
    pick: { jira: 'Projet Jira', trello: 'Tableau Trello', github: 'Dépôt GitHub Issues' },
  },

  // What a column gives the external tickets, and the comments written there.
  mapping: {
    title: 'Correspondance des statuts',
    note: "L'état que prend le ticket externe quand son ticket arrive dans la colonne ; « Commenter » y ajoute un commentaire (critères, issue de la validation…).",
    comment: 'Commenter',
    unchanged: '— inchangé',
    commentOnArrival: "Commenter à l'arrivée dans « {column} »",
    cell: '{service} — {column}',
  },

  sync: {
    title: 'Synchronisation',
    updateStatus: 'Mettre à jour le statut externe',
    updateStatusDesc: 'Quand un ticket importé change de colonne dans le Kanban',
    loopSummary: 'Publier un résumé à chaque boucle',
    loopSummaryDesc: "Critères atteints et notes de l'agent, sur le ticket externe",
    extractCriteria: "Extraire les critères d'acceptation",
    extractCriteriaDesc: 'Depuis la description (Jira, GitHub) ou la checklist (Trello) du ticket importé',
  },

  autoImport: {
    title: 'Import automatique',
    enable: 'Importer les tickets étiquetés',
    enableDesc: "Les tickets ouverts des sources liées qui portent l'étiquette arrivent dans « À faire »",
    label: 'Étiquette',
    labelHint: 'label Jira, étiquette Trello ou GitHub',
    every: 'Vérifier toutes les',
    minutes: '{n} min',
  },

  // The window that imports tickets.
  modal: {
    title: 'Importer des tickets',
    sub: 'Dans « À faire » du Kanban de {project}',
    noSource: 'Aucune source liée à ce projet',
    noSourceBody: 'Connecte Jira, Trello ou GitHub Issues, puis choisis le projet, le tableau ou le dépôt à associer à {project}.',
    thisProject: 'ce projet',
    linkSource: 'Lier une source',
    sources: 'Sources',
    pickedCount: { one: '{count} sélectionné', other: '{count} sélectionnés' },
    manage: '⚙ Gérer les sources',
    filters: 'Filtres',
    selectAll: 'Tout sélectionner',
    searching: 'Recherche…',
    results: { one: '{n} résultat', other: '{n} résultats' },
    shown: { one: '{n} affiché', other: '{n} affichés' },
    shownOf: { one: '{n} affiché sur {total}', other: '{n} affichés sur {total}' },
    noMatch: 'Aucun ticket ne correspond à la recherche.',
    criteriaFound: { one: '✓ {count} critère détecté', other: '✓ {count} critères détectés' },
    alreadyImported: 'Déjà dans le Kanban',
    showMore: 'Afficher plus',
    nonePicked: 'Aucun ticket sélectionné',
    picked: { one: '{n} ticket sélectionné', other: '{n} tickets sélectionnés' },
    maxLoops: 'Boucles max',
    importCount: { one: 'Importer {n} ticket', other: 'Importer {n} tickets' },
    allKnown: 'Ces tickets sont déjà dans le Kanban.',
  },
} as const satisfies Tree;

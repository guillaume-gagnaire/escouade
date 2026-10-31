import type { Tree } from '../types';

// Jira, Trello, GitHub, Linear, ClickUp, GitLab and Azure Boards: the integrations tab and the import of tickets.
export default {
  // What each service calls the place its tickets are in (the project, the board, the repository).
  container: {
    jira: 'Projet',
    trello: 'Tableau',
    github: 'Dépôt',
    linear: 'Équipe',
    clickup: 'Liste',
    gitlab: 'Projet',
    azure: 'Projet',
    other: 'Conteneur',
  },
  // The search field of the import, by service.
  searchHint: {
    jira: 'Rechercher par clé ou par texte…',
    trello: 'Rechercher une carte…',
    github: 'Rechercher une issue…',
    linear: 'Rechercher par identifiant (ENG-42) ou par titre…',
    clickup: 'Rechercher par id (#86abx3, ABC-12) ou par titre…',
    gitlab: 'Rechercher une issue (#12) ou par texte…',
    azure: 'Rechercher un élément de travail (#12) ou par titre…',
    other: 'Rechercher un ticket…',
  },
  // The toast after an import: « 3 tickets importés depuis Jira et Trello ».
  imported: { one: '{count} ticket importé depuis {from}', other: '{count} tickets importés depuis {from}' },

  // The two parts of the tab: where the tickets come from, and where the code is.
  parts: {
    sources: {
      title: 'Sources du Kanban',
      note: 'Import et synchro des tickets : un ticket importé suit sa colonne là-bas (état, commentaires).',
    },
    forges: {
      title: 'Hébergeurs de code',
      note: 'PR et MR des agents, CI et déploiements : un compte par instance, que chaque projet trouve par son remote.',
    },
  },

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
    createLinearKey: "Créer une clé d'API Linear",
    clickupKeyHint: 'pk_…',
    createClickupKey: "Créer une clé d'API ClickUp",
    // GitLab Issues and Azure Boards have no account of their own: their code host's serves.
    byForge: 'Compte de l’hébergeur {forge} : il se connecte et se déconnecte dans Hébergeurs de code.',
    // Azure Boards needs the right Work Items on the Azure DevOps token.
    workItems: 'Son jeton doit avoir le droit Work Items (lecture et écriture).',
    connectForge: 'Connecter {forge}…',
  },

  // The code hosts' accounts: one per instance, with what each check found.
  forges: {
    title: 'Instances connectées',
    note: "Une par adresse (github.com, celle de l'entreprise…). Le compte de github.com sert aussi à GitHub Issues, ceux de GitLab à GitLab Issues, ceux d’Azure DevOps à Azure Boards.",
    empty: 'Aucun hébergeur connecté.',
    connected: 'Connecté · {label}',
    unchecked: '{label} · pas encore vérifié',
    checking: 'Vérification…',
    expired: 'Jeton expiré ou révoqué · {label}',
    missing: 'Droits manquants : {scopes}',
    unreachable: 'Injoignable · {label}',
    unread: 'Trousseau du système illisible : relance Escouade ou reconnecte le compte.',
    viaCli: 'Jeton de {cli}',
    issues: 'Sert aussi à {source}',
    noTickets: 'Pas de tickets : PR et CI seulement',
    projects: { one: 'Projet : {list}', other: 'Projets : {list}' },
    noAccount: { one: 'Aucun compte, pour le projet {list}', other: 'Aucun compte, pour les projets {list}' },
    connectHost: 'Connecter {host}',
    connectEllipsis: 'Connecter un hébergeur…',
    recheck: 'Vérifier',
    disconnect: 'Déconnecter',
    disconnectHost: 'Déconnecter {host}',
    formLabel: 'Connexion à un hébergeur',
    kind: 'Hébergeur',
    address: 'Adresse',
    token: 'Jeton {hint}',
    tokenHint: '(vide : celui de {cli})',
    createToken: 'Créer un jeton',
    scopes: 'Droits à donner au jeton : {scopes}',
    guessing: "Recherche du type d'hébergeur…",
    unknownKind: "Type d'hébergeur introuvable : choisis-le.",
  },

  // The project's sources.
  sources: {
    title: 'Sources liées à {project}',
    connectFirst: 'Connecte un compte ci-dessus pour lier une source à ce projet.',
    none: 'Aucun',
    // The choice of the source, by service: its container, then the service.
    pick: {
      jira: 'Projet Jira',
      trello: 'Tableau Trello',
      github: 'Dépôt GitHub Issues',
      linear: 'Équipe Linear',
      clickup: 'Liste ClickUp',
      gitlab: 'Projet GitLab Issues',
      azure: 'Projet Azure Boards',
      other: 'Conteneur de la source',
    },
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
    extractCriteriaDesc:
      'Depuis la description (Jira, GitHub, Linear, GitLab), la checklist (Trello, ClickUp) ou le champ Acceptance Criteria (Azure Boards) du ticket importé',
  },

  autoImport: {
    title: 'Import automatique',
    enable: 'Importer les tickets étiquetés',
    enableDesc: "Les tickets ouverts des sources liées qui portent l'étiquette arrivent dans « À faire »",
    label: 'Étiquette',
    /** A column mapped to a label of the service rather than to one of its states. */
    labelState: 'Étiquette « {name} »',
    labelHint: 'label Jira, étiquette Trello, GitHub, Linear, ClickUp ou GitLab, tag Azure Boards',
    every: 'Vérifier toutes les',
    minutes: '{n} min',
  },

  // The window that imports tickets.
  modal: {
    title: 'Importer des tickets',
    sub: 'Dans « À faire » du Kanban de {project}',
    noSource: 'Aucune source liée à ce projet',
    noSourceBody:
      'Connecte Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards, puis choisis le projet, le tableau, le dépôt, l’équipe ou la liste à associer à {project}.',
    thisProject: 'ce projet',
    linkSource: 'Lier une source',
    // The project's own repository, on a code host whose source is connected.
    linkOwn: 'Lier {name} ({service})',
    linking: 'Liaison…',
    otherSource: 'Lier une autre source',
    sources: 'Sources',
    pickedCount: { one: '{n} sélectionné', other: '{n} sélectionnés' },
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

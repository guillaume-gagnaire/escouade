import type { Tree } from '../types';

// The MCP server: its switch, what it was asked, and how its tools read in the conversation.
export default {
  // The « Harness » tab of the settings: the server, given to the agents and declared in every harness installed, and
  // what it was asked.
  section: {
    title: 'Serveur MCP',
    /** The one switch (ESC-91, ESC-119): the agents of Escouade, and every harness installed outside Escouade, each with a token of its own. */
    enable: 'Les agents peuvent piloter Escouade',
    enableDesc:
      'Les agents d’Escouade, et chaque harness installé lancé hors d’Escouade (Claude Code, Codex, OpenCode, Copilot CLI, dans un terminal ou leur app), lisent tes projets, agents et tickets, et peuvent créer des tickets ou lancer des agents, ce qui dépense du quota, valider des tickets (tests, fusion, push, pull request) ou en supprimer. Un agent déjà lancé reçoit les outils au prochain démarrage de son process.',
    /** ESC-669: the tools of the browsers (list_browsers, read_browser…), under the server. */
    browsers: 'Agents et navigateurs',
    browsersDesc:
      'Les agents ouvrent un navigateur de leur projet à côté de leur conversation et le ferment (open_browser, close_browser), et lisent ceux de « Navigateurs » (list_browsers, read_browser, capture_browser, browser_console) et les pilotent geste par geste (drive_browser) : le lancement du projet sans demander ; une autre adresse, avec ton accord, une fois par origine (montrer y passe sans demander, agir attend ton accord) ; un artefact qu’ils ont publié, pour te le montrer. Ton premier geste dans la page leur reprend la main. Jamais les adresses d’Escouade, ni ce poste hors des ports déclarés, ni les cookies, le stockage, un fichier, un téléchargement ou un secret d’Escouade ; un mot de passe seulement sur le lancement et avec ton accord.',
    /** ESC-673: the tools of the agents' pages (show_page, get_page, close_page), under the server. */
    pages: 'Pages des agents',
    pagesDesc:
      'Les agents te montrent une page dans « Navigateurs » (show_page) : une revue, un schéma, des options à comparer, ou leur propre HTML, qu’ils font défiler et mettent en évidence (drive_browser). Escouade la copie et la sert à part, sans réseau, sans accès à Escouade ni à tes fichiers. Coupé, leurs outils disparaissent et leurs lignes se ferment ; rien n’est effacé.',
    /** ESC-423 (audit SEC-03): who among the agents answers another agent's permissions and questions. */
    answer: 'Les agents peuvent répondre aux demandes des autres agents',
    answerDesc:
      'Par answer_permission et answer_question, et les demandes de la boîte « À toi » comprises, une à une, quand tu les laisses au chef d’escouade (answer_ask). « Piloter Escouade » ne les donne pas : par défaut, seul toi (et Claude hors d’Escouade, pour les questions et les permissions) peux répondre aux demandes d’un agent.',
    answerNobody: 'Personne',
    answerChef: 'Seulement le chef d’escouade',
    answerAll: 'Tous les agents',
    answerChefDesc: 'Le chef d’escouade se choisit projet par projet, dans Réglages › Projets › Équipe.',
    answerAllWarn:
      'Ça retire l’humain de la boucle : un agent détourné par un texte qu’il a lu peut autoriser ce qu’un autre agent demande, sans que tu le voies.',
    /** The project's « Chef d'escouade » (ESC-423), in Réglages › Projets › Équipe, projet par projet. */
    chef: 'Chef d’escouade',
    chefDesc:
      'L’agent qui répond aux permissions et aux questions des autres agents de ce projet, quand « Les agents peuvent répondre… » vaut « Seulement le chef d’escouade » (Réglages › Agents). Aucun agent ne se désigne lui-même.',
    chefNone: 'Aucun',
    chefOff: 'Sans effet tant que Réglages › Agents › « Les agents peuvent répondre… » ne vaut pas « Seulement le chef d’escouade ».',
    /** The list of the declarations, under the switch: one line per account of each harness installed. */
    list: 'Déclarations dans les harness',
    working: 'Déclaration dans les harness installés…',
    /** `{harness}`: Claude, Codex, OpenCode, Copilot. `{name}`: the account. */
    where: '{harness} · compte {name}',
    declared: 'Déclaré',
    failed: 'Pas déclaré',
    notDeclared: 'Pas déclaré pour le compte {name} : {error}',
    serverFailed: 'Le serveur n’a pas démarré : {error}',
    /** The agents whose process started before the switch was on (« Outils d’Escouade au prochain démarrage » on their conversation). */
    nextStart: {
      one: '{count} agent déjà lancé aura les outils au prochain démarrage de son process : « Relancer » sur sa conversation.',
      other: '{count} agents déjà lancés auront les outils au prochain démarrage de leur process : « Relancer » sur leur conversation.',
    },
    /** `{shell}`: the shell the command is written for, PowerShell or sh. */
    command: 'À lancer à la main dans {shell} :',
    copy: 'Copier la commande',
    copied: 'Commande copiée',
    /** A harness whose user config Escouade writes itself: the entry to paste in it. */
    paste: 'À ajouter à la main dans {file} :',
    copyEntry: 'Copier l’entrée',
    copiedEntry: 'Entrée copiée',
    copyFailed: 'Copie impossible : {error}',
    note: 'Escouade écrit elle-même son serveur dans la config de chaque compte actif de chaque harness installé : ~/.claude.json (Claude Code), ~/.codex/config.toml (Codex), ~/.config/opencode/opencode.json (OpenCode), ~/.copilot/mcp-config.json (Copilot CLI) pour le compte Principal, le reste du fichier et ses commentaires gardés. Aucune commande n’est lancée : le jeton ne passe sur aucune ligne de commande. Il est écrit en clair dans ces fichiers, lisibles de ton seul compte : tout programme qui les lit peut piloter Escouade. Désactiver change le jeton, puis retire l’entrée partout. Un harness installé ou un compte ajouté plus tard est déclaré aussitôt.',
  },
  activity: {
    title: 'Activité MCP',
    list: 'Appels au serveur MCP, les plus récents en tête',
    empty: 'Aucun appel pour l’instant.',
    /** The result of a call (a request refused before it reached a tool has none). */
    outcome: { ok: 'Fait', refused: 'Refusé', error: 'Erreur' },
  },
  tools: {
    /** The badge of one of its tools: `tool` is its name in words (below), or as Claude names it. */
    named: 'Escouade · {tool}',
    listProjects: 'Lister les projets',
    listAgents: 'Lister les agents',
    listFolders: 'Lister les dossiers d’agents',
    createFolder: 'Créer un dossier d’agents',
    updateFolder: 'Modifier un dossier d’agents',
    deleteFolder: 'Supprimer un dossier d’agents',
    moveAgent: 'Ranger un agent',
    listTickets: 'Lister les tickets',
    getTicket: 'Lire un ticket',
    getTicketImage: 'Lire une image d’un ticket',
    getTicketFile: 'Lire un fichier d’un ticket',
    getUsage: 'Lire le quota',
    listBackups: 'Lister les copies des données',
    getBoard: 'Lire les réglages du Kanban',
    getWorkflow: 'Lire les workflows',
    getSpecTree: 'Lire l’arbre de la spec',
    readSpecPage: 'Lire une page de la spec',
    writeSpecPage: 'Écrire une page de la spec',
    searchSpec: 'Chercher dans la spec',
    listDecisions: 'Lister les décisions',
    readDecision: 'Lire une décision',
    proposeDecision: 'Proposer une décision',
    replaceDecision: 'Proposer une décision de remplacement',
    listDocuments: 'Lister les documents',
    readDocument: 'Lire un document',
    searchDocuments: 'Chercher dans les documents',
    addDocument: 'Ajouter un document',
    getDocsToShare: 'Lire ce que les docs ont à partager',
    shareDocs: 'Partager les docs',
    setBoard: 'Régler le Kanban',
    getAgentSummary: 'Résumer un agent',
    listRequests: 'Lister les demandes en attente',
    createTicket: 'Créer un ticket',
    updateTicket: 'Modifier un ticket',
    moveTicket: 'Déplacer un ticket',
    startTicket: 'Lancer un ticket',
    createAgent: 'Lancer un agent',
    sendMessage: 'Envoyer un message',
    stopAgent: 'Arrêter un agent',
    deleteAgent: 'Demander la suppression d’un agent',
    listBackgroundWork: 'Lister les sous-agents et tâches de fond d’un agent',
    listBrowsers: 'Lister les navigateurs',
    readBrowser: 'Lire un navigateur',
    openBrowser: 'Ouvrir un navigateur',
    closeBrowser: 'Fermer un navigateur',
    showPage: 'Montrer une page',
    getPage: 'Lire l’état d’une page',
    closePage: 'Fermer une page',
    captureBrowser: 'Capturer un navigateur',
    driveBrowser: 'Piloter un navigateur',
    browserConsole: 'Lire la console d’un navigateur',
    getBackgroundOutput: 'Lire la sortie d’une tâche de fond ou le rapport d’un sous-agent',
    stopBackgroundWork: 'Arrêter une tâche de fond ou un sous-agent',
    messageSubagent: 'Écrire à un sous-agent',
    answerPermission: 'Répondre à une demande d’autorisation',
    answerQuestion: 'Répondre à une question',
    reportProgress: 'Donner son avancement',
    splitTicket: 'Découper son ticket',
    approveTicket: 'Valider un ticket',
    rejectTicket: 'Renvoyer un ticket',
    resumeTicket: 'Reprendre un ticket',
    resolveConflict: 'Résoudre un conflit',
    dismissBlock: 'Écarter un blocage',
    deleteTicket: 'Supprimer un ticket',
    waitTicket: 'Attendre des tickets',
    // The box « À toi » (ESC-509).
    askUser: 'Déposer une demande dans « À toi »',
    updateAsk: 'Mettre à jour une demande',
    withdrawAsk: 'Retirer une demande',
    listAsks: 'Lister les demandes de « À toi »',
    getAsk: 'Lire une demande',
    waitAsk: 'Attendre une réponse',
    getAskFile: 'Lire un fichier d’une demande',
    answerAsk: 'Répondre à une demande laissée',
    claimInboxView: 'Prendre en charge une ligne d’« À toi »',
    settleInboxView: 'Cocher une ligne d’« À toi » comme réglée',
  },
  progress: {
    /** What a status line an agent reported (`report_progress`) is, for assistive technologies; the line is the agent's own words. */
    label: 'Ce que l’agent dit faire',
  },
  permission: {
    /** What a permission card lists: each argument the tool is given, by its name. */
    args: 'Ce que Claude donne à l’outil',
    argName: '{name} :',
    /** A value shown cut: how many characters are not, and where the whole request is. */
    cut: {
      one: '{n} caractère de plus n’est pas montré : lis la demande entière dans la conversation.',
      other: '{n} caractères de plus ne sont pas montrés : lis la demande entière dans la conversation.',
    },
  },
} as const satisfies Tree;

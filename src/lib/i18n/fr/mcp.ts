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
    note: 'Escouade déclare son serveur dans la config de chaque compte actif de chaque harness installé : ~/.claude.json (Claude Code, par claude mcp add), ~/.codex/config.toml (Codex), ~/.config/opencode/opencode.json (OpenCode), ~/.copilot/mcp-config.json (Copilot CLI) pour le compte Principal, le reste du fichier et ses commentaires gardés. Le jeton y est écrit en clair. Désactiver change le jeton, puis retire l’entrée partout. Un harness installé ou un compte ajouté plus tard est déclaré aussitôt.',
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
    listTickets: 'Lister les tickets',
    getTicket: 'Lire un ticket',
    getTicketImage: 'Lire une image d’un ticket',
    getTicketFile: 'Lire un fichier d’un ticket',
    getUsage: 'Lire le quota',
    getBoard: 'Lire les réglages du Kanban',
    getWorkflow: 'Lire les workflows',
    indexSearch: 'Chercher dans l’index du code',
    indexContext: 'Lire le contexte d’une tâche dans l’index',
    indexCallers: 'Chercher les appelants dans l’index',
    indexCallees: 'Chercher les appelés dans l’index',
    indexBody: 'Lire le corps d’un symbole dans l’index',
    indexImpact: 'Mesurer l’impact d’un changement',
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
    listBackgroundWork: 'Lister les sous-agents et tâches de fond d’un agent',
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

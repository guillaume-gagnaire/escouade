// Everything the French page says. en.ts says the same, key for key.
// The French guillemets hold non-breaking spaces ( ), so that « Importer » never splits over two lines.
import type { Catalog } from './catalog';
import { DOWNLOAD, SUPPORT } from './site';
import { docPath } from './docs';

export const fr: Catalog = {
  meta: {
    title: 'Escouade — l’environnement de développement pensé pour les agents',
    description:
      'Escouade est un environnement de développement agentique (ADE) gratuit et open source, pour Windows, macOS et Linux : planifie dans un Kanban, fais coder des agents en parallèle, chacun dans son worktree, puis relis, teste et livre sans quitter la fenêtre.',
    locale: 'fr_FR',
  },

  header: {
    sections: 'Sections',
    video: 'Vidéo',
    features: 'Fonctionnalités',
    install: 'Installer',
    roadmap: 'Feuille de route',
    faq: 'FAQ',
    github: 'GitHub',
    discord: 'Discord',
    docs: 'Documentation',
    download: 'Télécharger',
    language: 'Langue',
  },

  anchors: { features: 'fonctionnalites', install: 'installer', roadmap: 'feuille-de-route' },

  hero: {
    line1: 'L’environnement de développement',
    line2: 'pensé pour les agents.',
    lede: 'Planifier, coder, relire, tester, livrer : tout le cycle se fait avec des agents qui travaillent en parallèle, dans un ADE gratuit, open source, qui tourne sur ta machine.',
    download: 'Télécharger pour Windows, macOS et Linux',
    github: 'Voir sur GitHub',
    meta: 'Version {version} · Windows 10 et 11, macOS 11 et plus, Linux · Gratuit et open source (GPL-3.0)',
    video: 'Voir la vidéo',
  },

  spotlight: {
    title: 'Celui qui a besoin de toi passe devant.',
    text: 'Une question, une autorisation, une tâche finie : l’agent le signale, et tu réponds sans chercher dans quel terminal il attend. Essaie : réponds à tests-e2e.',
    points: [
      'Ctrl+J saute au prochain agent qui attend',
      'La vue d’ensemble montre tous les agents, ceux qui attendent en tête',
      'Carillon, notification système, onglet qui clignote : tu choisis ce qui te prévient',
    ],
  },

  video: { title: 'Escouade à l’œuvre.' },

  features: {
    title: 'Du ticket au merge.',
    lead: 'Les cinq étapes de toujours. La différence : une escouade d’agents les fait avec toi, en parallèle.',
    stages: {
      plan: {
        title: 'Planifier',
        text: 'Écris ce qu’il faut faire, ou importe-le : chaque ticket porte ses critères d’acceptation, et le pilote auto le confie à un agent.',
        points: [
          {
            title: 'Des captures et des fichiers dans le ticket',
            text: 'Colle une capture ou une maquette, joins un journal ou un tableur : l’agent du ticket les reçoit avec sa consigne.',
          },
          {
            title: 'Des tickets qui s’attendent',
            text: 'Un ticket peut en attendre d’autres avant de partir, ou reprendre une branche qui existe déjà.',
          },
          {
            title: 'Un pilote auto qui ménage ton quota',
            text: 'De 1 à 6 agents en parallèle, un nombre de boucles par ticket, et une pause près de la limite de tes quotas.',
          },
        ],
      },
      code: {
        title: 'Coder',
        text: 'Des agents en parallèle, chacun dans son worktree : tu suis leur travail en direct, et tu reprends la main dans l’éditeur ou un terminal.',
        points: [
          {
            title: 'Sous-agents et tâches de fond',
            text: 'Ce que lance un agent se voit dans la barre latérale : chaque sous-agent avec sa conversation, chaque commande de fond avec sa sortie en direct.',
          },
          {
            title: 'L’avancée d’un plan',
            text: 'Un bandeau dit où en est un agent qui suit un plan : les tâches faites, les sous-agents au travail, la tâche bloquée.',
          },
          {
            title: 'Tous les fichiers en pièce jointe',
            text: 'Une capture du bug, un PDF, une archive, un journal de 150 Mo : glisse-les dans le message, l’agent les lit.',
          },
        ],
      },
      review: {
        title: 'Relire et tester',
        text: 'Chaque changement se relit avant d’entrer : diffs, historique et branches, puis l’app lancée sur la fonctionnalité du ticket.',
        points: [
          {
            title: 'Des branches sans commande',
            text: 'Changer de branche, en créer, nettoyer celles qui sont mergées, lancer un agent sur une branche qui existe déjà.',
          },
          {
            title: 'De vrais terminaux',
            text: 'PowerShell, Git Bash et WSL sous Windows, zsh, bash et fish ailleurs, avec l’autocomplétion de ton shell, pour lancer tes tests à la main, jusque dans le worktree d’un agent.',
          },
          {
            title: 'Rechercher partout',
            text: 'Ctrl+K cherche dans les conversations, les commandes et les sorties d’outils de tous tes agents.',
          },
        ],
      },
      ship: {
        title: 'Livrer',
        text: 'Un ticket que tu valides part comme tu l’as réglé : commit, merge dans la branche cible, push ou pull request. Escouade fait les gestes, tu gardes la décision.',
        points: [
          {
            title: 'Un commit que tu relis',
            text: 'Rédigé par l’agent, ou un message proposé que tu relis avant de cliquer, dans la langue de ton équipe.',
          },
          {
            title: 'Merge, squash ou rebase',
            text: 'Le worktree d’un agent ou d’un ticket entre dans sa branche cible, puis il est nettoyé. Un conflit revient à l’agent, qui le résout.',
          },
          {
            title: 'Push ou pull request',
            text: 'La pull request d’un ticket s’ouvre avec sa description, son avancement et ses critères d’acceptation.',
          },
          {
            title: 'Tes .env restent chez toi',
            text: 'Les fichiers que git ignore sont copiés dans chaque worktree pour que l’app s’y lance, et ne sont jamais commités, poussés ni mergés.',
          },
        ],
      },
      steer: {
        title: 'Piloter',
        text: 'Tu mènes l’escouade sans la surveiller : un signal quand on t’attend, le coût et les quotas en direct, et une app que tes agents pilotent aussi.',
        points: [
          {
            title: 'Plusieurs comptes',
            text: 'Perso, pro : chacun ses quotas et ses statistiques, et un agent arrêté par la limite d’usage reprend sur un autre compte.',
          },
          {
            title: 'Un serveur MCP',
            text: 'Un agent, ou ton assistant hors d’Escouade, lit tes projets, crée et mène des tickets, écrit aux agents et répond à leurs questions, sous les garde-fous du pilote auto.',
          },
          {
            title: 'La vue d’ensemble',
            text: 'Tous les agents de tous les projets sur une page, ceux qui attendent ta réponse en tête.',
          },
        ],
      },
    },
    items: {
      agents: {
        title: 'Projets en onglets, agents en parallèle',
        text: 'Un onglet par projet, et dans chacun autant d’agents que tu veux : chacun sa conversation, son modèle, son effort et, si tu veux, son worktree git. Statut, tokens, coût et fichiers touchés, en direct.',
        alt: 'La fenêtre d’Escouade : les tuiles des projets à gauche, des agents sur Claude Code, Codex et Cursor, et la conversation de l’un d’eux avec son plan et le menu des modèles de son compte',
      },
      chat: {
        title: 'Un vrai chat, ou l’interface de Claude Code : à toi de choisir',
        text: 'Markdown, code coloré, appels d’outils qui se déplient sur un diff, questions et autorisations en cartes auxquelles tu réponds d’un clic. Tu préfères l’interface de Claude Code ? Un bouton y fait passer l’agent, dans un terminal, sans perdre sa session, et Escouade le suit toujours : notifications, coût, plan.',
        alt: 'Une conversation : un appel d’outil déplié sur son diff, les tests passés et deux captures d’écran envoyées par l’agent',
      },
      notifications: {
        title: 'Tu sais quand on t’attend',
        text: 'Une question, une tâche finie : l’onglet et la carte clignotent, un carillon sonne, une notification système dit ce qui est demandé. Ctrl+J saute au prochain agent qui attend, et fermer la fenêtre ne coupe rien.',
        alt: 'Un agent qui attend ta réponse : sa carte en tête de la liste, la pastille sur la tuile du projet et sa question avec ses choix',
      },
      git: {
        title: 'Git sous les yeux',
        text: 'Fichiers non commités par agent, diffs côte à côte, git graph du dépôt. Un commit rédigé par l’agent ou un message proposé que tu relis, puis le merge ou le squash d’un worktree dans sa branche.',
        alt: 'La conversation et les fichiers côte à côte : les changements non commités d’un projet à deux dépôts, et le diff de l’un d’eux',
      },
      editeur: {
        title: 'Un éditeur, sans quitter l’app',
        text: 'Arborescence, onglets, coloration et lignes modifiées dans la marge. Ctrl+clic va à une définition sans serveur de langage, Ctrl+P ouvre un fichier par son nom, Ctrl+Maj+F cherche dans tous les fichiers.',
        alt: 'L’éditeur intégré : l’arborescence du projet, des onglets et le code avec ses lignes modifiées dans la marge',
      },
      tableau: {
        title: 'Des tickets, que des agents prennent seuls',
        text: 'Le Kanban de chaque projet : écris un ticket et ses critères d’acceptation. Pilote auto activé, des agents les prennent, chacun dans son worktree, et bouclent jusqu’à ce que chaque critère soit atteint.',
        alt: 'Le Kanban d’un projet : des tickets à faire, deux en cours sur Codex et Cursor, un à tester avec ses critères atteints et les tickets mergés',
      },
      test: {
        title: 'Teste chaque ticket en un clic',
        text: 'Un ticket passe « À tester » : « Tester » te montre la recette écrite par l’agent, lance ses serveurs sur leurs ports réservés et ouvre ton navigateur sur la fonctionnalité.',
        alt: 'La fenêtre « Tester DEM-4 » : la préparation terminée, deux serveurs prêts et l’adresse ouverte dans le navigateur',
      },
      integrations: {
        title: 'Tes tickets Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, dans le Kanban',
        text: 'Lie une source à ton projet, « Importer », coche : les tickets arrivent avec leurs critères d’acceptation. Escouade tient leur statut à jour et commente le ticket d’origine quand il est prêt à tester, puis terminé.',
        alt: 'La fenêtre « Importer des tickets » : trois tickets Jira cochés, avec leurs critères d’acceptation détectés',
      },
      lancement: {
        title: 'Lance ton projet d’un clic',
        text: 'Les commandes qui lancent ton projet (front, API, worker…), écrites par toi ou proposées d’après le projet, chacune dans son terminal avec son statut en direct. Un plantage se voit tout de suite, avec son code.',
        alt: 'Les commandes de lancement d’un projet, dont une plantée, et le journal du serveur de développement',
      },
      stats: {
        title: 'Tokens, coût, quotas : en direct',
        text: 'La barre de statut suit ton quota de 5 h, ton quota de la semaine et le coût du jour, qui monte pendant que tes agents travaillent. Le détail par projet, par modèle, par agent et par ticket est dans les statistiques.',
        alt: 'La page des statistiques : tokens par jour sur deux semaines, coût par projet et par modèle',
      },
      remote: {
        title: 'Et depuis ton téléphone',
        text: 'Active le remote control sur un agent : sa session s’ouvre sur claude.ai et dans l’app Claude sur mobile, et ce que tu y envoies s’affiche aussi dans Escouade.',
        alt: 'Un agent en remote control : un message envoyé depuis claude.ai dans sa conversation, et son menu avec le lien claude.ai',
      },
    },
  },

  cards: [
    {
      title: 'Français et anglais',
      text: 'L’interface dans ta langue, et une langue à part pour les commits, les pull requests et les commentaires des tickets.',
    },
    { title: 'Limite d’usage ? Il reprend', text: 'Un agent arrêté par sa limite d’usage reprend seul dès que ton quota revient.' },
    { title: 'Toujours là', text: 'Fenêtre fermée, Escouade reste dans la zone de notification et reprend chaque session au redémarrage.' },
    {
      title: 'Derrière un proxy',
      text: 'Proxy HTTP(S) pour Claude, les quotas, les mises à jour, les intégrations et, si tu veux, les terminaux.',
    },
    {
      title: 'Mises à jour automatiques',
      text: 'Signées, téléchargées en arrière-plan, installées sans fenêtre quand aucun agent ne travaille.',
    },
    { title: 'Au clavier', text: 'Ctrl+1…9 les projets, Ctrl+N un agent, Ctrl+J celui qui attend, Ctrl+K chercher, Échap interrompre.' },
  ],

  install: {
    title: 'Prêt en trois étapes.',
    steps: [
      {
        title: 'Installe un harness',
        text: 'Les agents d’Escouade tournent sur le harness de ta machine : Claude Code, Codex, OpenCode ou GitHub Copilot CLI. Installe-le et connecte-toi une fois, avec ton propre compte.',
        link: { label: 'Les quatre harness', href: docPath('fr', 'harness') },
      },
      {
        title: 'Installe Escouade',
        text: 'Le .exe pour Windows 10 ou 11 (avec Git for Windows), le .dmg pour macOS 11 ou plus récent, sur Mac Apple Silicon ou Intel, l’AppImage ou le .deb pour Linux (x86-64).',
        link: { label: 'Dernière version', href: DOWNLOAD },
      },
      {
        title: 'Ouvre un projet',
        text: 'Choisis un dossier, crée un agent et écris ta première demande. Les suivants arrivent avec Ctrl+N (⌘N sur Mac).',
      },
    ],
  },

  roadmap: {
    title: 'Ce qui arrive ensuite.',
    lead: 'Ton hébergeur de code dans la boucle, les plugins et leur place de marché, et la dictée par Wispr Flow. Pas de date promise : le chantier est décrit en détail sur GitHub.',
    next: 'Version {version} · en préparation',
    items: [
      {
        title: 'Wispr Flow',
        text: 'La dictée de Wispr Flow dans la zone de saisie et la conversation vocale, comme service en ligne et comme app de bureau.',
      },
      {
        title: 'GitHub, GitLab, Bitbucket et Gitea',
        text: 'Les pull requests des tickets suivies jusqu’au merge, avec leur CI et leurs déploiements, sans quitter Escouade.',
      },
      {
        title: 'Les plugins et leur place de marché',
        text: 'Escouade s’étend par des plugins, et chaque intégration externe en devient un : hébergeurs, sources de tickets, harness. Une place de marché les rassemble, relus avant d’y entrer.',
      },
    ],
    plan: 'Le plan détaillé',
    changelog: 'Ce qui est déjà sorti',
  },

  faq: {
    title: 'Questions fréquentes.',
    items: [
      {
        q: 'Qu’est-ce qu’un ADE ?',
        a: 'Un environnement de développement agentique : l’endroit où tu développes avec des agents de bout en bout, pas seulement celui où tu leur parles. Dans Escouade, tu écris les tickets, des agents codent en parallèle, chacun dans son worktree, et tu relis, testes et livres au même endroit : Kanban, conversation, éditeur, terminaux, Git et tests.',
      },
      {
        q: 'Ça remplace mon éditeur ?',
        a: 'Pas forcément. L’éditeur d’Escouade suffit pour relire et retoucher ce que font les agents (arborescence, onglets, recherche dans tous les fichiers, aller à une définition), mais il n’a ni serveur de langage ni extensions. Garde ton IDE pour le reste : chaque worktree est un vrai dossier, que tu ouvres où tu veux.',
      },
      {
        q: 'Quel harness fait tourner les agents ?',
        a: 'Celui de ta machine, avec ton propre compte : Claude Code (abonnement Claude ou clé API), Codex (compte ChatGPT), OpenCode (le fournisseur de ton choix, modèles locaux compris) ou GitHub Copilot CLI (compte GitHub avec Copilot). Chaque agent a le sien, et la documentation les compare. Cursor CLI viendra plus tard.',
      },
      {
        q: 'C’est gratuit ?',
        a: 'Oui, et open source, sous licence GPL-3.0 (ou ultérieure). Escouade utilise ton propre harness et ton propre compte, sans intermédiaire. Si elle t’est utile, tu peux soutenir le projet :',
        link: { label: 'offrir un café', href: SUPPORT.fr },
      },
      {
        q: 'Où vont mes données ?',
        a: 'Nulle part : projets, conversations et statistiques restent sur ta machine, dans ~/.escouade/. Le réseau ne sert qu’aux harness eux-mêmes, à la lecture de tes quotas, aux mises à jour et, si tu les connectes, à Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards, où un ticket importé reçoit son statut et des commentaires.',
      },
      {
        q: 'Et mes jetons Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards ?',
        a: 'Ils restent sur ta machine : dans le trousseau du système (Gestionnaire d’identification Windows, Trousseau macOS), ou sous Linux dans le fichier des intégrations de ~/.escouade/, et ne servent qu’aux appels de ces services. Pour GitHub, Escouade peut aussi reprendre celui de gh.',
      },
      {
        q: 'Je préfère l’interface de Claude Code, je peux la garder ?',
        a: 'Oui. Le bouton « Vue Claude Code », dans l’en-tête d’un agent, le fait tourner dans l’interface de Claude Code elle-même, dans un terminal : tes commandes /, tes raccourcis, tes dialogues. « Vue Escouade » le ramène dans le chat, sans perdre la session. Un réglage ouvre directement les nouveaux agents dans l’interface de Claude Code. Les agents des tickets, que le pilote auto mène, restent dans le chat.',
      },
      {
        q: 'Ça marche sur Mac ou Linux ?',
        a: 'Oui. Sur Mac : macOS 11 ou plus récent, Apple Silicon comme Intel. Sous Linux (x86-64) : l’AppImage, qui se met à jour toute seule, ou le .deb pour Debian et Ubuntu.',
      },
      {
        q: 'Le pilote auto peut-il épuiser mon quota ?',
        a: 'Tu choisis combien d’agents travaillent en parallèle (de 1 à 6), combien de boucles chaque ticket a au plus, et à quel pourcentage de tes quotas le pilote auto se met en pause. Rien ne démarre tant qu’un agent attend la fin de sa limite d’usage, et un ticket arrivé à sa dernière boucle passe « À tester » avec « Objectif partiel ».',
      },
      {
        q: 'Mes fichiers .env partent-ils dans les commits ?',
        a: 'Non. Les fichiers du projet que git ignore (.env* par défaut) sont copiés dans le worktree de chaque agent pour que l’app s’y lance ; à la validation d’un ticket, Escouade refuse de les commiter, de les pousser ou de les merger.',
      },
      {
        q: 'Comment se font les mises à jour ?',
        a: 'L’app cherche une nouvelle version toutes les cinq minutes et la télécharge en arrière-plan. Elle s’installe sans fenêtre d’installeur, au redémarrage que tu choisis, ou d’elle-même quand aucun agent ne travaille et que tout est enregistré, après t’avoir prévenu 30 secondes avant.',
      },
      {
        q: 'C’est un produit Anthropic ?',
        a: 'Non. Escouade est un projet indépendant, non affilié à Anthropic, OpenAI ni GitHub. Claude et Claude Code sont des marques d’Anthropic, Codex d’OpenAI, GitHub Copilot de GitHub.',
      },
    ],
  },

  close: { title: 'Ton prochain environnement de développement.' },

  footer: {
    github: 'GitHub',
    discord: 'Discord',
    support: 'Offrir un café',
    license: 'Licence GPL-3.0',
    privacy: 'Confidentialité',
    legal: 'Mentions légales',
    note: 'Projet indépendant, non affilié à Anthropic, OpenAI ni GitHub. Claude et Claude Code sont des marques d’Anthropic, Codex d’OpenAI, GitHub Copilot de GitHub.',
  },

  notFound: { title: 'Page introuvable', text: 'Cette adresse ne mène nulle part.', home: 'Retour à l’accueil' },
  failure: { title: 'Une erreur est survenue', home: 'Retour à l’accueil' },

  demo: {
    status: { ready: 'Prêt', running: 'En cours', question: 'Question', done: 'Terminé', totest: 'À tester' },
    agents: 'Agents',
    loop: 'boucle {loop}/{max}',
    criteria: { one: '{count} critère atteint sur {total}', other: '{count} critères atteints sur {total}' },
    kanban: 'Kanban',
    projects: 'Projets',
    running: '{count} en cours',
    waiting: '{count} en attente',
    done: { one: '{count} terminé', other: '{count} terminés' },
    today: 'Aujourd’hui ≈ {amount}',
    pause: 'Mettre la démonstration en pause',
    caption: 'Démonstration animée · données fictives',
    description:
      ' : cinq agents du projet demo-api travaillent en parallèle ; l’un pose une question à laquelle tu peux répondre, un agent de ticket boucle sur ses critères jusqu’à « À tester ».',
  },

  squad: {
    names: { auth: 'refacto-auth', e2e: 'tests-e2e', docs: 'docs-api', login: 'fix-login' },
    ticketSlugs: ['paginer-les-users', 'filtrer-par-role', 'exporter-en-csv'],
    question: { text: 'Lancer toute la suite e2e (38 tests) ?', options: ['Oui', 'Seulement auth'] },
    results: {
      files: { one: '{count} fichier modifié', other: '{count} fichiers modifiés' },
      tests: { one: '{count} test passé', other: '{count} tests passés' },
      met: 'Critères atteints · à tester',
    },
  },

  docs: {
    title: 'Documentation',
    lead: 'Tout ce que fait Escouade, et comment t’en servir : de l’installation au pilote auto, chaque fonctionnalité et chaque réglage.',
    meta: {
      title: 'Documentation d’Escouade',
      description:
        'La documentation d’Escouade, l’environnement de développement agentique : installation, agents et conversation, Git et worktrees, Kanban et pilote auto, intégrations, comptes, réglages et raccourcis.',
    },
    pageTitle: '{title} · Documentation d’Escouade',
    groups: {
      start: { title: 'Démarrer', text: 'Ce qu’est Escouade, l’installer, choisir ses harness et faire tes premiers pas.' },
      agents: {
        title: 'Agents et conversation',
        text: 'Lancer des agents, leur parler, suivre leur travail et être prévenu quand ils ont besoin de toi.',
      },
      code: {
        title: 'Code et Git',
        text: 'Relire et commiter leurs changements, donner à chaque agent son worktree, éditer, lancer le projet.',
      },
      board: {
        title: 'Kanban et automatisation',
        text: 'Confier des tickets au pilote auto, les tester, les valider, les importer, piloter Escouade par MCP.',
      },
      usage: { title: 'Comptes et consommation', text: 'Plusieurs comptes par harness, les quotas, le coût et les statistiques.' },
      settings: { title: 'Réglages', text: 'Chaque onglet de la fenêtre des réglages, option par option.' },
      plugins: {
        title: 'Plugins',
        text: 'Ajouter des fonctionnalités à Escouade : le manifeste d’un plugin, son installation, son protocole, ses points d’extension.',
      },
      reference: { title: 'Référence', text: 'Les raccourcis, les données locales, les mises à jour et le dépannage.' },
    },
    pages: {
      introduction: 'Présentation',
      installation: 'Installation',
      harness: 'Les harness',
      'quick-start': 'Prise en main',
      interface: 'L’interface',
      'managing-agents': 'Les agents',
      chat: 'La conversation',
      plan: 'L’avancée d’un plan',
      overview: 'Suivre ses agents',
      notifications: 'Notifications',
      'claude-code-view': 'La vue terminal',
      'remote-control': 'Remote control',
      git: 'Git et commits',
      worktrees: 'Worktrees',
      branches: 'Branches',
      editor: 'L’éditeur',
      terminals: 'Les terminaux',
      launch: 'Lancer le projet',
      board: 'Le Kanban',
      autopilot: 'Le pilote auto',
      testing: 'Tester et valider',
      integrations: 'Tickets externes',
      'shared-board': 'Partager le Kanban',
      'docs-space': 'L’espace Docs',
      workflows: 'Les workflows',
      'shared-memory': 'Mémoire partagée',
      'code-index': 'Index du code',
      mcp: 'Le serveur MCP',
      accounts: 'Plusieurs comptes',
      usage: 'Quotas et coût',
      stats: 'Statistiques',
      'settings-application': 'Application',
      'settings-claude-code': 'Harness',
      'settings-accounts': 'Comptes',
      'settings-notifications': 'Notifications',
      'settings-projects': 'Projets',
      'settings-board': 'Kanban',
      'settings-integrations': 'Intégrations',
      'settings-terminals': 'Terminaux',
      'settings-network': 'Réseau',
      'settings-voice': 'Voix',
      'settings-about': 'À propos',
      'plugins/installer': 'Installer un plugin',
      'plugins/manifest': 'Le manifeste',
      'plugins/api': 'Le protocole',
      'plugins/events': 'Les événements',
      'plugins/sdk': 'Le SDK',
      'plugins/testing': 'Tester un plugin',
      'plugins/themes': 'Les thèmes',
      'plugins/publishing': 'Publier sur le store',
      shortcuts: 'Raccourcis clavier',
      'escouade-yml': '.escouade.yml',
      data: 'Données locales',
      updates: 'Mises à jour',
      troubleshooting: 'Dépannage',
    },
    nav: 'Pages de la documentation',
    menu: 'Sommaire',
    toc: 'Sur cette page',
    previous: 'Précédent',
    next: 'Suivant',
    edit: 'Modifier cette page sur GitHub',
    callouts: { note: 'À noter', tip: 'Astuce', warning: 'Attention' },
    search: {
      button: 'Rechercher',
      placeholder: 'Rechercher dans la documentation',
      empty: 'Rien trouvé pour « {query} ».',
      keys: '↑ ↓ pour choisir, Entrée pour ouvrir, Échap pour fermer',
      failed: 'La recherche n’a pas pu charger les pages. Vérifie ta connexion et rouvre-la.',
      close: 'Fermer',
    },
  },
};

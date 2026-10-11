// Everything the French page says. en.ts says the same, key for key.
import type { Catalog } from './catalog';
import { DOWNLOAD } from './site';

export const fr: Catalog = {
  meta: {
    title: 'Escouade — le poste de pilotage de tes agents Claude Code',
    description:
      'Escouade, le poste de pilotage de tes agents Claude Code : une app Windows et macOS gratuite et open source pour piloter plusieurs agents en parallèle, dans une seule fenêtre, et leur confier des tickets, même ceux de Jira, Trello ou GitHub, qu’ils mènent jusqu’au but.',
    locale: 'fr_FR',
  },

  header: {
    sections: 'Sections',
    video: 'Vidéo',
    features: 'Fonctionnalités',
    install: 'Installer',
    faq: 'FAQ',
    github: 'GitHub',
    language: 'Langue',
  },

  anchors: { features: 'fonctionnalites', install: 'installer' },

  hero: {
    line1: 'Une escouade de Claude.',
    line2: 'Une seule fenêtre.',
    lede: 'Escouade, le poste de pilotage de tes agents Claude Code : une app Windows et macOS pour piloter plusieurs agents en parallèle, avec projets en onglets, chat natif, git, éditeur, terminaux, statistiques, et un Kanban de tickets, écrits par toi ou importés de Jira, Trello ou GitHub, que des agents prennent seuls jusqu’au but.',
    download: 'Télécharger pour Windows et macOS',
    github: 'Voir sur GitHub',
    meta: 'Version {version} · Windows 10 et 11, macOS 11 et plus · Gratuit et open source (MIT)',
    video: 'Voir la vidéo',
  },

  features: {
    eyebrow: 'Fonctionnalités',
    title: 'Tout pour travailler avec plusieurs Claude',
    lead: 'Chaque agent avance de son côté ; toi, tu gardes la vue d’ensemble et tu interviens quand il le faut.',
    items: {
      agents: {
        title: 'Projets en onglets, agents en parallèle',
        text: 'Chaque projet a son onglet et sa couleur, qui teinte toute l’interface. Dans chacun, autant d’agents Claude Code que tu veux, chacun avec sa conversation, son modèle et son effort ; Haiku leur trouve un nom dès ta première demande.',
        points: [
          'Statut en direct : en cours, question, terminé',
          'Tokens, coût et fichiers touchés par agent',
          'Un worktree git par agent, si tu veux',
        ],
        alt: 'La fenêtre d’Escouade : les projets en onglets, la liste des agents et la conversation de l’un d’eux',
      },
      chat: {
        title: 'Un vrai chat, pas un terminal',
        text: 'Markdown, code coloré, appels d’outils compacts que tu déplies pour voir un diff ou la sortie d’une commande. Joins une image, un PDF ou un fichier, cite un @fichier, lance une /commande, et écris pendant que Claude travaille : il en tient compte à l’étape suivante. Ctrl+K retrouve un message dans les conversations de tous tes agents.',
        points: [
          'Les questions et autorisations de Claude en cartes, auxquelles tu réponds aussi au clavier',
          'Modèle (avec sa version), effort et mode modifiables à tout moment',
          'Une carte résume chaque tâche : durée, coût, fichiers modifiés',
        ],
        alt: 'Une conversation : réponse de Claude en markdown, appel d’outil déplié sur son diff',
      },
      notifications: {
        title: 'Tu sais quand on t’attend',
        text: 'Quand un agent pose une question ou termine, Escouade le signale : onglet et carte qui clignotent, carillon, notification système qui dit ce qui est demandé, barre des tâches qui clignote (ou icône du Dock qui rebondit). Fermer la fenêtre ne coupe rien : chaque agent reprend sa session au redémarrage.',
        points: [
          'Ctrl+J saute au prochain agent qui attend',
          'La vue d’ensemble montre tous les agents, ceux qui attendent en tête',
          'Tu choisis ce qui te prévient : questions, fins, erreurs, tickets',
        ],
        alt: 'Un agent en attente de réponse, sa question et sa carte dans la barre latérale',
      },
      git: {
        title: 'Git sous les yeux',
        text: 'Fichiers non commités par agent ou pour tout le projet, diff unifié ou côte à côte, git graph du dépôt avec la branche de l’agent en avant. La disposition moitié / moitié montre la conversation et les fichiers ensemble, mis à jour pendant que l’agent écrit.',
        points: [
          'Commit rédigé par l’agent, ou direct avec un message proposé que tu relis',
          'Merge ou squash d’un worktree dans sa branche de base, puis nettoyage',
          'Pull, push et fetch depuis la barre de statut',
        ],
        alt: 'La disposition moitié / moitié : la conversation, les fichiers non commités et leur diff côte à côte',
      },
      editeur: {
        title: 'Un éditeur, sans quitter l’app',
        text: 'Retouche un fichier du projet ou du worktree d’un agent sans changer de fenêtre : arborescence, onglets, coloration, recherche, et les lignes modifiées depuis le dernier commit marquées dans la marge. Ctrl+clic va à une définition sans serveur de langage, Ctrl+P ouvre un fichier par son nom, Ctrl+Maj+F cherche dans tous les fichiers.',
        points: [
          'Voir les changements dans le texte, et annuler un bloc',
          'Compare avec ce que l’agent vient d’écrire avant de choisir',
          'Renommer, supprimer vers la corbeille, ouvrir un terminal ici',
        ],
        alt: 'L’éditeur intégré : l’arborescence du worktree, des onglets et le code avec ses lignes modifiées',
      },
      tableau: {
        title: 'Des tickets, que des agents prennent seuls',
        text: 'Le Kanban de chaque projet : à faire, en cours, à tester, terminé. Écris un ticket avec ses critères d’acceptation ; en pilote auto, des agents le prennent, chacun dans son worktree avec ses propres ports, et bouclent jusqu’à ce que chaque critère soit atteint.',
        points: [
          'Pilote auto, de 1 à 6 agents en parallèle, en pause près de la limite de tes quotas',
          'Boucle, critères, avancement et coût en direct sur chaque carte',
          'Valider : tests, commit généré, merge, pull request ou push ; ou renvoyer à l’agent',
        ],
        alt: 'Le Kanban d’un projet : deux tickets en cours avec leur boucle et leurs critères, et les tickets terminés',
      },
      test: {
        title: 'Teste chaque ticket en un clic',
        text: 'Quand un ticket passe « À tester », « ▶ Tester » te montre la recette de l’agent, puis prépare son worktree, lance ses serveurs sur ses ports réservés, attend qu’ils répondent et ouvre ton navigateur directement sur la fonctionnalité développée.',
        points: [
          'La recette vient de l’agent : tu la lis avant qu’elle tourne',
          'Les logs restent dans la section Lancement, sous son nom',
          '« Préparer le lancement » pour tout agent à worktree',
        ],
        alt: 'La fenêtre « Tester DEM-6 » : préparation, serveurs prêts et adresse ouverte dans le navigateur',
      },
      integrations: {
        title: 'Tes tickets Jira, Trello et GitHub, dans le Kanban',
        text: 'Connecte Jira, Trello ou GitHub Issues et lie une source à ton projet. « Importer » cherche et filtre ses tickets : coche-les, ils arrivent dans le Kanban avec leurs critères d’acceptation. Escouade tient ensuite leur statut à jour et commente le ticket d’origine quand il est prêt à tester, puis terminé.',
        points: [
          'Critères d’acceptation repris de la description ou de la checklist',
          'Statut et commentaires synchronisés, colonne par colonne, et retentés en cas d’échec',
          'Import automatique, si tu l’actives, des tickets étiquetés claude-ready',
        ],
        alt: 'La fenêtre « Importer des tickets » : trois tickets Jira cochés, avec leurs critères d’acceptation détectés',
      },
      lancement: {
        title: 'Lance ton projet d’un clic',
        text: 'Configure les commandes qui lancent ton projet (front, API, worker…), chacune avec son shell et son dossier, ou laisse Claude lire le projet et te les proposer. Chaque commande tourne dans son terminal, avec son statut en direct.',
        points: [
          'Tout lancer, tout arrêter, relancer',
          'Un plantage se voit tout de suite, avec son code',
          'Arrêter coupe aussi ce que la commande a lancé',
        ],
        alt: 'Trois commandes de lancement, dont une plantée, et le journal du serveur de développement',
      },
      stats: {
        title: 'Tokens, coût, quotas : en direct',
        text: 'La barre de statut suit ton quota de session de 5 h, ton quota hebdomadaire, leur réinitialisation et le coût du jour, qui monte pendant que Claude travaille. Un agent arrêté par sa limite d’usage reprend tout seul quand le quota revient.',
        points: [
          'Tokens d’entrée, de cache et de sortie, par jour, semaine ou mois',
          'Coût par projet, par modèle, par agent et par ticket',
          'Mémoire et processeur pris par Claude',
        ],
        alt: 'La page des statistiques : tokens par jour, coût par projet et par modèle',
      },
      remote: {
        title: 'Et depuis ton téléphone',
        text: 'Active le remote control sur un agent : sa session s’ouvre sur claude.ai et dans l’app Claude sur mobile. Ce que tu y envoies s’affiche aussi dans Escouade.',
        points: [
          'Au cas par cas, d’un clic droit sur l’agent',
          'Même session après un redémarrage',
          'L’agent reste joignable tant que Escouade tourne',
        ],
        alt: 'Un téléphone sur claude.ai et Escouade qui affichent la même conversation',
      },
    },
  },

  cards: [
    { title: 'De vrais terminaux', text: 'PowerShell, Git Bash et WSL intégrés, avec l’autocomplétion de ton shell.' },
    {
      title: 'Limite d’usage ? Il reprend',
      text: 'Un agent arrêté par sa limite d’usage reprend tout seul dès que ton quota revient, et le Kanban attend avant de lancer un nouveau ticket.',
    },
    {
      title: 'Toujours là',
      text: 'Fermer la fenêtre ne coupe pas les agents : Escouade reste dans la zone de notification et reprend chaque session au redémarrage.',
    },
    {
      title: 'Derrière un proxy',
      text: 'Proxy HTTP(S) pour Claude, les quotas, les mises à jour, les intégrations et, si tu veux, les terminaux.',
    },
    {
      title: 'Mises à jour automatiques',
      text: 'Les nouvelles versions, signées, se téléchargent en arrière-plan et s’installent sans fenêtre, quand aucun agent ne travaille.',
    },
    {
      title: 'Au clavier',
      text: 'Ctrl+1…9 pour les projets, Ctrl+N pour un agent, Ctrl+J pour celui qui attend, Ctrl+Entrée pour autoriser, Ctrl+K pour chercher, Échap pour interrompre.',
    },
  ],

  install: {
    eyebrow: 'Installer',
    title: 'Prêt en trois étapes',
    steps: [
      {
        title: 'Installe Claude Code',
        text: 'Escouade pilote le Claude Code installé sur ta machine : installe-le et connecte-toi une fois, avec ton abonnement Claude ou une clé API.',
        link: { label: 'Documentation de Claude Code', href: 'https://code.claude.com/docs/fr/overview' },
      },
      {
        title: 'Installe Escouade',
        text: 'Télécharge l’installeur de la dernière version : le .exe pour Windows 10 ou 11 (avec Git for Windows), le .dmg pour macOS 11 ou plus récent (Mac Apple Silicon ou Intel).',
        link: { label: 'Dernière version', href: DOWNLOAD },
      },
      {
        title: 'Ouvre un projet',
        text: 'Choisis un dossier, crée un agent et écris ta première demande. Les suivants arrivent avec Ctrl+N (⌘N sur Mac).',
      },
    ],
  },

  faq: {
    eyebrow: 'FAQ',
    title: 'Questions fréquentes',
    items: [
      {
        q: 'C’est gratuit ?',
        a: 'Oui, et open source, sous licence MIT. Escouade utilise ton propre Claude Code : ton abonnement Claude ou ta clé API, sans intermédiaire.',
      },
      {
        q: 'Où vont mes données ?',
        a: 'Nulle part : projets, conversations et statistiques restent sur ta machine, dans ~/.escouade/. Le réseau ne sert qu’à Claude Code lui-même, à la lecture de tes quotas, aux mises à jour de l’app et, si tu les connectes, à Jira, Trello ou GitHub : un ticket importé y reçoit son statut et des commentaires (ses critères, ce qui a été fait).',
      },
      {
        q: 'Et mes jetons Jira, Trello ou GitHub ?',
        a: 'Ils restent sur ta machine, dans le trousseau du système (Gestionnaire d’identification Windows, Trousseau macOS), et ne servent qu’aux appels de ces services. Pour GitHub, Escouade peut aussi reprendre celui de gh.',
      },
      {
        q: 'Ça marche sur Mac ou Linux ?',
        a: 'Sur Mac, oui : macOS 11 ou plus récent, Apple Silicon comme Intel. Linux n’est pas encore pris en charge.',
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
        a: 'Non. Escouade est un projet indépendant, non affilié à Anthropic. Claude et Claude Code sont des marques d’Anthropic.',
      },
    ],
  },

  footer: {
    github: 'GitHub',
    license: 'Licence MIT',
    note: 'Projet indépendant, non affilié à Anthropic. Claude et Claude Code sont des marques d’Anthropic.',
  },

  notFound: { title: 'Page introuvable', text: 'Cette adresse ne mène nulle part.', home: 'Retour à l’accueil' },
  failure: { title: 'Une erreur est survenue', home: 'Retour à l’accueil' },

  demo: {
    status: { ready: 'Prêt', running: 'En cours', question: 'Question', done: 'Terminé', totest: 'À tester' },
    agents: 'Agents',
    loop: 'boucle {loop}/{max}',
    criteria: { one: '{count} critère atteint sur {total}', other: '{count} critères atteints sur {total}' },
    tokens: 'tok',
    running: '{count} en cours',
    waiting: '{count} en attente',
    done: { one: '{count} terminé', other: '{count} terminés' },
    today: 'Aujourd’hui ≈ {amount}',
    pause: 'Mettre la démonstration en pause',
    caption: 'Démonstration animée · données fictives',
    description:
      ': cinq agents Claude Code du projet demo-api travaillent en parallèle ; l’un pose une question à laquelle tu peux répondre, un agent de ticket boucle sur ses critères jusqu’à « À tester ».',
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
};

// Everything the site says.

export const REPO = 'https://github.com/guillaume-gagnaire/escouade';
export const DOWNLOAD = `${REPO}/releases/latest`;
/** Public address of the site, for links shared on social networks. */
export const SITE = 'https://guillaume-gagnaire.github.io/escouade/';

export interface Feature {
  id: string;
  title: string;
  text: string;
  points: string[];
  /** In public/. */
  image: string;
  alt: string;
}

export const FEATURES: Feature[] = [
  {
    id: 'agents',
    title: 'Projets en onglets, agents en parallèle',
    text: 'Chaque projet a son onglet et sa couleur, qui teinte toute l’interface. Dans chacun, autant d’agents Claude Code que tu veux, chacun avec sa conversation, son modèle et son effort ; Haiku leur trouve un nom dès ta première demande.',
    points: [
      'Statut en direct : en cours, question, terminé',
      'Tokens, coût et fichiers touchés par agent',
      'Un worktree git par agent, si tu veux',
    ],
    image: 'images/agents.jpg',
    alt: 'La fenêtre d’Escouade : les projets en onglets, la liste des agents et la conversation de l’un d’eux',
  },
  {
    id: 'chat',
    title: 'Un vrai chat, pas un terminal',
    text: 'Markdown, code coloré, appels d’outils compacts que tu déplies pour voir un diff ou la sortie d’une commande. Joins une image, un PDF ou un fichier, cite un @fichier, lance une /commande, et écris pendant que Claude travaille : il en tient compte à l’étape suivante. Ctrl+K retrouve un message dans les conversations de tous tes agents.',
    points: [
      'Les questions et autorisations de Claude en cartes, auxquelles tu réponds aussi au clavier',
      'Modèle (avec sa version), effort et mode modifiables à tout moment',
      'Une carte résume chaque tâche : durée, coût, fichiers modifiés',
    ],
    image: 'images/chat.jpg',
    alt: 'Une conversation : réponse de Claude en markdown, appel d’outil déplié sur son diff',
  },
  {
    id: 'notifications',
    title: 'Tu sais quand on t’attend',
    text: 'Quand un agent pose une question ou termine, Escouade le signale : onglet et carte qui clignotent, carillon, notification système qui dit ce qui est demandé, barre des tâches qui clignote (ou icône du Dock qui rebondit). Fermer la fenêtre ne coupe rien : chaque agent reprend sa session au redémarrage.',
    points: [
      'Ctrl+J saute au prochain agent qui attend',
      'La vue d’ensemble montre tous les agents, ceux qui attendent en tête',
      'Tu choisis ce qui te prévient : questions, fins, erreurs, tickets',
    ],
    image: 'images/notifications.jpg',
    alt: 'Un agent en attente de réponse, sa question et sa carte dans la barre latérale',
  },
  {
    id: 'git',
    title: 'Git sous les yeux',
    text: 'Fichiers non commités par agent ou pour tout le projet, diff unifié ou côte à côte, git graph du dépôt avec la branche de l’agent en avant. La disposition moitié / moitié montre la conversation et les fichiers ensemble, mis à jour pendant que l’agent écrit.',
    points: [
      'Commit rédigé par l’agent, ou direct avec un message proposé que tu relis',
      'Merge ou squash d’un worktree dans sa branche de base, puis nettoyage',
      'Pull, push et fetch depuis la barre de statut',
    ],
    image: 'images/git.jpg',
    alt: 'La disposition moitié / moitié : la conversation, les fichiers non commités et leur diff côte à côte',
  },
  {
    id: 'editeur',
    title: 'Un éditeur, sans quitter l’app',
    text: 'Retouche un fichier du projet ou du worktree d’un agent sans changer de fenêtre : arborescence, onglets, coloration, recherche, et les lignes modifiées depuis le dernier commit marquées dans la marge. Ctrl+clic va à une définition sans serveur de langage, Ctrl+P ouvre un fichier par son nom, Ctrl+Maj+F cherche dans tous.',
    points: [
      'Voir les changements dans le texte, et annuler un bloc',
      'Compare avec ce que l’agent vient d’écrire avant de choisir',
      'Renommer, supprimer vers la corbeille, ouvrir un terminal ici',
    ],
    image: 'images/editor.jpg',
    alt: 'L’éditeur intégré : l’arborescence du worktree, des onglets et le code avec ses lignes modifiées',
  },
  {
    id: 'tableau',
    title: 'Des tickets, que des agents prennent seuls',
    text: 'Le Kanban de chaque projet : à faire, en cours, à tester, terminé. Écris un ticket avec ses critères d’acceptation ; en pilote auto, des agents le prennent, chacun dans son worktree avec ses propres ports, et bouclent jusqu’à ce que chaque critère soit atteint.',
    points: [
      'Pilote auto, de 1 à 6 agents en parallèle, en pause près de la limite de tes quotas',
      'Boucle, critères, avancement et coût en direct sur chaque carte',
      'Valider : tests, commit généré, merge, pull request ou push ; ou renvoyer à l’agent',
    ],
    image: 'images/board.jpg',
    alt: 'Le Kanban d’un projet : deux tickets en cours avec leur boucle et leurs critères, et les tickets terminés',
  },
  {
    id: 'test',
    title: 'Teste chaque ticket en un clic',
    text: 'Quand un ticket passe « À tester », « ▶ Tester » te montre la recette de l’agent, puis prépare son worktree, lance ses serveurs sur ses ports réservés, attend qu’ils répondent et ouvre ton navigateur directement sur la fonctionnalité développée.',
    points: [
      'La recette vient de l’agent : tu la lis avant qu’elle tourne',
      'Les logs restent dans la section Lancement, sous son nom',
      '« Préparer le lancement » pour tout agent à worktree',
    ],
    image: 'images/test.jpg',
    alt: 'La fenêtre « Tester DEM-6 » : préparation, serveurs prêts et adresse ouverte dans le navigateur',
  },
  {
    id: 'integrations',
    title: 'Tes tickets Jira, Trello et GitHub, dans le Kanban',
    text: 'Connecte Jira, Trello ou GitHub Issues et lie une source à ton projet. « Importer » cherche et filtre ses tickets : coche-les, ils arrivent dans le Kanban avec leurs critères d’acceptation. Escouade tient ensuite leur statut à jour et commente le ticket d’origine quand il est prêt à tester, puis terminé.',
    points: [
      'Critères d’acceptation repris de la description ou de la checklist',
      'Statut et commentaires synchronisés, colonne par colonne, et retentés en cas d’échec',
      'Import automatique, si tu l’actives, des tickets étiquetés claude-ready',
    ],
    image: 'images/integrations.jpg',
    alt: 'La fenêtre « Importer des tickets » : trois tickets Jira cochés, avec leurs critères d’acceptation détectés',
  },
  {
    id: 'lancement',
    title: 'Lance ton projet d’un clic',
    text: 'Configure les commandes qui lancent ton projet (front, API, worker…), chacune avec son shell et son dossier, ou laisse Claude lire le projet et te les proposer. Chaque commande tourne dans son terminal, avec son statut en direct.',
    points: [
      'Tout lancer, tout arrêter, relancer',
      'Un plantage se voit tout de suite, avec son code',
      'Arrêter coupe aussi ce que la commande a lancé',
    ],
    image: 'images/launch.jpg',
    alt: 'Trois commandes de lancement, dont une plantée, et le journal du serveur de développement',
  },
  {
    id: 'stats',
    title: 'Tokens, coût, quotas : en direct',
    text: 'La barre de statut suit ton quota de session de 5 h, ton quota hebdomadaire, leur réinitialisation et le coût du jour, qui monte pendant que Claude travaille. Un agent arrêté par sa limite d’usage reprend tout seul quand le quota revient.',
    points: [
      'Tokens d’entrée, de cache et de sortie, par jour, semaine ou mois',
      'Coût par projet, par modèle, par agent et par ticket',
      'Mémoire et processeur pris par Claude',
    ],
    image: 'images/stats.jpg',
    alt: 'La page des statistiques : tokens par jour, coût par projet et par modèle',
  },
  {
    id: 'remote',
    title: 'Et depuis ton téléphone',
    text: 'Active le remote control sur un agent : sa session s’ouvre sur claude.ai et dans l’app Claude sur mobile. Ce que tu y envoies s’affiche aussi dans Escouade.',
    points: [
      'Au cas par cas, d’un clic droit sur l’agent',
      'Même session après un redémarrage',
      'L’agent reste joignable tant que Escouade tourne',
    ],
    image: 'images/remote.jpg',
    alt: 'Un téléphone sur claude.ai et Escouade qui affichent la même conversation',
  },
];

export interface Card {
  title: string;
  text: string;
}

export const CARDS: Card[] = [
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
];

export interface Step {
  title: string;
  text: string;
  link?: { label: string; href: string };
}

export const STEPS: Step[] = [
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
];

export interface Question {
  q: string;
  a: string;
}

export const FAQ: Question[] = [
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
    a: 'Tu choisis combien d’agents travaillent en parallèle (de 1 à 6), combien de boucles chaque ticket a au plus, et à quel pourcentage de tes quotas le pilote auto se met en pause. Rien ne démarre tant qu’un agent attend la fin de sa limite d’usage, et un ticket arrivé à sa dernière boucle passe « À tester » avec « Objectif partiel ».',
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
];

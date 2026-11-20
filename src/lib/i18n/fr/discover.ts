import type { Tree } from '../types';

// La découverte des fonctionnalités (ESC-506, docs/ONBOARDING.md § 7, direction C « La ligne ») : la carte « À découvrir »
// (Aide › À découvrir), ses stations rangées dans le cycle, le détail de celle choisie, les bulles au bon moment et la
// bande « Nouveau dans la 2.3 ».
export default {
  title: 'À découvrir',
  /** `{done}` sur `{total}`: those on this device. */
  progress: '{done} sur {total} · chaque station t’amène au vrai geste',
  trunk: 'Installé · harness, compte, projet, premier agent',
  hideAll: 'Tout masquer',
  showAgain: 'Réafficher',
  /** « Tout masquer » done: the map stays here. */
  hiddenNote: 'Masqué : plus de bulle ni de point sur Aide. La carte reste ici, dans Aide › À découvrir.',
  /** The band's « Voir »: the map on what is new. `{release}`: « 2.3 ». */
  onlyNew: 'Nouveau dans la {release}',
  seeAll: 'Tout voir',
  close: 'Retour au projet',
  /** The name of the map, for a screen reader. */
  mapLabel: 'Les découvertes, rangées dans le cycle',
  lanes: {
    plan: 'Planifier',
    code: 'Coder',
    review: 'Relire',
    ship: 'Livrer',
    drive: 'Piloter',
  },
  legend: {
    done: 'fait',
    todo: 'à essayer',
    dismissed: 'écarté',
    absent: 'pas sur ce poste',
  },
  /** Under a station. */
  state: {
    done: 'fait',
    dismissed: 'écarté',
    /** `{name}`: what it needs first. */
    after: 'après « {name} »',
  },
  absent: {
    noMicrophone: 'aucun micro sur ce poste',
    noTeamServer: 'pas de serveur d’équipe',
    policy: 'non permis sur ce poste',
    noMicrophoneLong: 'Aucun micro n’est branché sur cette machine : la voix attend qu’il y en ait un.',
    noTeamServerLong: 'Aucun serveur d’équipe n’est connu ici : une invitation de ton équipe (un lien escouade://) en apporte un.',
  },
  /** The detail of the station chosen. */
  detail: {
    try: 'Essayer',
    again: 'Y retourner',
    dismiss: 'Écarter',
    restore: 'Remettre',
    /** `{name}`: what it needs first. */
    first: 'D’abord : {name}',
    done: 'Fait : coché par ce que tu as fait dans l’app.',
    dismissed: 'Écarté : il ne te sera plus proposé. « Remettre » le replace sur la carte.',
    /** Before what it says it costs. */
    before: 'Avant le geste',
    plugins: 'Ouvrir Plugins',
  },
  /** The bubbles. */
  tip: {
    ok: 'Compris',
    dismiss: 'Écarter',
    map: 'Tout ce qui reste à découvrir',
    label: 'À découvrir : {name}',
  },
  /** The band of a device set up before the release. */
  band: {
    text: 'Nouveau dans la {release} : les PR dans Escouade, l’éditeur VS Code, ton équipe.',
    see: 'Voir',
    close: 'Fermer pour cette version',
  },
  /** The rail's Aide. */
  help: {
    item: 'À découvrir',
    /** `{done}` sur `{total}`. */
    itemDetail: '{done} sur {total} : la voix, le Kanban, les PR…',
    itemHidden: 'Masqué : la carte reste ici',
    dot: 'Il reste des choses à découvrir',
  },
  /** Réglages › Général. */
  settings: {
    row: 'À découvrir',
    rowDesc: 'La carte de ce qu’Escouade sait faire, chaque chose menant au vrai geste. Ce qui est écarté y reste, avec « Remettre ».',
    button: 'Ouvrir la carte',
  },
  items: {
    ticket: {
      name: 'Écrire un ticket',
      line: 'Un ticket et ses critères d’acceptation : un agent boucle jusqu’à les atteindre, puis te rend son bilan.',
      meta: 'le Kanban',
      facts: 'Rien ne part avant « Lancer » : c’est toi qui démarres l’agent du ticket.',
    },
    autopilot: {
      name: 'Le pilote auto',
      line: 'Il lance seul les tickets prêts, un agent chacun, dans la limite que tu fixes ; il ne valide ni ne merge jamais sans toi.',
      meta: 'un ticket prêt',
      facts:
        'Chaque ticket qu’il lance consomme le quota de ton compte. Jamais sans toi : la validation, le merge, une permission que ton mode demande.',
    },
    docs: {
      name: 'La spec et les décisions',
      line: 'L’onglet Docs : la spec du projet et ses décisions, tenues à jour par les agents des tickets.',
      meta: 'onglet Docs',
      facts: 'Une décision proposée par un agent attend ton accord, à la validation de son ticket.',
    },
    sources: {
      name: 'Importer des tickets',
      line: 'Les tickets de Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards dans ton Kanban : un agent les prend et les fait avancer.',
      meta: 'un jeton, au trousseau',
      facts:
        'Un jeton de la source, gardé dans le trousseau du système. Escouade installe le plugin de la source depuis l’app, sans réseau ; la connexion, elle, demande le réseau.',
    },
    dictation: {
      name: 'Dicter un message',
      line: 'Maintiens le micro sous le message, parle, relâche : ton texte arrive dans le champ, prêt à relire. Essaie d’abord le micro dans Réglages › Voix.',
      meta: '148 Mo, local',
      facts:
        'Whisper « base », 148 Mo, téléchargé une fois, au premier clic sur le micro, s’il n’est pas déjà là. Transcrit sur ta machine : rien de ce que tu dis n’en sort. Ou un service en ligne, avec ta clé, dans Réglages › Voix.',
    },
    voice: {
      name: 'Parler à un agent',
      line: 'Une conversation à voix haute : tu parles, l’agent te répond à voix haute, tu l’interromps quand tu veux.',
      meta: 'micro et son',
      facts:
        'La voix locale se télécharge au premier essai, sa taille dite avant. Chaque échange est un message à l’agent : il consomme son quota.',
    },
    terminal: {
      name: 'Un terminal',
      line: 'Un terminal dans le dossier du projet ou le worktree d’un agent, à côté de ses conversations.',
      meta: '+ des terminaux',
      facts: 'Il tourne sur ta machine, dans ton shell.',
    },
    editor: {
      name: 'L’éditeur',
      line: 'Ouvrir un fichier du projet, voir ce qu’un agent y a changé.',
      meta: 'Ctrl P',
      facts: 'La vue VS Code peut télécharger Node à la demande : elle le dit avant.',
    },
    secondAgent: {
      name: 'Un 2ᵉ agent',
      line: 'Deux agents en parallèle, chacun dans son worktree, sur sa branche : ils ne se gênent pas.',
      meta: 'Ctrl N',
      facts: 'Chaque message d’un agent consomme le quota de son compte.',
    },
    launch: {
      name: 'Lancer le projet',
      line: 'Les commandes qui lancent ton projet (serveur de dev, watchers), proposées par un agent, relues avant de tourner.',
      meta: 'commandes relues',
      facts: '« Proposer des commandes » envoie un message à un agent : il consomme ton quota.',
    },
    forge: {
      name: 'Connecter GitHub',
      line: 'Les PR et la CI de tes tickets, dans Escouade : GitHub, GitLab, Bitbucket, Gitea ou Azure DevOps.',
      meta: 'un jeton, au trousseau',
      facts:
        'Un jeton de ton hébergeur, gardé dans le trousseau du système. Escouade installe le plugin de l’hébergeur depuis l’app, sans réseau ; la connexion, elle, demande le réseau.',
    },
    prView: {
      name: 'Une PR dans Escouade',
      line: 'Sa description, ses fichiers, sa CI et sa discussion, sans quitter l’app.',
      meta: 'onglet PR',
      facts: 'Elle est lue sur ton hébergeur, avec ton jeton : rien n’y est écrit sans ton clic.',
    },
    prReview: {
      name: 'Relecture par un agent',
      line: 'Un agent relit la PR et la commente ligne par ligne, comme un relecteur.',
      meta: 'une PR ouverte',
      facts: 'La relecture consomme le quota de ton compte.',
    },
    approve: {
      name: 'Valider un ticket',
      line: 'Le bilan de l’agent relu, « Valider » fait entrer son travail : merge, PR ou push, selon le réglage du Kanban.',
      meta: 'un ticket À tester',
      facts: 'Selon le réglage du Kanban, « Valider » merge, pousse ou ouvre une PR : relis le bilan avant.',
    },
    openPr: {
      name: 'Ouvrir une PR',
      line: 'La branche d’un agent poussée, sa PR ouverte sur ton hébergeur, sa CI suivie dans Escouade.',
      meta: 'le panneau PR et CI',
      facts: 'Pousser et ouvrir la PR passent par ton jeton : rien ne part sans ton clic.',
    },
    plugin: {
      name: 'Un plugin',
      line: 'Le store : d’autres harness, des hébergeurs, des services de voix, des thèmes.',
      meta: 'le store',
      facts: 'Le store demande le réseau ; chaque plugin dit ses permissions avant de s’installer.',
    },
    mcp: {
      name: 'Le serveur MCP',
      line: 'Piloter Escouade depuis un autre client : Claude Code dans un terminal crée des tickets, lit le Kanban, lance des agents.',
      meta: 'un autre client',
      facts: 'Il écoute sur cette machine seulement, derrière un jeton.',
    },
    account: {
      name: 'Un 2ᵉ compte',
      line: 'Un compte perso et un pro, ou un autre harness : quand l’un atteint sa limite, ses agents passent au suivant.',
      meta: 'la passation',
      facts: 'Un nouveau compte se connecte dans un terminal de l’app ; un autre harness installe son plugin, et le dit avant.',
    },
    remote: {
      name: 'Ton téléphone',
      line: 'Escouade à distance : répondre à tes agents depuis ton téléphone.',
      meta: 'Escouade à distance',
      facts: 'Elle écoute derrière un jeton, et se règle sur ce poste.',
    },
    team: {
      name: 'Ton équipe',
      line: 'Le Kanban, la spec et les décisions partagés avec ton équipe, sur votre serveur d’équipe.',
      meta: 'par une invitation',
      facts: 'Une invitation de ton équipe (un lien escouade://) t’y fait entrer ; tu vérifies son empreinte avant.',
    },
  },
} as const satisfies Tree;

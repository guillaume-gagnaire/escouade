import type { Tree } from '../types';

// The Claude accounts.
export default {
  /** The user's own account, as long as they have not renamed it (its name is saved in the language of its first save). */
  principal: 'Principal',

  // The « Comptes Claude » tab of the settings.
  tab: {
    list: 'Comptes Claude',
    order: 'Les nouveaux agents partent sur le premier compte actif sous le seuil de pause, dans cet ordre.',
    name: 'Nom',
    folder: 'Dossier',
    executable: 'Exécutable',
    executableDefault: 'Celui des réglages',
    active: 'Actif',
    lastActive: 'Il faut au moins un compte actif.',
    up: 'Monter {name}',
    down: 'Descendre {name}',
    signIn: 'Se connecter…',
    signInAgain: 'Se reconnecter…',
    remove: 'Supprimer',
    add: 'Ajouter un compte…',
    stillUsed: { one: 'Le compte sert encore à {count} agent.', other: 'Le compte sert encore à {count} agents.' },
    state: {
      checking: 'Vérification…',
      connected: 'Connecté · {email}',
      connectedNoEmail: 'Connecté',
      notConnected: 'Pas connecté',
      /** Principal without a claude.ai sign-in: most likely run with an API key. */
      principalNotConnected: 'Pas connecté par un compte claude.ai (clé d’API ?)',
      inactive: 'Inactif',
    },
    removeConfirm: {
      title: 'Supprimer le compte « {name} » ?',
      body: 'Les nouveaux agents ne partiront plus dessus. Son dossier reste sur le disque, avec sa connexion et ses conversations : {path}',
      confirm: 'Supprimer',
    },
  },

  // « Ajouter un compte… » and « Se connecter… ».
  modal: {
    addTitle: 'Ajouter un compte Claude',
    signInTitle: 'Se connecter au compte « {name} »',
    name: 'Nom',
    namePlaceholder: 'Pro, Équipe…',
    share: 'Partager avec {name}',
    files: 'Fichiers',
    folders: 'Dossiers',
    filesCopied: 'copiés (un lien de fichier demande des droits d’administrateur)',
    mode: 'Partage',
    link: 'Lier (un changement vaut pour les deux comptes)',
    copy: 'Copier',
    nothingToShare: '{name} n’a ni réglages, ni CLAUDE.md, ni skills, agents, commandes, plugins, hooks ou styles de sortie à partager.',
    neverShared: 'La connexion (.credentials.json) et .claude.json ne sont jamais partagés : chaque compte a les siens.',
    create: 'Créer et se connecter',
    signInHint: 'Connecte-toi dans le terminal ci-dessous (commande /login si Claude Code ne la propose pas).',
    connected: 'Connecté au compte {email}',
    connectedNoEmail: 'Connecté',
    ended: 'Claude Code s’est arrêté avant la connexion.',
    restart: 'Relancer',
    done: 'Terminé',
  },

  // The account chip of the Composer, before the agent's first message.
  composer: {
    caption: 'Compte',
    auto: 'Automatique ({name})',
    autoDetail: 'Celui du projet, sinon le premier compte sous le seuil de pause.',
    inactive: '{name} (inactif)',
    locked: 'Le compte d’un agent ne change plus après son premier message (sa conversation est rangée dans ce compte).',
  },

  // « Compte préféré » in a project's settings.
  project: {
    label: 'Compte préféré',
    desc: 'Le compte sur lequel partent les nouveaux agents et les tickets de ce projet.',
    auto: 'Automatique',
    autoTitle: 'Le premier compte actif sous le seuil de pause, dans l’ordre des comptes',
    inactive: '{name} (inactif)',
  },

  // Under the autopilot's pause: the accounts that are past « Pause au-delà du quota ».
  pause: {
    over: { one: 'Le compte {accounts} a passé le seuil.', other: 'Les comptes {accounts} ont passé le seuil.' },
  },
} as const satisfies Tree;

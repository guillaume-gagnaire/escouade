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

  // The current account's quota in the status bar (the percentage is in a tooltip), and the panel of every account's.
  quota: {
    fiveHour: { label: '5h', name: 'Quota sur 5 heures' },
    sevenDay: { label: '7j', name: 'Quota sur 7 jours' },
    /** `{countdown}`: the time left, as `fCountdown` writes it. */
    reset: 'reset {countdown}',
    /** The value of a bar: `{pct}`: « 42 % »; `{date}`: « 10/10 »; `{time}`: « 18:00 ». */
    tip: '{pct} · remise à zéro le {date} à {time}',
    unavailable: 'Quota indisponible',
    /** One window as the bar group says it to the keyboard and to screen readers; `{name}`: « Quota sur 5 heures », `{tip}`: the tooltip's text. */
    line: '{name} : {tip}',
  },

  // The panel the quota group opens, above the status bar, when there are several accounts.
  panel: {
    /** The button that is the quota group; `{name}`: the current account. */
    open: 'Quotas par compte (compte en cours : {name})',
    title: 'Quotas des comptes Claude',
    current: 'en cours',
    over: 'au-delà du seuil de pause',
    /** What the account's turns cost today; `{amount}`: « 1,20 $ » (« ≈ 1,20 $ » while a turn is running). */
    today: "Aujourd'hui : {amount}",
  },

  // The card of a turn stopped by the usage limit: go on on another account (the session goes along).
  resume: {
    /** `{account}`: the account the agent goes on on. */
    on: 'Reprendre sur {account}',
    /** The button that opens the menu of the other accounts. */
    another: 'Reprendre sur un autre compte',
    /** The turn that followed the move failed; `{account}`: the account it went to, `{error}`: what Claude Code said. */
    failed: 'La reprise sur {account} a échoué : {error}.',
    /** `{account}`: the account the agent came from. */
    back: 'Revenir sur {account}',
  },

  // The setting of the Claude Code tab.
  switchOnLimit: {
    label: 'Reprendre sur un autre compte un agent de ticket arrêté par la limite',
    desc: 'Sinon il attend la remise à zéro de son compte',
  },
} as const satisfies Tree;

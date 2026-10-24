import type { Tree } from '../types';

// The new project's assistant: its steps, the copy of another project's settings, the checks.
export default {
  title: 'Nouveau projet',
  /** Opened again on an existing project, from Réglages › Projets. */
  titleEdit: 'Configurer {name}',
  stepOf: 'Étape {n} sur {count}',
  stepsLabel: 'Étapes',
  yes: 'oui',
  no: 'non',
  currentBranch: 'branche courante',
  draftPr: 'PR brouillon',
  actions: {
    merge: 'merge',
    pr: 'PR',
    push: 'push de la branche',
    keep: 'branche gardée',
  },
  /** The steps, as the rail names them. */
  steps: {
    start: 'Départ',
    copied: 'Réglages copiés',
    worktrees: 'Worktrees',
    launch: 'Lancement',
    kanban: 'Kanban',
    account: 'Compte et commit',
    integrations: 'Intégrations',
    summary: 'Récapitulatif',
  },
  skippedTag: 'passé',
  optionalTag: 'facultatif',
  /** What each step is for, under its title. */
  leads: {
    copied: 'Décoche ce que tu ne veux pas reprendre. Ce qui dépend du dossier est vérifié dans {name}.',
    worktrees: 'Où travaillent les agents et ce qui prépare chaque worktree.',
    launch: 'Les commandes qui lancent le projet, chacune dans son terminal.',
    kanban: 'Ce que fait la validation d’un ticket, et comment ses agents travaillent.',
    account: 'Le harness et le compte des agents de ce projet, qui écrit les commits, et s’il est privé.',
    integrations: 'Les tickets peuvent venir de Jira, Trello, GitHub ou Linear. Rien n’est obligatoire.',
    summary: 'Ce qui empêcherait la méthode de travail de marcher, puis ce qui sera réglé.',
  },
  start: {
    label: 'Point de départ',
    copy: 'Copier la configuration d’un projet',
    copyDesc: 'Worktrees, lancement, Git, Kanban et compte, vérifiés dans ce dépôt. Les intégrations et les tickets ne sont pas copiés.',
    copyFrom: 'Projet copié',
    noOther: 'Aucun autre projet ouvert à copier.',
    /** The folder's .escouade.yml (ESC-146), above the recommended setup. */
    file: 'Réglages de .escouade.yml',
    fileDesc: {
      one: 'Réglages lus dans .escouade.yml, le fichier partagé du dépôt : {count} réglage.',
      other: 'Réglages lus dans .escouade.yml, le fichier partagé du dépôt : {count} réglages.',
    },
    filePending: 'Ses commandes attendront ton accord une fois le projet créé.',
    privateLeftOut: {
      one: 'Un projet privé n’est pas proposé : rien d’un projet privé ne passe à un autre.',
      other: '{count} projets privés ne sont pas proposés : rien d’un projet privé ne passe à un autre.',
    },
    recommended: 'Configuration recommandée',
    recommendedDesc:
      'Un worktree par agent, merge squash dans la branche courante, worktree supprimé une fois validé, commits conventionnels, l’agent résout les conflits.',
    blank: 'Partir de zéro',
    blankDesc: 'Les valeurs par défaut d’Escouade, à régler dans les étapes suivantes.',
    current: 'Garder la configuration actuelle',
    currentDesc: 'Les réglages du projet tels qu’ils sont, à revoir étape par étape.',
  },
  buttons: {
    next: 'Suivant',
    skip: 'Passer',
    back: 'Retour',
    withoutWizard: 'Créer sans l’assistant',
    create: 'Créer le projet',
    creating: 'Création…',
    save: 'Enregistrer',
    saving: 'Enregistrement…',
    fill: '✦ Remplir automatiquement',
    filling: 'Claude lit le projet…',
    addCommand: '+ Ajouter une commande',
  },
  /** Said once Claude's commands are in the step. */
  filled: {
    one: '{count} commande proposée par Claude : relis-la avant de créer.',
    other: '{count} commandes proposées par Claude : relis-les avant de créer.',
  },
  copy: {
    groups: {
      worktrees: 'Worktrees',
      launch: 'Lancement',
      kanban: 'Kanban',
      account: 'Git et compte',
    },
    count: { one: '{count} réglage', other: '{count} réglages' },
    notCopied: 'Pas copiés : les sources liées (Jira, Trello, GitHub, Linear) et les tickets.',
    prefixes: 'Préfixes des branches',
    unnamed: 'Sans nom',
    validation: 'Validation',
    parallel: 'En parallèle · pilote auto',
    /** `{autopilot}`: « oui » or « non ». */
    parallelValue: { one: '{count} agent · pilote auto {autopilot}', other: '{count} agents · pilote auto {autopilot}' },
    agents: 'Harness · modèle · effort · mode',
    dirRelative: 'relatif au projet',
    dirAbsolute: 'chemin complet d’un autre projet : non repris',
    dirFound: 'dossier « {dir} » présent',
    dirMissing: 'dossier « {dir} » introuvable ici',
    branchFound: 'branche présente',
    branchReplaced: 'branche introuvable : {branch} à la place',
    branchReplacedNone: 'branche introuvable : la branche courante à la place',
    accountActive: 'compte actif',
    accountGone: 'compte inactif, réservé ou supprimé : Automatique',
  },
  worktrees: {
    note: 'Les tickets travaillent toujours dans leur worktree ; ce réglage vaut pour les agents ouverts à la main.',
  },
  launch: {
    none: 'Aucune commande : ajoute-en une, ou laisse Claude lire le projet.',
  },
  account: {
    remoteAll: 'Déjà allumé pour tous les agents (Réglages › Harness).',
    /** ESC-128: its harness has no account yet. */
    noAccount: 'Aucun compte {harness} : les agents et les tickets de ce projet ne démarreront qu’une fois un compte ajouté.',
    /** A private project's accounts (ESC-163): reserved to it alone once it is made. */
    reserved: 'Comptes réservés à ce projet',
    reservedDesc:
      'Cochés, ils ne servent plus que lui : les autres projets ne partent plus dessus, et sessions, mémoire et historique de {harness} n’y sont partagés avec aucun autre projet.',
    reservedAlready: 'déjà réservé',
    reservedNone: 'Aucun compte {harness} libre : ajoute un compte réservé dans Réglages › Projets, une fois le projet créé.',
  },
  integrations: {
    none: 'Aucun compte Jira, Trello, GitHub ou Linear n’est connecté : connecte-en un dans Réglages › Intégrations, après la création.',
    source: 'Source {service}',
    noSource: 'Aucune',
    loading: 'Chargement…',
  },
  summary: {
    firstAgent: 'Créer un premier agent',
    firstAgentDesc: 'Ouvre directement une conversation dans ce projet.',
    rows: {
      worktrees: 'Worktrees',
      setup: 'Préparation',
      launch: 'Lancement',
      validation: 'Validation',
      conflicts: 'Conflits',
      agents: 'Agents des tickets',
      harness: 'Harness',
      private: 'Projet privé',
      commit: 'Commit',
      integrations: 'Intégrations',
    },
    perAgent: 'un par agent',
    shared: 'dossier du projet',
    commands: { one: '{count} commande', other: '{count} commandes' },
    agentsValue: { one: '{count} en parallèle', other: '{count} en parallèle' },
    noneValue: 'aucune',
    autopilot: 'pilote auto',
    /** « Projet privé »: its accounts reserved, and whether it goes on shared ones at the limit. */
    privateValue: { one: 'oui · {count} compte réservé', other: 'oui · {count} comptes réservés' },
    privateNoAccount: 'oui · aucun compte réservé',
    sharedOnLimit: 'comptes partagés à la limite',
  },
  /** The checks: what would keep the way of working from working, and its correction. */
  checks: {
    noCommit: {
      title: 'Le dépôt n’a aucun commit',
      desc: 'Aucun ticket ne démarre : un worktree part d’un commit. Fais un premier commit (vide : aucun fichier n’est ajouté).',
      fix: 'Faire un premier commit',
    },
    noTarget: {
      title: 'La branche cible {branch} est introuvable',
      desc: 'La validation n’aurait nulle part où merger. Prends la branche courante, ou choisis-en une dans l’étape Kanban.',
      fix: 'Prendre la branche courante',
    },
    tracked: {
      title: 'Les worktrees iraient dans un dossier suivi par git',
      desc: 'Ce dossier contient des fichiers du dépôt : aucun worktree ne peut y être créé. Reviens au dossier par défaut, .claude/worktrees.',
      fix: 'Utiliser .claude/worktrees',
    },
    noAccount: {
      title: 'Aucun compte ne sert ce projet',
      desc: 'Rien ne démarre : active un compte, ou retire sa réservation à d’autres projets, dans Réglages › Comptes.',
    },
    noClaude: {
      title: 'Aucun harness trouvé',
      desc: 'Installe un harness (Claude Code : npm install -g @anthropic-ai/claude-code), ou indique son chemin dans Réglages › Harness.',
    },
    ok: 'Harness trouvé · un compte sert le projet · dépôt prêt',
  },
  /** Réglages › Projets: the assistant opened again on the project. */
  reopen: {
    label: 'Assistant de configuration',
    desc: 'Revoir les réglages du projet étape par étape, ou y copier ceux d’un autre projet.',
    button: 'Ouvrir l’assistant…',
  },
} as const satisfies Tree;

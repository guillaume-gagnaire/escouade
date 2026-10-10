import type { Tree } from '../types';

// Commits, diffs, the files panel and the git graph.
export default {
  // The side panel's uncommitted files.
  files: {
    scopeAgent: 'Cet agent',
    scopeProject: 'Tout le projet',
    hintWorktree: 'worktree {path} · isolé des autres agents',
    hintAgent: 'Fichiers modifiés par cet agent (d’après ses éditions)',
    hintProject: 'Tous les agents du projet · attribution par worktree',
    emptyAgent: 'Aucun fichier modifié par cet agent.',
    emptyProject: 'Aucune modification non commitée.',
    openNamed: 'Ouvrir {name} dans l’éditeur',
    showDiff: 'Voir le diff',
    commitAgent: 'Commit…',
    commitAll: 'Commit tout…',
    merge: 'Merger {branch} → {base}…',
    changesOf: 'Modifications de {name}',
    /** After the 500 rows listed (here and in the commit's window): `n` is the count as written. */
    more: { one: '… et {n} autre fichier', other: '… et {n} autres fichiers' },
    // Abandoning the changes of a file, from its menu.
    restore: 'Restaurer le fichier',
    deleteFile: 'Supprimer le fichier…',
    discardChanges: 'Abandonner les modifications…',
    deleteTitle: 'Supprimer « {name} » ?',
    discardTitle: 'Abandonner les modifications de « {name} » ?',
    deleteBody: '{path} n’a jamais été commité : il est supprimé du disque, sans retour possible.',
    discardBody: '{path} revient à son état du dernier commit : ses modifications non commitées sont perdues.',
    discardConfirm: 'Abandonner les modifications',
  },

  // The diff of a file, of the files listed, or of a commit.
  diff: {
    unified: 'Unifié',
    split: 'Côte à côte',
    closeTitle: 'Fermer ({key})',
    none: 'Aucune différence.',
    binary: 'Fichier binaire.',
    tooLarge: 'Diff trop volumineux pour être affiché.',
    /** Always long (more than 1500 lines): `n` is the count as written. */
    large: 'Diff volumineux ({n} lignes)',
    showMore: { one: 'Afficher {count} ligne de plus', other: 'Afficher {count} lignes de plus' },
    ofFile: 'Diff de {path}',
  },

  // A direct commit.
  commit: {
    title: 'Commit',
    scope: 'Modifications de {owner}',
    filesLabel: 'Fichiers du commit',
    noneForAgent: 'Aucun fichier à commiter pour cet agent.',
    noneForProject: 'Aucune modification dans le dossier du projet : le worktree d’un agent se commite depuis « Cet agent ».',
    /** `files`: their names, joined. */
    leftOut: {
      one: 'Jamais commité : {files} (copié dans les worktrees).',
      other: 'Jamais commités : {files} (copiés dans les worktrees).',
    },
    message: 'Message',
    proposing: 'Haiku rédige le message…',
    noProposal: 'Pas de proposition : {reason}.',
    created: 'Commit {hash} créé',
    regenerate: 'Régénérer',
    commit: 'Commiter',
    committing: 'Commit…',
    discardTitle: 'Abandonner le message ?',
    discardBody: 'Le message que tu as écrit pour ce commit sera perdu.',
    discardConfirm: 'Abandonner',
  },

  // The history of the repository.
  graph: {
    empty: 'Aucun commit.',
    tag: 'tag {name}',
    agentBranch: 'branche {branch} de l’agent {agent}',
  },

  // What an agent is asked and told about its work: commit, merge, remote control.
  agent: {
    /** Sent to the agent as its message. */
    commitPrompt:
      "Commite les modifications que tu as faites dans ce dépôt, avec un message clair au format Conventional Commits. N'inclus que les fichiers que tu as modifiés ; s'il y a plusieurs sujets distincts, fais plusieurs commits.",
    commitAllPrompt:
      'Commite toutes les modifications en cours du dépôt, regroupées en commits cohérents, avec des messages clairs au format Conventional Commits.',
    commitRequested: 'Demande de commit envoyée à {name}',
    mergeTitle: 'Merger {branch} dans {base} ?',
    mergeBody: "Les commits de l'agent « {name} » sont intégrés dans la branche « {base} » du projet.",
    mergeConfirm: 'Merger',
    squash: 'Squash (un seul commit)',
    merged: 'Merge effectué',
    // The project is not on the base branch: the merge starts by switching to it.
    switchTitle: 'Basculer sur « {base} » ?',
    switchBodyOnBranch: 'Le projet est sur la branche « {current} ». Escouade bascule sur « {base} » puis merge « {branch} ».',
    switchBodyDetached: 'Le projet n’est sur aucune branche (HEAD détachée). Escouade bascule sur « {base} » puis merge « {branch} ».',
    switchConfirm: 'Basculer et merger',
    remoteOn: '{name} est accessible depuis claude.ai et l’app Claude',
    linkCopied: 'Lien claude.ai copié',
  },
} as const satisfies Tree;

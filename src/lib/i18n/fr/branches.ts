import type { Tree } from '../types';

// The branches: the picker of the status bar, switching, creating, deleting and cleaning up.
export default {
  /** The picker the status bar's branch button opens. */
  picker: {
    title: 'Branches',
    searchLabel: 'Chercher une branche',
    searchPlaceholder: 'Chercher une branche…',
    local: 'Locales',
    remote: 'Distantes',
    /** The branch button when no branch is checked out. */
    detached: 'HEAD détachée',
    /** The tooltip of the branch button when the repository has no remote to sync with. */
    switchTitle: 'Changer de branche',
    empty: 'Aucune branche.',
    noMatch: 'Aucune branche ne correspond.',
    /** `{count}`: branches left out of the list to keep it short, which the search finds. */
    more: {
      one: '{count} autre branche : tape pour la chercher.',
      other: '{count} autres branches : tape pour les chercher.',
    },
    switching: 'Changement de branche…',
    /** The branch the project's folder is on, read to a screen reader. */
    current: 'Branche du dossier du projet',
    /** `{agent}`: the agent whose worktree holds the branch. */
    usedBy: 'utilisée par l’agent {agent}',
    /** `{branch}`, `{agent}`: also what the toast says when the backend refuses because of it. */
    usedByWhy: 'La branche « {branch} » est utilisée par l’agent {agent}, dans son worktree.',
    otherWorktree: 'dans un autre worktree',
    /** `{path}`: the folder of a worktree that belongs to no agent. */
    otherWorktreeWhy: 'La branche « {branch} » est prise par le worktree {path}.',
    /** `{branch}`: the local branch that already tracks a remote branch. */
    trackedBy: 'suivie par {branch}',
    trackedByWhy: 'Suivie par la branche locale « {branch} » : c’est elle qui est extraite.',
    gone: 'distante supprimée',
    /** `{upstream}`: the remote branch a local one tracks. */
    goneWhy: 'La branche suivie {upstream} n’existe plus sur le dépôt distant.',
    tracks: 'Suit {upstream} : {ahead} à pousser, {behind} à tirer',
    ahead: { one: '{count} commit à pousser', other: '{count} commits à pousser' },
    behind: { one: '{count} commit à tirer', other: '{count} commits à tirer' },
    newBranch: 'Nouvelle branche…',
    mergedBranches: 'Branches mergées…',
    deleteLocal: 'Supprimer la branche…',
    deleteRemote: 'Supprimer la branche distante…',
  },
  /** The sync with the remote, at the foot of the picker. */
  sync: {
    pull: 'Récupérer',
    push: 'Pousser',
    publish: 'Publier',
    fetch: 'Fetch',
    now: 'maintenant',
    /** What the branch button says while it runs. */
    pulling: 'Récupération…',
    pushing: 'Envoi…',
    fetching: 'Fetch…',
  },
  /** Switching the project's folder to another branch. */
  switch: {
    /** `{branch}`: the branch asked for. */
    title: 'Changer pour « {branch} » ?',
    dirty: 'Il reste des changements non commités dans le dossier du projet.',
    stashAndSwitch: 'Mettre de côté (stash) et changer',
    /** `{name}`: the message of the stash made. */
    stashed: 'Changements mis de côté : « {name} » (git stash).',
    /** `{agent}`: the agent whose turn is running in the project's folder. */
    agentWorking: 'L’agent {agent} travaille dans le dossier du projet : attends la fin de son tour.',
  },
  create: {
    title: 'Nouvelle branche',
    name: 'Nom',
    namePlaceholder: 'feat/ma-branche',
    from: 'À partir de',
    /** `{hash}`: the commit chosen in the graph. */
    fromCommit: 'Le commit {hash}',
    /** Detached HEAD: the commit the folder is on. */
    fromHead: 'Le commit courant',
    switchTo: 'et y passer',
    /** `{branch}`: the branch to create. */
    created: 'Branche « {branch} » créée.',
    createdHere: 'Branche « {branch} » créée : tu es dessus.',
    dirtyTitle: 'Créer « {branch} » et y passer ?',
  },
  delete: {
    /** `{branch}`: the branch to delete. */
    title: 'Supprimer la branche « {branch} » ?',
    body: 'La branche « {branch} » sera supprimée en local.',
    /** `{remote}`: its remote copy, as git names it (`origin/feat`). */
    alsoRemote: 'Supprimer aussi {remote}',
    done: 'Branche « {branch} » supprimée.',
    remoteTitle: 'Supprimer la branche distante « {branch} » ?',
    remoteBody: 'Elle sera supprimée du dépôt distant, pour tout le monde.',
    forceTitle: 'Supprimer « {branch} » quand même ?',
    forceCommits: {
      one: '{count} commit n’est dans aucune autre branche.',
      other: '{count} commits ne sont dans aucune autre branche.',
    },
    /** Not in the base, but every commit of it is in another branch. */
    forceNoCommits: 'Elle n’est pas mergée dans la base du projet, mais ses commits sont dans une autre branche.',
    forceConfirm: 'Supprimer quand même',
  },
  /** « Branches mergées » : the clean-up. */
  merged: {
    title: 'Branches mergées',
    intro: 'Ces branches locales sont déjà dans la base du projet.',
    list: 'Branches à supprimer',
    empty: 'Aucune branche mergée à nettoyer.',
    remove: { one: 'Supprimer {count} branche', other: 'Supprimer {count} branches' },
    removing: 'Suppression…',
    done: { one: '{count} branche supprimée.', other: '{count} branches supprimées.' },
    /** `{branch}`: the branch that stayed; `{reason}`: why, as the backend says. */
    failed: '« {branch} » n’a pas pu être supprimée : {reason}',
  },
} as const satisfies Tree;

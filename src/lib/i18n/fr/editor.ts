import type { Tree } from '../types';

// The code editor: its tabs, the file tree, the searches.
export default {
  /** What a refused action says in a toast. */
  toast: {
    saveFailed: 'Enregistrement impossible : {error}',
    createFailed: 'Création impossible : {error}',
    copyFailed: 'Copie impossible : {error}',
    renameFailed: 'Renommage impossible : {error}',
    deleteFailed: 'Suppression impossible : {error}',
    compareFailed: 'Comparaison impossible : {error}',
    navigateFailed: 'Navigation impossible : {error}',
    tabsKeepOldName: 'Les onglets restent à l’ancien nom : {error}',
    ignoredByGit: '{name} est ignoré par git : l’arborescence ne le montre pas.',
    noLongerIgnored: '{name} n’est plus ignoré par git : il peut être commité.',
    fileNotFound: 'Fichier introuvable : {path}',
    noDefinition: 'Aucune définition trouvée pour « {name} ».',
    newerOnDiskUnsaved: 'Le fichier a encore changé sur le disque : rien n’est enregistré, la comparaison montre sa nouvelle version.',
    nothingSaved: 'Rien n’a été enregistré : le fichier est revenu à la version que tu avais ouverte.',
    changedAgainKept: 'Le fichier a encore changé sur le disque : rien n’est enregistré, tes modifications sont toujours là.',
  },

  /** The bar above the editor. */
  head: {
    back: '← Conversation',
    unsaved: '● Non enregistré · {key}',
    saved: 'Enregistré',
  },

  /** The left column: the files and the search. */
  side: {
    viewLabel: 'Vue de la colonne',
    searchFiles: 'Rechercher dans les fichiers',
    searchFilesKey: 'Rechercher dans les fichiers ({key})',
    newFile: 'Nouveau fichier',
    newFolder: 'Nouveau dossier',
    widthLabel: 'Largeur de la colonne des fichiers',
  },

  /** The right-click menu of the tree. */
  menu: {
    newFile: 'Nouveau fichier…',
    newFolder: 'Nouveau dossier…',
    openTerminal: 'Ouvrir un terminal ici',
    rename: 'Renommer…',
    /** The Delete key, as written on the keyboard (shown in the menu). */
    deleteKey: 'Suppr',
    copyRelativePath: 'Copier le chemin relatif',
  },

  /** The line above the text: how the file differs from its reference version (`reference`: « main », « HEAD »…). */
  diff: {
    newFile: 'Nouveau fichier · absent de {reference}',
    changedLines: { one: '{n} ligne modifiée vs {reference}', other: '{n} lignes modifiées vs {reference}' },
    same: 'Identique à {reference}',
    show: 'Voir les changements',
  },

  /** The banner when the file changed on the disk or was deleted. */
  banner: {
    newerOnDisk: 'Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version.',
    comparing: 'Comparaison avec la version du disque.',
    changed: 'Ce fichier a changé sur le disque.',
    reload: 'Recharger',
    compare: 'Comparer',
    keepMine: 'Garder ma version',
    deleted: 'Ce fichier a été supprimé.',
    recreate: 'Le recréer en enregistrant',
  },

  /** What is asked before a tab with unsaved changes closes. */
  save: {
    title: 'Enregistrer « {name} » ?',
    body: 'Ses modifications seront perdues si tu ne les enregistres pas.',
    dontSave: 'Ne pas enregistrer',
  },

  /** What is asked before a file or a folder of the tree goes to the trash. */
  remove: {
    title: 'Supprimer « {name} » ?',
    bodyFile: 'Il part dans la corbeille.',
    /** `n`: the files the folder holds. */
    bodyFolderFiles: {
      one: 'Le dossier et son fichier partent dans la corbeille.',
      other: 'Le dossier et ses {n} fichiers partent dans la corbeille.',
    },
    bodyFolderEmpty: 'Le dossier part dans la corbeille.',
  },

  /** What the pane says in place of the text. */
  empty: {
    selectFile: 'Sélectionne un fichier dans l’arborescence.',
    binary: 'Fichier binaire : pas d’aperçu.',
    tooLarge: 'Fichier trop volumineux pour l’éditeur ({size}).',
    missing: 'Ce fichier n’existe pas (ou plus).',
  },

  /** The status bar under the text (the encoding and the end of lines are written the same in every language). */
  status: {
    position: 'Ln {line}, Col {col}',
    tabs: 'Tabulations',
    spaces: 'Espaces : {size}',
  },

  tabs: {
    label: 'Fichiers ouverts',
    close: 'Fermer {name}',
    /** `folder` tells two tabs of the same name apart. */
    closeIn: 'Fermer {name} · {folder}',
    closeUnsaved: 'Fermer (non enregistré)',
  },

  /** The field naming a new file or folder, and renaming one, in the tree. */
  newField: {
    fileName: 'Nom du nouveau fichier',
    folderName: 'Nom du nouveau dossier',
    rename: 'Renommer « {name} »',
  },

  tree: {
    ignoredByGit: 'Ignoré par git',
  },

  /** The source of the editor: the project's branch, or the worktree of an agent. */
  source: {
    /** The name of the branch when git gives none. */
    noBranch: 'projet',
    label: 'Source : {name}',
    kindWorktree: 'worktree',
    kindBranch: 'branche',
    projectBranch: 'Branche du projet · {path}',
    delta: 'Δ {count}',
    clean: 'propre',
    worktree: 'worktree · {name}',
    branch: 'branche · {name}',
  },

  /** Counts of the tree and of the searches; `n` is `count` written with its thousands apart. */
  count: {
    files: { one: '{n} fichier', other: '{n} fichiers' },
    results: { one: '{n} résultat', other: '{n} résultats' },
    changes: { one: '{n} modif.', other: '{n} modif.' },
    listTruncated: 'liste tronquée',
  },

  /** The places a followed identifier may lead to. */
  targets: {
    title: 'Définitions de « {label} »',
  },

  /** Added to the question of a deletion that loses unsaved files (the sentence is added to a text). */
  lossNotice: {
    one: '{count} fichier non enregistré dans l’éditeur sera perdu.',
    other: '{count} fichiers non enregistrés dans l’éditeur seront perdus.',
  },

  buffers: {
    /** After « Comparaison impossible : ». */
    diskNotText: 'la version du disque n’est pas du texte',
  },

  rename: {
    openUnsaved: '« {name} » est ouvert avec des modifications non enregistrées.',
  },

  /** Why a name typed in the tree is refused. */
  name: {
    startsWithSlash: 'Un nom ne peut pas commencer par une barre oblique.',
    endsWithFile: 'Le nom doit finir par celui d’un fichier.',
    endsWithDir: 'Le nom doit finir par celui d’un dossier.',
    invalidFile: '« {name} » n’est pas un nom de fichier valide.',
    invalidDir: '« {name} » n’est pas un nom de dossier valide.',
    intoItself: 'Un dossier ne peut pas aller dans lui-même.',
    isAFile: '« {name} » est un fichier.',
    exists: '« {name} » existe déjà à cet endroit.',
    existsIgnored: '« {name} » existe déjà à cet endroit (ignoré par git).',
  },

  /** The button of a block of the text compared with another version (and CodeMirror's « Revert this chunk »). */
  compare: {
    revertBlock: 'Annuler ce bloc',
    takeBlock: 'Prendre ce bloc',
  },

  language: {
    /** A file whose language is not known. */
    text: 'Texte',
  },

  /** « Rechercher dans les fichiers », in the left column. */
  search: {
    invalidRegex: 'Expression régulière invalide.',
    title: 'Recherche',
    caseSensitive: 'Respecter la casse',
    wholeWord: 'Mot entier',
    regex: 'Expression régulière',
    results: 'Résultats',
    searching: 'Recherche…',
    /** `results` and `files` are counts written out (« 3 résultats », « 2 fichiers »). */
    found: '{results} dans {files}',
    none: 'Aucun résultat.',
    timedOut: 'Recherche arrêtée après 10 s : résultats partiels.',
    truncated: 'Résultats limités aux {max} premiers.',
    fileRow: '{name}, {results}',
    fileRowIn: '{name}, {dir}, {results}',
    lineRow: 'Ligne {line} : {text}',
  },

  /** « Ouvrir un fichier » (Ctrl+P). */
  quickOpen: {
    title: 'Ouvrir un fichier',
    placeholder: 'Nom du fichier, ou nom:42 pour une ligne',
    loading: 'Chargement des fichiers…',
    noMatch: 'Aucun fichier ne correspond.',
    firstResults: 'Les {max} premiers résultats : précise ta recherche.',
    opensAtLine: 'ouvre à la ligne {line}',
    listTruncated: 'liste tronquée',
    ignored: 'ignoré par git',
    keyChoose: 'choisir',
    keyOpen: 'ouvrir',
    keyClose: 'fermer',
  },

  /** CodeMirror's own texts (search panel, merge view), by the English phrase it asks with. */
  cm: {
    accept: 'Accepter',
    reject: 'Rejeter',
    unchangedLines: '$ lignes inchangées',
    find: 'Rechercher',
    replace: 'Remplacer',
    next: 'suivant',
    previous: 'précédent',
    all: 'tout',
    matchCase: 'respecter la casse',
    byWord: 'mot entier',
    regexp: 'expression régulière',
    replaceVerb: 'remplacer',
    replaceAll: 'tout remplacer',
    close: 'fermer',
    currentMatch: 'occurrence courante',
    replacedMatches: '$ remplacements',
    replacedOnLine: 'remplacé à la ligne $',
    onLine: 'à la ligne',
    goToLine: 'Aller à la ligne',
    go: 'aller',
  },
} as const satisfies Tree;

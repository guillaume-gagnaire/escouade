import type { Tree } from '../types';

// The code editor: its tabs, the file tree, the searches.
export default {
  /** What a refused action says in a toast. */
  toast: {
    saveFailed: 'Enregistrement impossible : {error}',
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

  search: {
    invalidRegex: 'Expression régulière invalide.',
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

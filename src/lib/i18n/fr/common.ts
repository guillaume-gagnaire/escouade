import type { Tree } from '../types';

// The words every zone needs, the most repeated across the interface. Filled once: a zone adds what it lacks to
// its own file rather than here, so that two tasks never write the same file.
export default {
  // Actions
  cancel: 'Annuler',
  close: 'Fermer',
  save: 'Enregistrer',
  delete: 'Supprimer',
  remove: 'Retirer',
  rename: 'Renommer',
  edit: 'Modifier',
  add: 'Ajouter',
  create: 'Créer',
  open: 'Ouvrir',
  openInEditor: 'Ouvrir dans l’éditeur',
  copy: 'Copier',
  copyPath: 'Copier le chemin',
  retry: 'Réessayer',
  stop: 'Stopper',
  stopAll: 'Tout arrêter',
  browse: 'Parcourir',
  search: 'Rechercher',
  searchEllipsis: 'Rechercher…',
  refresh: 'Actualiser',
  show: 'Afficher',
  collapse: 'Réduire',
  collapseAll: 'Tout réduire',
  clear: 'Effacer',
  import: 'Importer',
  reply: 'Répondre',
  later: 'Plus tard',

  // States
  loading: 'Chargement…',
  inProgress: 'En cours',
  error: 'Erreur',

  // Things
  settings: 'Réglages',
  overview: 'Vue d’ensemble',
  editor: 'Éditeur',
  name: 'Nom',
  project: 'Projet',
  agent: 'Agent',
  agents: 'Agents',
  files: 'Fichiers',
  folder: 'Dossier',
  model: 'Modèle',
  cost: 'Coût',
  color: 'Couleur',
  command: 'Commande',

  // Counts
  count: {
    files: { one: '{count} fichier', other: '{count} fichiers' },
    lines: { one: '{count} ligne', other: '{count} lignes' },
    results: { one: '{count} résultat', other: '{count} résultats' },
    agents: { one: '{count} agent', other: '{count} agents' },
    projects: { one: '{count} projet', other: '{count} projets' },
    tickets: { one: '{count} ticket', other: '{count} tickets' },
    loops: { one: '{count} boucle', other: '{count} boucles' },
  },
} as const satisfies Tree;

import type { Tree } from '../types';

// « Recherche par le sens » (ESC-254) : le modèle des vecteurs, un pour tous les projets de cette machine, qui sert à la mémoire partagée et aux docs.
export default {
  allTitle: 'Tous les projets',
  label: 'Recherche par le sens',
  desc: 'En plus des mots, la mémoire partagée et les docs se cherchent par le sens, avec un petit modèle multilingue ({model}, licence {license}) qui tourne sur cette machine. Un seul téléchargement de {size}, pour tous les projets ; ensuite, rien ne sort de la machine.',
  download: 'Télécharger ({size})',
  resume: 'Reprendre ({pct} %)',
  cancel: 'Annuler',
  downloaded: 'Téléchargé',
  delete: 'Effacer',
  deleteTitle: 'Effacer le modèle ({size}) : la recherche se fait par les mots seuls jusqu’au prochain téléchargement.',
  progress: 'Téléchargement du modèle',
} as const satisfies Tree;

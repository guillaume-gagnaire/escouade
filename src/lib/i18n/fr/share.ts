import type { Tree } from '../types';

// The shared Kanban (ESC-190): a ticket shared with the team is a file `.escouade/tickets/<id>.yml` in the project's
// folder, written by Escouade and committed by the user. What the window says of it: the card, its menu, the form, the
// first launch's question (the user's name), the settings, the files panel.
export default {
  card: {
    mark: 'Partagé avec l’équipe · {file}',
    by: 'Ajouté par {name}',
    theirsTodo: 'Ajouté par {name} · le pilote auto ne le lance pas',
    takenBy: 'Pris par {name}',
    reviewAt: 'À tester chez {name}',
    prAt: 'En revue chez {name}',
    doneBy: 'Fait par {name}',
    alsoOn: '{name} y travaille aussi d’après son fichier. Ton agent continue ; ton prochain statut réécrit le fichier.',
    left: '{name} ne le partage plus. Ton agent l’avait pris : il reste chez toi.',
    gone: 'Son fichier n’est plus dans le dossier du projet : Escouade ne le réécrit plus.',
    shareAgain: 'Partager à nouveau',
    keepMine: 'Garder le mien',
    takeTheirs: 'Prendre le sien',
    openFile: 'Ouvrir le fichier',
    unreadable: 'Ticket partagé illisible',
    newer: 'Écrit par une version plus récente d’Escouade : lu, jamais réécrit.',
    someone: 'quelqu’un',
  },
  menu: {
    share: 'Partager avec l’équipe',
    unshare: 'Arrêter le partage',
    hide: 'Masquer chez moi',
    deleteForTeam: 'Supprimer pour l’équipe…',
    removeShared: 'Son fichier partagé est supprimé aussi, à commiter : il disparaîtra chez l’équipe à son pull.',
    removeTheirsTitle: 'Supprimer {key} pour l’équipe ?',
    removeTheirsBody: 'Son fichier est supprimé, à commiter. Il disparaîtra chez {name} et le reste de l’équipe à leur pull.',
  },
  board: {
    hidden: { one: '1 ticket masqué', other: '{count} tickets masqués' },
    show: 'Afficher',
    hide: 'Masquer',
  },
  toast: {
    shared: '{key} partagé : {file} est à commiter avec ton travail',
    unshared: '{key} n’est plus partagé : la suppression de son fichier est à commiter',
  },
  form: {
    label: 'Partager avec l’équipe',
    help: 'Escouade écrit {file} dans le dossier du projet et le tient à jour à chaque statut ; tu le commites avec ton travail.',
    helpNew:
      'Escouade écrit son fichier dans .escouade/tickets/ du dossier du projet et le tient à jour à chaque statut ; tu le commites avec ton travail.',
    big: 'Au-delà de 5 Mo, un fichier joint reste chez toi.',
    theirs: 'Ajouté par {name} : ce que tu changes va dans son fichier, pour toute l’équipe.',
  },
  name: {
    title: 'Ton nom pour l’équipe',
    body: 'Quand tu partages un ticket du Kanban, ou fais avancer celui d’un autre, ton nom est écrit dans son fichier : c’est ce que tes coéquipiers liront.',
    label: 'Ton nom',
    help: 'Repris de git config user.name. Modifiable dans Réglages › Général, et par projet.',
    helpEmpty: 'Modifiable dans Réglages › Général, et par projet.',
    preview: 'Chez eux :',
    later: 'Plus tard',
    save: 'Enregistrer',
  },
  settings: {
    group: 'Équipe',
    name: 'Ton nom',
    nameDesc:
      'Écrit dans les tickets que tu partages et dans ceux que tu fais avancer : tes coéquipiers le voient sur leur Kanban. Un projet peut avoir le sien (Réglages › Projets › Équipe).',
    id: 'Ton identifiant',
    idDesc:
      'Ce qui distingue tes tickets de ceux d’un homonyme : tiré de ton e-mail git, donc le même sur tes autres postes. L’e-mail lui-même n’est écrit nulle part.',
    projectGroup: 'Kanban partagé',
    projectName: 'Ton nom dans ce projet',
    projectNameDesc: 'Vide : celui de Réglages › Général ({name}). Gardé sur ce poste, jamais dans .escouade.yml.',
    projectNameGit: 'Vide : {name}, repris de git (git config user.name de ce dépôt). Gardé sur ce poste, jamais dans .escouade.yml.',
    fromGit: 'Repris de git',
    noName: 'pas encore de nom',
    files: 'Tickets partagés',
    filesNone:
      'Aucun pour l’instant : ils iront dans {dir}, relus quand ils changent (un pull, un changement de branche, une modification).',
    filesDesc: {
      one: '1 ticket partagé dans {dir} ({mine} de toi), relu quand il change (un pull, un changement de branche, une modification).',
      other:
        '{count} tickets partagés dans {dir} ({mine} de toi), relus quand ils changent (un pull, un changement de branche, une modification).',
    },
    shareNew: 'Partager chaque nouveau ticket',
    shareNewDesc: 'Un ticket créé ici, à la main, par un import ou par MCP, est partagé d’office ; on peut toujours arrêter.',
  },
  git: {
    group: 'Tickets partagés',
    note: 'Écrits par Escouade dans le dossier du projet ; elle ne les commite jamais. Commite-les avec ton travail, l’équipe les reçoit à son pull.',
  },
} as const satisfies Tree;

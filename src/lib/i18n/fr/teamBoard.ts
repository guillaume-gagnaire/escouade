import type { Tree } from '../types';

// The Kanban of a team's project (TS-6, docs/TEAM-SERVER.md §4): who holds a ticket, its conflicts with a teammate's
// writes, « Ne pas partager », the autopilot's tickets of the team, and the link of a project to its team's Kanban.
export default {
  card: {
    heldBy: 'Pris par {name} · depuis {since}',
    heldHere: 'Pris par ce poste · depuis {since}',
    someone: 'un collègue',
    unreachable: 'Injoignable depuis {count} min',
    interrupted: 'Interrompu chez {name} : son bail a expiré. Le pilote auto ne le reprend pas.',
    resume: 'Reprendre',
    reviewElsewhere: 'Sur le poste qui l’a pris : il se valide là-bas.',
    assigned: 'Assigné à {name}',
    assignedMe: 'Assigné à moi',
    localOnly: 'Sur ce poste seulement',
    conflict: '{name} a changé {field} en même temps',
    keepMineAgain: 'Reprendre le mien',
    keepTheirs: 'Garder le sien',
    unmerged: '{name} a changé la description en même temps, sur les mêmes lignes : elles ne se fusionnent pas.',
    keepMine: 'Garder le mien',
    takeTheirs: 'Prendre le sien',
    field: {
      title: 'le titre',
      description: 'la description',
      criteria: 'les critères',
      column: 'la colonne',
      rank: 'la place',
      key: 'la clé',
      maxLoops: 'les boucles',
      progress: 'l’avancement',
      partial: 'l’état',
      iteration: 'la boucle',
      assignee: 'l’assignation',
    },
  },
  menu: {
    assignMe: 'Me l’assigner',
    unassign: 'Ne plus me l’assigner',
    local: 'Ne pas partager',
    share: 'Partager avec l’équipe',
  },
  form: {
    local: 'Ne pas partager',
    localHelp: 'Sur ce poste seulement : rien de ce ticket ne part vers l’équipe.',
    team: 'Ticket de l’équipe',
    teamHelp: 'Ce ticket est sur le Kanban de l’équipe : ce que tu changes part chez tes collègues.',
  },
  settings: {
    takeTodo: 'Prendre aussi « À faire » de l’équipe',
    takeTodoDesc:
      'Le pilote auto prend les tickets de l’équipe qui te sont assignés ; coché, aussi ceux de « À faire ». Réglage de ce poste, jamais partagé.',
  },
  link: {
    title: 'Kanban de l’équipe',
    create: 'Partager son Kanban avec l’équipe',
    join: 'Rejoindre',
    idLabel: 'Identifiant du Kanban d’un collègue',
    idPlaceholder: '32 caractères',
    linked: 'Kanban de l’équipe · {id}',
    copy: 'Copier l’identifiant',
  },
} as const satisfies Tree;

import type { Tree } from '../types';

// L’écran des copies (ESC-487), dans Réglages › Avancé : les copies de chaque fichier du dossier de données, ce qu’elles
// contiennent, les comparer à aujourd’hui, en reprendre une ; les historiques des projets fermés à retrouver.
export default {
  group: 'Copies des données',
  note: 'Escouade copie ce qu’elle ne peut pas refaire : au démarrage, puis toutes les 10 minutes pour un fichier qui a changé. Elle garde les 6 dernières copies, la première de chaque heure sur 48 heures et la première de chaque jour sur 30 jours.',
  total: '{size} de copies, {max} au plus',
  folder: 'Dossier des copies',
  now: 'Faire une copie maintenant',
  nowDone: 'Copies faites des fichiers qui ont changé.',
  open: 'Ouvrir le dossier des copies',
  loading: 'Lecture des copies…',
  /** What each file is, as the screen names it. */
  what: {
    state: 'Projets, tickets et agents',
    settings: 'Réglages et comptes des harness',
    plugins: 'Plugins installés',
    integrations: 'Comptes de tickets et d’hébergeurs',
    ticketAccounts: 'Autres comptes de tickets',
    syncQueue: 'Synchronisations de tickets en attente',
    closed: 'Projet fermé « {name} »',
    closedUnnamed: 'Projet fermé',
    tracks: 'Suivi des workflows d’un projet',
  },
  closedGroup: {
    one: 'Historique de {count} projet fermé',
    other: 'Historiques de {count} projets fermés',
  },
  copies: { one: '{count} copie', other: '{count} copies' },
  none: 'Aucune copie pour l’instant.',
  missing: 'Absent aujourd’hui',
  unreadable: 'Illisible',
  today: 'Aujourd’hui : {what}',
  todayEmpty: 'Aujourd’hui : lisible',
  readable: 'Lisible',
  /** What a copy holds, in counts: one sentence per kind, joined by commas. */
  count: {
    projects: { one: '{count} projet', other: '{count} projets' },
    agents: { one: '{count} agent', other: '{count} agents' },
    tickets: { one: '{count} ticket', other: '{count} tickets' },
    accounts: { one: '{count} compte', other: '{count} comptes' },
    plugins: { one: '{count} plugin', other: '{count} plugins' },
    entries: { one: '{count} élément', other: '{count} éléments' },
  },
  compare: 'Comparer',
  restore: 'Reprendre',
  showCopies: 'Voir les copies',
  hideCopies: 'Masquer les copies',
  diff: {
    title: 'La copie du {at} comparée à aujourd’hui',
    note: 'Rien n’est écrit : c’est ce que la reprise de cette copie changerait.',
    kinds: {
      projects: 'Projets',
      tickets: 'Tickets',
      agents: 'Agents',
      accounts: 'Comptes',
      plugins: 'Plugins',
    },
    same: 'La copie et le fichier d’aujourd’hui ont les mêmes projets, tickets, agents, comptes et plugins.',
    more: { one: '… et {count} autre', other: '… et {count} autres' },
    unreadable: 'Cette copie ne se lit pas : elle ne peut pas être reprise.',
    close: 'Fermer la comparaison',
  },
  confirm: {
    title: 'Reprendre la copie du {at} ?',
    body: 'Le fichier {file} d’aujourd’hui ({today}) sera remplacé par cette copie ({copy}). Il est d’abord mis de côté sous un nom daté, à côté de lui : rien n’est perdu. La copie est relue avant d’être posée, au prochain démarrage d’Escouade.',
    bodyClosed:
      'L’historique de ce projet fermé sera remplacé par cette copie ({copy}). Celui d’aujourd’hui est d’abord mis de côté sous un nom daté : rien n’est perdu.',
    lost: 'Seraient perdus : {list}.',
    gained: 'Reviendraient : {list}.',
    confirm: 'Reprendre la copie',
  },
  pending: {
    title: 'La copie du {at} de {file} sera reprise au prochain démarrage',
    body: 'Escouade la relira puis la posera, le fichier d’aujourd’hui mis de côté. Redémarre maintenant, ou quitte Escouade quand tu veux.',
    restart: 'Redémarrer maintenant',
    cancel: 'Annuler la reprise',
    cancelled: 'Reprise annulée : le fichier d’aujourd’hui reste tel quel.',
  },
  atWork: {
    one: '{count} agent travaille : la reprise attend la fin de son travail, un redémarrage couperait son tour.',
    other: '{count} agents travaillent : la reprise attend la fin de leur travail, un redémarrage couperait leur tour.',
  },
  lost: {
    title: 'Projets fermés à retrouver',
    note: 'Leur historique n’est plus sur le disque, mais une copie en reste (gardée 30 jours).',
    detail: 'Copie du {at} · {what}',
    find: 'Retrouver',
    found: 'L’historique de « {name} » est retrouvé : il est parmi les projets fermés.',
    reopen: 'Rouvrir',
  },
  setAside: {
    title: 'Fichiers mis de côté',
    note: 'Gardés tels quels par une reprise, ou par un démarrage qui les a trouvés illisibles. Escouade ne les efface jamais : supprime-les à la main quand tu n’en as plus besoin.',
    unreadable: '{file}, illisible au démarrage',
    beforeRestore: '{file}, avant une reprise',
  },
  why: {
    title: 'Ce qui n’a pas de copies',
    body: 'Les conversations s’écrivent ligne à ligne et sont forcées sur le disque à chaque tour ; la mémoire partagée vit dans le dossier du projet et se partage par git ; les caches (plugins compilés, catalogue du store, lecture des workflows, texte des documents) se refont.',
  },
} as const satisfies Tree;

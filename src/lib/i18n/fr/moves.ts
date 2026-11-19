import type { Tree } from '../types';

// What changes machines in server mode (ESC-538, docs/SERVER-UX.md § 4, 7 (S12, S13) and 10): the folders of the
// server in « Nouveau projet », the files of the agents received on this computer, the ports relayed, the
// notifications named by their space, the sign-in of an account, the secrets kept in a file.
export default {
  browse: {
    sources: 'D’où vient le projet',
    clone: 'Cloner un dépôt',
    folder: 'Un dossier du serveur',
    path: 'Un chemin',
    lead: 'Les dossiers sont ceux de {name} ({host}), pas de ce poste : le projet et ses agents vivront sur le serveur.',
    up: 'Dossier parent',
    home: 'Dossier personnel',
    loading: 'Lecture du dossier…',
    empty: 'Aucun dossier ici.',
    more: '{count} dossiers de plus, non listés : tape le chemin.',
    project: 'Déjà un projet',
    projectOf: 'Déjà le projet « {name} »',
    repo: 'git · {branch}',
    repoNoBranch: 'git',
    plain: 'dossier',
    choose: 'Choisir ce dossier',
    chosen: 'Choisi : {path}',
    disk: 'Le disque du serveur : {free} libres sur {total}',
    list: 'Dossiers de {path}',
    open: 'Ouvrir {name}',
  },
  clone: {
    url: 'Adresse du dépôt',
    urlPlaceholder: 'git@github.com:acme/api.git ou https://…',
    into: 'Dans',
    folder: 'Dossier',
    go: 'Cloner sur {name}',
    busy: 'Clone en cours sur {name}…',
    done: 'Cloné dans {path}',
    note: 'Le serveur clone avec ses propres clés et identifiants git : rien ne passe par ce poste.',
  },
  files: {
    sentFrom: 'envoyé depuis {name}',
    copiedTo: 'copié sur ce poste dans {folder}',
    onServer: 'sur {name}',
    downloaded: 'Téléchargé',
    openCopy: 'Ouvrir la copie',
    revealCopy: 'Afficher sur ce poste',
    download: 'Télécharger sur ce poste',
    seeOnServer: 'Voir sur le serveur',
    runnable: 'un script ne s’ouvre pas d’Escouade : il se télécharge seulement',
    downloading: 'Téléchargement…',
    saved: '« {name} » copié sur ce poste dans {folder}',
    scriptSaved: '« {name} » est un programme ou un script : téléchargé dans {folder}, jamais ouvert d’ici.',
  },
  launch: {
    relayed: 'Le port {remote} de {name} est relayé sur ce poste : {address}',
    relayFailed: 'Le port {port} de {name} n’a pas pu être relayé sur ce poste : {why}',
  },
  folder: {
    onServer: 'Ce dossier est sur {name} : il s’ouvre dans le panneau Fichiers, pas dans l’Explorateur de ce poste.',
    noReveal: 'Ce dossier est sur {name} : rien à ouvrir sur ce poste.',
  },
  notify: {
    /** The head of a notification of a server, before its own title. */
    title: '{name} · {title}',
  },
  reconnected: { one: 'Reconnecté à {name} · {count} événement rattrapé', other: 'Reconnecté à {name} · {count} événements rattrapés' },
  reconnectedNone: 'Reconnecté à {name} · rien de neuf',
  login: {
    hint: 'Connecte-toi dans la page ouverte sur ce poste ; le terminal ci-dessous suit la connexion sur {name}.',
    title: 'Connecter le compte sur {name}',
    ready: 'La page de connexion est prête : elle s’ouvre dans le navigateur de ce poste.',
    opened: 'La page de connexion s’est ouverte sur ce poste.',
    open: 'Ouvrir la page sur ce poste',
    again: 'Rouvrir la page',
    copy: 'Copier l’adresse',
    copied: 'Adresse copiée',
    kept: 'Le compte sera gardé sur {name}, pas sur ce poste.',
    relayed: 'Le retour par localhost:{port} est relayé à {name} : finis la connexion dans la page, rien d’autre à faire.',
    relayBusy:
      'Le port {port} est pris sur ce poste : le retour par localhost ne peut pas être relayé. Colle le code de la page dans le terminal ci-dessous.',
    paste: 'Si la page affiche un code à la fin, colle-le dans le terminal ci-dessous.',
    device: 'Tape sur la page le code que le terminal affiche ci-dessous.',
    waiting: 'L’adresse de connexion s’affiche dans le terminal dès que le harness la donne.',
  },
  secrets: {
    inFile: 'Gardés dans un fichier sur {name}',
    inFileOne: 'Gardée dans un fichier sur {name}',
    detail:
      '~/.escouade/secrets.json, lisible du seul compte du service : un serveur n’a pas de trousseau. Pas dans le trousseau de ce poste.',
  },
  voice: {
    onPoste: 'La voix est celle de ce poste : son micro, sa transcription et ses clés. Seul le texte part vers {name}.',
  },
} as const satisfies Tree;

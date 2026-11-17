import type { Tree } from '../types';

// « Les docs d'un projet d'équipe » (TS-9, docs/TEAM-SERVER.md §2.7, C4-B): la spec, les décisions et les documents sur
// le serveur d'équipe, chiffrés ; le choix par projet ; les conflits ; l'import depuis le dépôt.
export default {
  place: {
    label: 'Docs du projet d’équipe',
    desc: 'Où vivent la spec, les décisions et les documents de ce projet : un choix de l’équipe, le même pour tous ses membres.',
    server: 'Sur le serveur d’équipe',
    repo: 'Dans le dépôt',
    serverNote:
      'Chiffrées : le serveur n’en lit ni les titres, ni les chemins, ni les textes. Ce poste en garde une copie, qui se lit et s’écrit hors ligne. Ce qu’écrit l’agent d’un ticket reste son brouillon jusqu’à la validation.',
    repoNote: 'Dans .escouade/ du dépôt, partagées par git comme avant ; le serveur ne porte que le Kanban et la mémoire.',
    toServer:
      'Passer sur le serveur importe la spec, les décisions et les documents du dépôt (numéros gardés) ; rien n’est retiré du dépôt.',
    toRepo: 'Revenir au dépôt y écrit les docs du serveur (git montre ce qui change, tu commites) ; le serveur les garde aussi.',
    noTeam: 'Ce projet n’est d’aucune équipe : ses docs sont dans son dépôt.',
    private: 'Projet privé : ses docs ne partent jamais sur le serveur d’équipe.',
    journal: 'Journal de l’équipe',
    follow: 'Suivre le journal d’un collègue',
    followHint: 'L’identifiant (32 caractères) que montre le poste d’un collègue, pour un projet que l’équipe a déjà.',
    followGo: 'Suivre',
    busy: 'Un instant…',
  },
  sync: {
    synced: 'Docs sur le serveur d’équipe',
    pending: {
      one: 'Docs sur le serveur d’équipe · {count} changement en attente',
      other: 'Docs sur le serveur d’équipe · {count} changements en attente',
    },
    offline: 'Hors ligne : les docs se lisent et s’écrivent sur ce poste, elles partiront au retour',
    offlinePending: {
      one: 'Hors ligne · {count} changement en attente : les docs se lisent et s’écrivent sur ce poste, il partira au retour',
      other: 'Hors ligne · {count} changements en attente : les docs se lisent et s’écrivent sur ce poste, ils partiront au retour',
    },
  },
  conflict: {
    text: 'La page « {title} » a été écrite ici et par un collègue en même temps, sur les mêmes lignes.',
    mine: 'Garder la mienne',
    theirs: 'Prendre la sienne',
    agent: 'Donner à l’agent rédacteur',
    agentBusy: 'L’agent rédacteur fusionne les deux versions.',
    compare: 'Comparer',
    mineTitle: 'La mienne',
    theirsTitle: 'La sienne',
  },
  lost: {
    text: 'Décision « {title} » : un collègue a changé « {field} » en même temps ; sa version a gagné. La tienne : « {mine} ».',
    ok: 'Compris',
  },
  repoFiles: {
    text: {
      one: 'Importé sur le serveur d’équipe, {count} fichier reste dans .escouade/ du dépôt : {files}{more}. Escouade ne le retire pas sans toi ; tu commites ensuite.',
      other:
        'Importés sur le serveur d’équipe, {count} fichiers restent dans .escouade/ du dépôt : {files}{more}. Escouade ne les retire pas sans toi ; tu commites ensuite.',
    },
    more: ' et {count} de plus',
    remove: 'Retirer du dépôt',
  },
  tooLarge: {
    text: {
      one: '{count} document dépasse {max} Mo : il reste sur ce poste, l’équipe ne le voit pas ({files}).',
      other: '{count} documents dépassent {max} Mo : ils restent sur ce poste, l’équipe ne les voit pas ({files}).',
    },
  },
  waiting: {
    text: {
      one: '{count} décision acceptée attend son numéro : le serveur le donne au retour en ligne, sans doublon possible.',
      other: '{count} décisions acceptées attendent leur numéro : le serveur les donne au retour en ligne, sans doublon possible.',
    },
  },
} as const satisfies Tree;

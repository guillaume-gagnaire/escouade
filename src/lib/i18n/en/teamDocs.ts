import { defineZone } from '../types';

// “A team project’s docs” (TS-9, docs/TEAM-SERVER.md §2.7, C4-B): the spec, the decisions and the documents on the
// team server, encrypted; the choice per project; the conflicts; the import from the repository.
export default defineZone('teamDocs', {
  place: {
    label: 'Team project docs',
    desc: 'Where this project’s spec, decisions and documents live: the team’s choice, the same for all its members.',
    server: 'On the team server',
    repo: 'In the repository',
    serverNote:
      'Encrypted: the server reads neither their titles, nor their paths, nor their texts. This machine keeps a copy, read and written offline too. What a ticket’s agent writes stays its draft until the ticket is validated.',
    repoNote: 'In the repository’s .escouade/, shared through git as before; the server carries only the board and the memory.',
    toServer:
      'Moving to the server imports the repository’s spec, decisions and documents (numbers kept); nothing is removed from the repository.',
    toRepo: 'Going back to the repository writes the server’s docs there (git shows what changes, you commit); the server keeps them too.',
    noTeam: 'This project belongs to no team: its docs are in its repository.',
    private: 'Private project: its docs never go to the team server.',
    journal: 'Team journal',
    follow: 'Follow a teammate’s journal',
    followHint: 'The id (32 characters) a teammate’s machine shows, for a project the team already has.',
    followGo: 'Follow',
    busy: 'One moment…',
  },
  sync: {
    synced: 'Docs on the team server',
    pending: {
      one: 'Docs on the team server · {count} change waiting',
      other: 'Docs on the team server · {count} changes waiting',
    },
    offline: 'Offline: the docs are read and written on this machine, they will leave once back online',
    offlinePending: {
      one: 'Offline · {count} change waiting: the docs are read and written on this machine, it will leave once back online',
      other: 'Offline · {count} changes waiting: the docs are read and written on this machine, they will leave once back online',
    },
  },
  conflict: {
    text: 'The page “{title}” was written here and by a teammate at the same time, on the same lines.',
    mine: 'Keep mine',
    theirs: 'Take theirs',
    agent: 'Hand it to the writer agent',
    agentBusy: 'The writer agent is merging both versions.',
    compare: 'Compare',
    mineTitle: 'Mine',
    theirsTitle: 'Theirs',
  },
  refused: {
    page: 'A page from the team was set aside: its path “{key}” leaves the spec’s folder. Nothing of it was written on this machine; the other pages are read.',
    decision:
      'A decision from the team was set aside: its id “{key}” is no decision’s. Nothing of it was written on this machine; the other decisions are read.',
    by: 'Written by the device {device}.',
    mine: 'Written by this machine.',
  },
  lost: {
    text: 'Decision “{title}”: a teammate changed “{field}” at the same time; their version won. Yours: “{mine}”.',
    ok: 'Got it',
  },
  repoFiles: {
    text: {
      one: 'Imported to the team server, {count} file is left in the repository’s .escouade/: {files}{more}. Escouade does not remove it without you; you commit afterwards.',
      other:
        'Imported to the team server, {count} files are left in the repository’s .escouade/: {files}{more}. Escouade does not remove them without you; you commit afterwards.',
    },
    more: ' and {count} more',
    remove: 'Remove from the repository',
  },
  tooLarge: {
    text: {
      one: '{count} document is over {max} MB: it stays on this machine, the team does not see it ({files}).',
      other: '{count} documents are over {max} MB: they stay on this machine, the team does not see them ({files}).',
    },
  },
  waiting: {
    text: {
      one: '{count} accepted decision waits for its number: the server gives it once back online, never twice.',
      other: '{count} accepted decisions wait for their numbers: the server gives them once back online, never twice.',
    },
  },
});

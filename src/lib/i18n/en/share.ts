import { defineZone } from '../types';

// The shared Kanban (ESC-190): see the French catalog.
export default defineZone('share', {
  card: {
    mark: 'Shared with the team · {file}',
    by: 'Added by {name}',
    theirsTodo: 'Added by {name} · the autopilot never starts it',
    takenBy: 'Taken by {name}',
    reviewAt: 'To review at {name}’s',
    prAt: 'In review at {name}’s',
    doneBy: 'Done by {name}',
    alsoOn: '{name} works on it too, says their file. Your agent goes on; your next status writes the file again.',
    left: '{name} no longer shares it. Your agent had taken it: it stays with you.',
    gone: 'Its file is no longer in the project’s folder: Escouade no longer writes it.',
    shareAgain: 'Share again',
    keepMine: 'Keep mine',
    takeTheirs: 'Take theirs',
    openFile: 'Open the file',
    unreadable: 'Shared ticket that can’t be read',
    newer: 'Written by a newer Escouade: read, never written.',
    someone: 'someone',
  },
  menu: {
    share: 'Share with the team',
    unshare: 'Stop sharing',
    hide: 'Hide for me',
    deleteForTeam: 'Delete for the team…',
    removeShared: 'Its shared file is deleted too, to be committed: it will go for the team at their pull.',
    removeTheirsTitle: 'Delete {key} for the team?',
    removeTheirsBody: 'Its file is deleted, to be committed. It will go for {name} and the rest of the team at their pull.',
  },
  board: {
    hidden: { one: '1 hidden ticket', other: '{count} hidden tickets' },
    show: 'Show',
    hide: 'Hide',
  },
  toast: {
    shared: '{key} shared: {file} is to be committed with your work',
    unshared: '{key} is no longer shared: the deletion of its file is to be committed',
  },
  form: {
    label: 'Share with the team',
    help: 'Escouade writes {file} in the project’s folder and keeps it up to date at each status; you commit it with your work.',
    helpNew:
      'Escouade writes its file in .escouade/tickets/ of the project’s folder and keeps it up to date at each status; you commit it with your work.',
    big: 'A joined file over 5 MB stays with you.',
    theirs: 'Added by {name}: what you change goes into its file, for the whole team.',
  },
  name: {
    title: 'Your name for the team',
    body: 'When you share a ticket of the Kanban, or move someone else’s on, your name is written in its file: it is what your teammates will read.',
    label: 'Your name',
    help: 'Taken from git config user.name. You can change it in Settings › General, and per project.',
    helpEmpty: 'You can change it in Settings › General, and per project.',
    preview: 'For them:',
    later: 'Later',
    save: 'Save',
  },
  settings: {
    group: 'Team',
    name: 'Your name',
    nameDesc:
      'Written in the tickets you share and in those you move on: your teammates see it on their Kanban. A project may have its own (Settings › Projects › Team).',
    id: 'Your id',
    idDesc:
      'What tells your tickets from a namesake’s: made from your git e-mail, so the same on your other machines. The e-mail itself is written nowhere.',
    projectGroup: 'Shared Kanban',
    projectName: 'Your name in this project',
    projectNameDesc: 'Empty: the one of Settings › General ({name}). Kept on this machine, never in .escouade.yml.',
    projectNameGit:
      'Empty: {name}, taken from git (git config user.name of this repository). Kept on this machine, never in .escouade.yml.',
    fromGit: 'Taken from git',
    noName: 'no name yet',
    files: 'Shared tickets',
    filesNone: 'None yet: they will go in {dir}, read again when they change (a pull, another branch, an edit).',
    filesDesc: {
      one: '1 shared ticket in {dir} ({mine} of yours), read again when it changes (a pull, another branch, an edit).',
      other: '{count} shared tickets in {dir} ({mine} of yours), read again when they change (a pull, another branch, an edit).',
    },
    shareNew: 'Share every new ticket',
    shareNewDesc: 'A ticket made here, by hand, by an import or through MCP, is shared from the start; you can always stop.',
  },
  git: {
    group: 'Shared tickets',
    note: 'Written by Escouade in the project’s folder; it never commits them. Commit them with your work, the team gets them at their pull.',
  },
});

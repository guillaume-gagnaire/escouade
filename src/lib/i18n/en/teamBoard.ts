import { defineZone } from '../types';

// The Kanban of a team's project (TS-6): see the French catalog.
export default defineZone('teamBoard', {
  card: {
    heldBy: 'Taken by {name} · since {since}',
    heldHere: 'Taken by this machine · since {since}',
    someone: 'a teammate',
    unreachable: 'Unreachable for {count} min',
    interrupted: 'Interrupted at {name}’s: their lease ran out. The autopilot does not take it up.',
    resume: 'Take up',
    reviewElsewhere: 'On the machine that took it: it is approved there.',
    assigned: 'Assigned to {name}',
    assignedMe: 'Assigned to me',
    localOnly: 'On this machine only',
    conflict: '{name} changed {field} at the same time',
    keepMineAgain: 'Put mine back',
    keepTheirs: 'Keep theirs',
    unmerged: '{name} changed the description at the same time, on the same lines: they do not merge.',
    keepMine: 'Keep mine',
    takeTheirs: 'Take theirs',
    field: {
      title: 'the title',
      description: 'the description',
      criteria: 'the criteria',
      column: 'the column',
      rank: 'the place',
      key: 'the key',
      maxLoops: 'the loops',
      progress: 'the progress',
      partial: 'the state',
      iteration: 'the loop',
      assignee: 'the assignment',
    },
  },
  menu: {
    assignMe: 'Assign to me',
    unassign: 'Unassign me',
    local: 'Do not share',
    share: 'Share with the team',
  },
  form: {
    local: 'Do not share',
    localHelp: 'On this machine only: nothing of this ticket goes to the team.',
    team: 'Ticket of the team',
    teamHelp: 'This ticket is on the team’s Kanban: what you change goes to your teammates.',
  },
  settings: {
    takeTodo: 'Also take the team’s “To do”',
    takeTodoDesc:
      'The autopilot takes the team’s tickets assigned to you; checked, also those of “To do”. A setting of this machine, never shared.',
  },
  link: {
    title: 'The team’s Kanban',
    create: 'Share its Kanban with the team',
    join: 'Join',
    idLabel: 'Id of a teammate’s Kanban',
    idPlaceholder: '32 characters',
    linked: 'The team’s Kanban · {id}',
    copy: 'Copy the id',
  },
});

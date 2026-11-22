import { defineZone } from '../types';

// The phone's app (MO-1, docs/MOBILE.md, direction B « the thread »): see the French catalog.
export default defineZone('mobile', {
  tabs: {
    label: 'The app’s tabs',
    inbox: 'Your turn',
    agents: 'Agents',
    kanban: 'Kanban',
    inboxWaiting: { one: 'Your turn, {count} waiting', other: 'Your turn, {count} waiting' },
  },
  create: {
    label: 'Create',
    title: 'Create',
    project: 'New project',
    projectHint: 'On a server: a cloned repository or one of its folders',
    agent: 'New agent',
    agentHint: 'In a project, with its first message',
    ticket: 'New ticket',
    ticketHint: 'Dictated, read over before it is created',
    soon: 'Coming to the phone soon',
    close: 'Close',
  },
  spaces: {
    label: 'The spaces',
    everywhere: 'Everywhere',
    everywhereLabel: { one: 'Everywhere, {count} waiting', other: 'Everywhere, {count} waiting' },
    chip: { one: '{name}, {count} waiting', other: '{name}, {count} waiting' },
    chipCut: '{name}, unreachable',
    chipConnecting: '{name}, connecting…',
  },
  band: {
    cut: 'Can’t reach {name} · retrying in {seconds} s',
    cutNow: 'Can’t reach {name} · retrying…',
    refused: '{name} refuses this phone: {reason}',
    retry: 'Try again',
  },
  inbox: {
    stopped: 'Your agents have stopped',
    asks: 'In your inbox',
    empty: 'Nothing is waiting for you',
    emptyIn: 'Nothing is waiting for you on {name}',
    emptyText: 'An agent that stops on a permission or a question, and each request of the « Your turn » inbox, show here.',
    countsOnly: 'The details of {name} don’t show here (the space’s policy): {waiting} waiting.',
  },
  agents: {
    empty: 'No agent',
    emptyIn: 'No agent on {name}',
    emptyText: 'The agents of your spaces, at work or stopped, show here with what they are doing.',
  },
  kanban: {
    choose: 'Choose a project',
    chooseText: 'A project’s Kanban opens here, one column at a time.',
  },
  welcome: {
    title: 'Escouade on this phone',
    text: 'Pair this phone with your computer or an Escouade server: it will show you what waits for you on each, and you will answer with a thumb.',
    how: 'Pairing by a QR code, shown on the computer or by a server, is coming soon.',
  },
  error: {
    deskOnly: '{command} is done on the computer.',
  },
});

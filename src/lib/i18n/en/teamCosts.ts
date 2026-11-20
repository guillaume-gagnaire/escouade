import { defineZone } from '../types';

// « Coûts » de l'équipe (TS-16): see the French catalog.
export default defineZone('teamCosts', {
  open: 'Costs',
  title: 'Costs and quotas',
  leadAdmin:
    'The totals of each device of the team, by day: encrypted by the administrators’ key, the team server reads none of them. Never a prompt, a reply, a file or a turn.',
  leadMember:
    'Your own totals, by day, and the total of each project: the other members’ totals are encrypted for the administrators alone.',
  loading: 'Reading the costs…',
  empty: 'No turn on a project of the team over this period.',
  period: {
    label: 'Period',
    week: 'Week',
    month: 'Month',
    custom: 'Chosen period',
    from: 'From',
    to: 'to',
  },
  by: {
    label: 'Group',
    member: 'By member',
    project: 'By project',
    ticket: 'By ticket',
    model: 'By harness and model',
  },
  col: {
    name: 'Name',
    tokens: 'Tokens',
    cost: 'Cost',
    quota: 'Highest quota',
  },
  total: 'Total',
  member: {
    me: 'You',
    other: 'Member {id}',
    unnamed: 'Unnamed (by project)',
  },
  plan: {
    only: 'API equivalent',
    some: 'partly an API equivalent',
    hint: 'A Claude Max or ChatGPT Pro account has no cost per turn: this is the quota it used, counted at the API’s price.',
  },
  budgets: {
    title: 'Budgets for the month',
    none: 'The team set no budget: an administrator writes them in the team’s file (costs.budgets).',
    stop: 'Beyond a budget, the autopilot stops (imposed by the team).',
    spent: '{spent} of {budget}',
  },
  summary: {
    title: 'The team’s month',
    team: 'The whole team',
  },
  key: {
    none: 'No administrator drew the key of the costs yet: your totals wait on this device.',
    stale: 'The key of the costs waits to be changed (an administrator left): your totals wait on this device.',
    epoch: 'Administrators’ key no. {epoch}',
  },
  mine: {
    title: 'What goes up of you',
    lead: 'Your totals by day, never your work: the tokens, the estimated cost, the highest quota and the total of each ticket, once an hour.',
    pending: {
      one: '{count} day waits to be sent.',
      other: '{count} days wait to be sent.',
    },
    none: 'Nothing has gone from this device yet.',
    show: 'See what went',
    hide: 'Hide',
    send: 'Send now',
  },
  grain: {
    project: 'The team {team} sees the totals by project: yours go without your name.',
    member: 'The team {team} sees the totals by member: its administrators see your totals by day, under your name.',
    memberUntold: 'The team {team} chose to see the totals by member: yours go without your name until you read the screen that says so.',
  },
  told: {
    title: 'Your team sees your totals by day',
    body: 'The team {team} switched to “by member”: its administrators now see, under your name, your totals by day (tokens, estimated cost, highest quota, total of each ticket).',
    never: 'Never your work: no prompt, no reply, no file name, no turn goes.',
    where: 'You can see what goes up of you at any time in Team › Costs.',
    ok: 'Got it',
    see: 'See what goes up',
  },
  notice: {
    launchBody: '{text}. Launch it anyway?',
    launchStops: '{text}: the autopilot of this project is stopped by the team {team}. Launch it anyway?',
    stopped: 'Autopilot stopped: {text}.',
  },
});

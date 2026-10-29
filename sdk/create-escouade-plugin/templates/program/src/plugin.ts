// What the plugin does: its handlers, kept apart from main.ts so that the tests drive them without a process.
import { createPlugin } from '@escouade-dev/plugin';

export const plugin = createPlugin();

// A fact of permissions.events: a ticket changed columns.
plugin.onEvent('ticket.column', (fact, p) => {
  if (fact.ticket?.column === 'test') p.log.info(`${fact.ticket.key} is ready to test (${fact.project?.name ?? ''})`);
});

// A permission an agent asks for, before the user is asked (permissions.decide). Answer within 500 ms.
plugin.decidePermission(({ request }) => {
  const command = typeof request.input?.command === 'string' ? request.input.command : '';
  if (request.kind === 'command' && /\bgit\s+push\b.*(--force\b|\s-f\b)/.test(command)) {
    return { decision: 'deny', message: 'No force push in this team: push a new commit instead.' };
  }
  // Leaves it to the user, as without the plugin.
  return { decision: 'ask' };
});

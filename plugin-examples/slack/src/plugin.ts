// Slack: the tickets to test and the agents waiting, in each project's channel. The bot's token is kept in the
// system's keychain (permissions.secrets), never in a file; the channel of a project comes from channels.json, or
// from the project's name (escouade-<name>).
import { readFileSync } from 'node:fs';
import { createPlugin, type Fact, type Plugin } from '@escouade-dev/plugin';

export interface Options {
  /** The channel of each project, by the project's name. */
  channels?: Record<string, string>;
}

/** channels.json, beside escouade-plugin.json (src/ and dist/ are one level below it). */
function readChannels(): Record<string, string> {
  try {
    return JSON.parse(readFileSync(new URL('../channels.json', import.meta.url), 'utf8'));
  } catch {
    return {};
  }
}

/** The channel of a project: the one channels.json gives it, else escouade-<its name>, as Slack writes names. */
export function channelOf(project: string, channels: Record<string, string>): string {
  const named = channels[project];
  if (named) return named.replace(/^#/, '');
  const slug = project
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return `escouade-${slug || 'project'}`.slice(0, 80);
}

/** What a fact says in Slack, or nothing for the facts it keeps quiet about. */
export function textOf(fact: Fact): string | null {
  if (fact.name === 'ticket.column' && fact.data.to === 'review') {
    return `:test_tube: *${fact.ticket?.key ?? ''}* to test: ${fact.ticket?.title ?? ''}`;
  }
  if (fact.name === 'agent.status' && fact.data.status === 'waiting') {
    return `:raised_hand: *${fact.agent?.name ?? 'An agent'}* waits for you`;
  }
  return null;
}

export function slack(options: Options = {}): Plugin {
  const channels = options.channels ?? readChannels();

  return createPlugin().onEvent(['ticket.column', 'agent.status'], async (fact, p) => {
    const text = textOf(fact);
    if (!text || !fact.project) return;
    const token = await p.secret('token');
    if (!token) return p.log.warn('no Slack token kept: nothing sent (Plugins › Slack › Settings)');
    // A bot token, never anything else, goes to Slack (and never into the journal).
    if (!token.startsWith('xoxb-')) return p.log.warn('the token kept is not a Slack bot token (xoxb-…): nothing sent');
    const channel = channelOf(fact.project.name, channels);
    try {
      const res = await fetch('https://slack.com/api/chat.postMessage', {
        method: 'POST',
        headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json; charset=utf-8' },
        body: JSON.stringify({ channel, text }),
      });
      const answer = (await res.json()) as { ok: boolean; error?: string };
      if (answer.ok) p.log.info(`posted in #${channel}`);
      else p.log.warn(`Slack refused the message in #${channel}: ${answer.error ?? res.status}`);
    } catch (e) {
      p.log.error(`Slack could not be reached: ${(e as Error).message}`);
    }
  });
}

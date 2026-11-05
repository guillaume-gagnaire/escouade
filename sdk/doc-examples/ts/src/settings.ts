// `secrets/get`, `settings/get` and the journal.
import type { Fact, Plugin } from '@escouade-dev/plugin';

// One of `permissions.secrets`; null while the user has not given it.
export function token(p: Plugin) {
  return p.secret('token');
}

// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
export async function channel(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.channel;
}

// A line of its journal: `debug`, `info`, `warn` or `error`.
export function journal(fact: Fact, p: Plugin) {
  p.log.info(`${fact.name} received`);
}

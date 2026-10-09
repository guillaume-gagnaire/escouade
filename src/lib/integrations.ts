// The external ticket systems as the window shows them, and the changes of a project's links.

import type { Column, Container, ExternalIssue, ExternalState, ProjectIntegrations, Service } from './types';

export const SERVICE_IDS: Service[] = ['jira', 'trello', 'github'];

export const SERVICES: Record<
  Service,
  { name: string; letter: string; color: string; ink: string; container: string; placeholder: string }
> = {
  jira: {
    name: 'Jira',
    letter: 'J',
    color: 'oklch(0.62 0.17 255)',
    ink: '#fff',
    container: 'Projet',
    placeholder: 'Rechercher par clé ou par texte…',
  },
  trello: {
    name: 'Trello',
    letter: 'T',
    color: 'oklch(0.66 0.12 230)',
    ink: '#fff',
    container: 'Tableau',
    placeholder: 'Rechercher une carte…',
  },
  github: {
    name: 'GitHub Issues',
    letter: 'GH',
    color: '#e8e3dc',
    ink: '#1b1917',
    container: 'Dépôt',
    placeholder: 'Rechercher une issue…',
  },
};

/** How a service names itself in a sentence (« depuis GitHub »). */
const SHORT: Record<Service, string> = { jira: 'Jira', trello: 'Trello', github: 'GitHub' };

export const shortName = (service: Service) => SHORT[service];

/** The page where Trello gives a token for an API key (read and write, no expiry). */
export function trelloTokenPage(key: string): string {
  return `https://trello.com/1/authorize?expiration=never&scope=read,write&response_type=token&name=Escouade&key=${encodeURIComponent(key.trim())}`;
}

/** « 3 tickets importés depuis Jira et Trello ». */
export function importedLabel(issues: { service: Service }[]): string {
  const names = [...new Set(issues.map((i) => i.service))].map((s) => SHORT[s]);
  const from = names.length > 1 ? `${names.slice(0, -1).join(', ')} et ${names[names.length - 1]}` : (names[0] ?? '');
  const n = issues.length;
  return `${n} ${n > 1 ? 'tickets importés' : 'ticket importé'} depuis ${from}`;
}

/** An external ticket's key in a selection. */
export const issueKey = (i: ExternalIssue) => `${i.service}|${i.id}`;

/** The project's source for `service` becomes `container` (with its default states), or none. */
export function linkSource(
  p: ProjectIntegrations,
  service: Service,
  container: Container | null,
  states: Partial<Record<Column, ExternalState>>,
): ProjectIntegrations {
  if (!container) return { ...p, links: p.links.filter((l) => l.service !== service) };
  const link = { service, container: container.id, name: container.name, states: { ...states } };
  const at = p.links.findIndex((l) => l.service === service);
  const links = at < 0 ? [...p.links, link] : p.links.map((l, i) => (i === at ? link : l));
  return { ...p, links };
}

/** The state a column gives the external tickets of `service`'s source (none: left as they are). */
export function setColumnState(p: ProjectIntegrations, service: Service, column: Column, state: ExternalState | null): ProjectIntegrations {
  return {
    ...p,
    links: p.links.map((l) => {
      if (l.service !== service) return l;
      const states = { ...l.states };
      if (state) states[column] = state;
      else delete states[column];
      return { ...l, states };
    }),
  };
}

/** A column's arrival writes a comment on the external ticket, or no longer does. */
export function toggleComment(p: ProjectIntegrations, column: Column): ProjectIntegrations {
  const comments = p.comments.includes(column) ? p.comments.filter((c) => c !== column) : [...p.comments, column];
  return { ...p, comments };
}

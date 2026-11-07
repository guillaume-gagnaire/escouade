// The external ticket systems as the window shows them, and the changes of a project's links.

import { fList } from './format';
import { t } from './i18n';
import type { Column, Container, ExternalIssue, ExternalState, ProjectIntegrations, Service, SourceDecl } from './types';

/**
 * The sources, in the registry's order: read from the registry of the core at the window's start (`setSources`).
 * Until then, and for a core that does not tell (a test's), the seven this version brings.
 */
export const SERVICE_IDS: Service[] = ['jira', 'trello', 'github', 'linear', 'clickup', 'gitlab', 'azure'];

/**
 * The sources whose account is a code host's (GitHub Issues: github.com's; GitLab Issues: every GitLab instance's;
 * Azure Boards: every Azure DevOps organization's).
 */
export const HOSTED: Service[] = ['github', 'gitlab', 'azure'];

/** The sources that have no account of their own: connected and disconnected as their code host, of this kind. */
export const BY_FORGE: Record<Service, string | undefined> = { gitlab: 'gitlab', azure: 'azure' };

/**
 * How the window shows each service. `container` (what the service calls the place its tickets are in) and
 * `placeholder` (the hint of the search field) are texts: getters, read when shown, so that they follow the language.
 */
export const SERVICES: Record<
  Service,
  { name: string; letter: string; color: string; ink: string; container: string; placeholder: string }
> = {
  jira: {
    name: 'Jira',
    letter: 'J',
    color: 'oklch(0.62 0.17 255)',
    ink: '#fff',
    get container() {
      return t('integrations.container.jira');
    },
    get placeholder() {
      return t('integrations.searchHint.jira');
    },
  },
  trello: {
    name: 'Trello',
    letter: 'T',
    color: 'oklch(0.66 0.12 230)',
    ink: '#fff',
    get container() {
      return t('integrations.container.trello');
    },
    get placeholder() {
      return t('integrations.searchHint.trello');
    },
  },
  github: {
    name: 'GitHub Issues',
    letter: 'GH',
    color: '#e8e3dc',
    ink: '#1b1917',
    get container() {
      return t('integrations.container.github');
    },
    get placeholder() {
      return t('integrations.searchHint.github');
    },
  },
  linear: {
    name: 'Linear',
    letter: 'L',
    color: 'oklch(0.58 0.16 280)',
    ink: '#fff',
    get container() {
      return t('integrations.container.linear');
    },
    get placeholder() {
      return t('integrations.searchHint.linear');
    },
  },
  clickup: {
    name: 'ClickUp',
    letter: 'C',
    color: 'oklch(0.6 0.2 300)',
    ink: '#fff',
    get container() {
      return t('integrations.container.clickup');
    },
    get placeholder() {
      return t('integrations.searchHint.clickup');
    },
  },
  gitlab: {
    name: 'GitLab Issues',
    letter: 'GL',
    color: 'oklch(0.66 0.17 45)',
    ink: '#fff',
    get container() {
      return t('integrations.container.gitlab');
    },
    get placeholder() {
      return t('integrations.searchHint.gitlab');
    },
  },
  azure: {
    name: 'Azure Boards',
    letter: 'AB',
    color: 'oklch(0.6 0.15 245)',
    ink: '#fff',
    get container() {
      return t('integrations.container.azure');
    },
    get placeholder() {
      return t('integrations.searchHint.azure');
    },
  },
};

/** How a service names itself in a sentence (« depuis GitHub »). */
const SHORT: Record<Service, string> = {
  jira: 'Jira',
  trello: 'Trello',
  github: 'GitHub',
  linear: 'Linear',
  clickup: 'ClickUp',
  gitlab: 'GitLab',
  azure: 'Azure Boards',
};

export const shortName = (service: Service) => SHORT[service];

/** The page where Trello gives a token for an API key (read and write, no expiry). */
export function trelloTokenPage(key: string): string {
  return `https://trello.com/1/authorize?expiration=never&scope=read,write&response_type=token&name=Escouade&key=${encodeURIComponent(key.trim())}`;
}

/** « 3 tickets importés depuis Jira et Trello ». */
export function importedLabel(issues: { service: Service }[]): string {
  const from = fList([...new Set(issues.map((i) => i.service))].map((s) => SHORT[s]));
  return t('integrations.imported', { count: issues.length, from });
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

/**
 * The registry of the sources, as the core tells it at the window's start (`InitialState::sources`): the lists above
 * are filled in place, so that what read them keeps reading them. A source the window has no logo for (a plugin's) is
 * shown by its initials, in a neutral color.
 */
export function setSources(sources: SourceDecl[]) {
  SERVICE_IDS.splice(0, SERVICE_IDS.length, ...sources.map((s) => s.id));
  HOSTED.splice(0, HOSTED.length, ...sources.filter((s) => s.host).map((s) => s.id));
  for (const id of Object.keys(BY_FORGE)) delete BY_FORGE[id];
  for (const s of sources) {
    if (s.host && !s.ownAccount) BY_FORGE[s.id] = s.host;
    SHORT[s.id] = s.short;
    SERVICES[s.id] ??= {
      name: s.name,
      letter: s.name.slice(0, 2).toUpperCase(),
      color: 'oklch(0.55 0.02 260)',
      ink: '#fff',
      container: t('integrations.container.other'),
      placeholder: t('integrations.searchHint.other'),
    };
  }
}

/** The label of the container's menu: the source's own words (« Projet Jira »), a generic one for a source the window has no words for. */
export function pickLabel(service: Service): string {
  switch (service) {
    case 'jira':
    case 'trello':
    case 'github':
    case 'linear':
    case 'clickup':
    case 'gitlab':
    case 'azure':
      return t(`integrations.sources.pick.${service}`);
    default:
      return t('integrations.sources.pick.other');
  }
}

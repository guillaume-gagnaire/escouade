import { describe, expect, it } from 'vitest';
import { importedLabel, linkSource, SERVICE_IDS, SERVICES, setColumnState, toggleComment, trelloTokenPage } from './integrations';
import type { ExternalIssue, ProjectIntegrations } from './types';

const none = (): ProjectIntegrations => ({ links: [], comments: ['review', 'done'] });

function issue(over: Partial<ExternalIssue> = {}): ExternalIssue {
  return {
    service: 'jira',
    id: 'ATL-1',
    key: 'ATL-1',
    title: 'Un ticket',
    kind: 'Story',
    meta: [],
    url: '',
    description: '',
    criteria: [],
    container: 'ATL',
    imported: false,
    ...over,
  };
}

describe('integrations', () => {
  it('knows each service by its letter, color and container', () => {
    expect(SERVICE_IDS).toEqual(['jira', 'trello', 'github']);
    expect(SERVICES.jira).toMatchObject({ name: 'Jira', letter: 'J', container: 'Projet' });
    expect(SERVICES.trello).toMatchObject({ name: 'Trello', letter: 'T', container: 'Tableau' });
    expect(SERVICES.github).toMatchObject({ name: 'GitHub Issues', letter: 'GH', container: 'Dépôt' });
  });

  it('opens Trello’s token page for the key typed', () => {
    expect(trelloTokenPage(' k 1 ')).toBe(
      'https://trello.com/1/authorize?expiration=never&scope=read,write&response_type=token&name=Escouade&key=k%201',
    );
  });

  it('says how many tickets came and from where', () => {
    expect(importedLabel([issue()])).toBe('1 ticket importé depuis Jira');
    expect(importedLabel([issue(), issue({ service: 'trello' }), issue()])).toBe('3 tickets importés depuis Jira et Trello');
    expect(importedLabel([issue({ service: 'github' }), issue(), issue({ service: 'trello' })])).toBe(
      '3 tickets importés depuis GitHub, Jira et Trello',
    );
  });

  it('links one source per service with its default states, and unlinks it', () => {
    const doing = { id: '3', name: 'In Progress' };
    let p = linkSource(none(), 'jira', { id: 'ATL', name: 'ATL — Atlas' }, { doing, review: doing });
    expect(p.links).toEqual([{ service: 'jira', container: 'ATL', name: 'ATL — Atlas', states: { doing, review: doing } }]);
    p = linkSource(p, 'trello', { id: 'b1', name: 'Atlas' }, {});
    // Another container replaces the service's link, in its place.
    p = linkSource(p, 'jira', { id: 'MOB', name: 'MOB — Mobile' }, {});
    expect(p.links.map((l) => [l.service, l.container])).toEqual([
      ['jira', 'MOB'],
      ['trello', 'b1'],
    ]);
    p = linkSource(p, 'jira', null, {});
    expect(p.links.map((l) => l.service)).toEqual(['trello']);
    expect(p.comments).toEqual(['review', 'done']);
  });

  it('sets a column’s state of a source, or leaves it as it is', () => {
    let p = linkSource(none(), 'trello', { id: 'b1', name: 'Atlas' }, {});
    p = setColumnState(p, 'trello', 'done', { id: 'l3', name: 'Recette' });
    expect(p.links[0].states).toEqual({ done: { id: 'l3', name: 'Recette' } });
    p = setColumnState(p, 'trello', 'done', null);
    expect(p.links[0].states).toEqual({});
  });

  it('turns a column’s comment on and off', () => {
    let p = toggleComment(none(), 'doing');
    expect(p.comments).toEqual(['review', 'done', 'doing']);
    p = toggleComment(p, 'review');
    expect(p.comments).toEqual(['done', 'doing']);
  });
});

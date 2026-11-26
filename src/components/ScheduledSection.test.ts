import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { menu } from '../lib/menu.svelte';
import { app } from '../lib/state.svelte';
import type { ScheduledTask } from '../lib/types';
import { agent, fakeBackend, project, resetApp, ticket } from '../test/ipc';
import ScheduledSection from './ScheduledSection.svelte';

const P = project();

function task(over: Partial<ScheduledTask>): ScheduledTask {
  return {
    id: 's1',
    projectId: 'p1',
    title: 'Revue des dépendances',
    description: 'Mets à jour les dépendances.',
    criteria: [],
    maxLoops: 5,
    engine: null,
    account: '',
    model: '',
    effort: '',
    rhythm: { kind: 'weekly', days: [1], day: 1, hour: 7, minute: 0, cron: '' },
    catchUp: true,
    paused: false,
    createdAt: 1,
    since: 1,
    lastDue: null,
    lastTicket: null,
    lastRunAt: null,
    lastNote: null,
    next: new Date(2026, 10, 9, 7, 0).getTime(),
    ...over,
  };
}

const row = (title: string) => screen.getByText(title).closest<HTMLElement>('[data-task]')!;

describe('ScheduledSection', () => {
  beforeEach(() => resetApp({ projects: [P] }));

  it('lists the project’s tasks with their rhythm, their next due time, « en cours » or their pause (programmees-barre.png)', () => {
    fakeBackend();
    const running = ticket({ id: 't44', key: 'DEM-44', column: 'doing', agentId: 'a1' });
    app.tickets = { t44: running };
    app.scheduled = [
      task({ lastTicket: 't44' }),
      task({
        id: 's2',
        title: 'Rapport des erreurs Sentry',
        rhythm: { kind: 'daily', days: [], day: 1, hour: 23, minute: 30, cron: '' },
      }),
      task({
        id: 's3',
        title: 'Purge des comptes inactifs',
        paused: true,
        next: null,
        rhythm: { kind: 'monthly', days: [], day: 1, hour: 9, minute: 0, cron: '' },
      }),
      task({ id: 's4', projectId: 'other', title: 'Ailleurs' }),
    ];
    render(ScheduledSection, { project: P });
    const section = screen.getByRole('region', { name: 'Programmées' });
    expect(section).toHaveTextContent('3');
    expect(row('Revue des dépendances')).toHaveTextContent('Chaque lundi 7:00 · en cours (DEM-44)');
    expect(row('Rapport des erreurs Sentry')).toHaveTextContent('Chaque jour 23:30 ·');
    expect(row('Purge des comptes inactifs')).toHaveTextContent('Le 1er du mois 9:00 · en pause');
    expect(row('Purge des comptes inactifs')).toHaveClass('paused');
    expect(screen.queryByText('Ailleurs')).toBeNull();
    // Each its switch: on while it runs, off paused.
    expect(within(row('Revue des dépendances')).getByRole('switch')).toHaveAttribute('aria-checked', 'true');
    expect(
      within(row('Purge des comptes inactifs')).getByRole('switch', { name: 'Reprendre « Purge des comptes inactifs »' }),
    ).toHaveAttribute('aria-checked', 'false');
  });

  it('opens the ticket under way from « en cours »', async () => {
    fakeBackend();
    app.agents = { a1: agent({ id: 'a1' }) };
    app.tickets = { t44: ticket({ id: 't44', key: 'DEM-44', column: 'doing', agentId: 'a1' }) };
    app.scheduled = [task({ lastTicket: 't44' })];
    render(ScheduledSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'en cours (DEM-44)' }));
    expect(app.ui.selectedAgent.p1).toBe('a1');
    // Not the task's form: the link only opens the ticket.
    expect(app.modal).toBeNull();
  });

  it('pauses and resumes a task with its switch', async () => {
    const backend = fakeBackend();
    app.scheduled = [task({}), task({ id: 's3', title: 'Purge', paused: true, next: null })];
    render(ScheduledSection, { project: P });
    await userEvent.click(screen.getByRole('switch', { name: 'Mettre en pause « Revue des dépendances »' }));
    await userEvent.click(screen.getByRole('switch', { name: 'Reprendre « Purge »' }));
    expect(backend.called('scheduled_pause').map((c) => c.args)).toEqual([
      { id: 's1', paused: true },
      { id: 's3', paused: false },
    ]);
    // The switch does not open the form.
    expect(app.modal).toBeNull();
  });

  it('opens the ticket’s form in « Programmer » from « ＋ », and on the task from its row', async () => {
    fakeBackend();
    const t = task({});
    app.scheduled = [t];
    render(ScheduledSection, { project: P });
    await userEvent.click(screen.getByRole('button', { name: 'Programmer une tâche' }));
    expect(app.modal).toEqual({ kind: 'schedule', projectId: 'p1' });
    app.modal = null;
    await userEvent.click(row('Revue des dépendances'));
    expect(app.modal).toMatchObject({ kind: 'schedule', projectId: 'p1', task: { id: 's1' } });
  });

  it('deletes a task from its menu, once confirmed', async () => {
    const backend = fakeBackend();
    app.scheduled = [task({})];
    render(ScheduledSection, { project: P });
    await userEvent.pointer({ keys: '[MouseRight]', target: row('Revue des dépendances') });
    const del = menu.open!.items.find((i) => i.label === 'Supprimer')!;
    del.onClick!();
    expect(app.modal).toMatchObject({ kind: 'confirm', title: 'Supprimer la tâche programmée ?' });
    await (app.modal as unknown as { onConfirm: () => Promise<void> }).onConfirm();
    expect(backend.called('scheduled_delete').map((c) => c.args)).toEqual([{ id: 's1' }]);
  });

  it('shows who scheduled a task an agent made, that it awaits the go-ahead, and accepts it (ESC-647)', async () => {
    const backend = fakeBackend();
    app.scheduled = [
      task({
        awaiting: true,
        next: null,
        print: 'abc123',
        author: { agentId: 'a1', agentName: 'Claude', projectId: 'p1', ticketId: 't3', ticketKey: 'DEM-3', at: 1 },
      }),
      task({ id: 's2', title: 'À moi', author: null }),
    ];
    render(ScheduledSection, { project: P });
    const r = row('Revue des dépendances');
    expect(r).toHaveTextContent('Chaque lundi 7:00 · attend ton accord');
    expect(r).toHaveClass('awaiting');
    expect(within(r).getByText('par Claude · DEM-3')).toHaveAccessibleDescription(
      'Programmée par l’agent Claude : elle ne lance rien avant ton accord.',
    );
    // No switch while it awaits: its go-ahead first.
    expect(within(r).queryByRole('switch')).toBeNull();
    // The user's own task names no author.
    expect(row('À moi').querySelector('[data-author]')).toBeNull();
    await userEvent.click(within(r).getByRole('button', { name: 'Accepter « Revue des dépendances », programmée par Claude' }));
    expect(backend.called('scheduled_accept').map((c) => c.args)).toEqual([{ id: 's1', print: 'abc123' }]);
    // The button does not open the form.
    expect(app.modal).toBeNull();
  });

  it('says what it is for while it has none', () => {
    fakeBackend();
    render(ScheduledSection, { project: P });
    expect(screen.getByRole('region', { name: 'Programmées' })).toHaveTextContent('Aucune tâche programmée');
  });
});

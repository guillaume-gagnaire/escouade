import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { ScheduledTask } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import TicketForm from './TicketForm.svelte';

const MON = new Date(2026, 10, 2, 7, 0).getTime();
const WEEK = 7 * 86_400_000;

/** The form in « Programmer », as App shows it (`app.modal` is the dialog it is). */
function show(task?: ScheduledTask) {
  app.modal = { kind: 'schedule', projectId: 'p1', task };
  return render(TicketForm, { projectId: 'p1', scheduling: true, task, onsubmit: () => false });
}

/** A backend whose « Prochaines » are three Mondays, or refuses the rhythm as `refuse` says. */
function backend(refuse?: (r: { kind: string; cron: string }) => string | null) {
  return fakeBackend({
    scheduled_preview: ({ rhythm }) => {
      const why = refuse?.(rhythm);
      if (why) throw new Error(why);
      return [MON, MON + WEEK, MON + 2 * WEEK];
    },
    scheduled_create: () => ({}),
    scheduled_update: () => ({}),
  });
}

const submit = () => screen.getByRole('button', { name: 'Programmer' });

describe('TicketForm in « Programmer »', () => {
  beforeEach(() => resetApp());

  it('shows its rhythm under its title, the prompt after, and no branch, files nor sharing (programmees-formulaire.png)', async () => {
    backend();
    show();
    const dialog = screen.getByRole('dialog', { name: 'Nouvelle tâche programmée' });
    const kinds = within(dialog).getByRole('group', { name: 'Rythme' });
    expect(
      within(kinds)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['Chaque jour', 'Chaque semaine', 'Chaque mois', 'Cron']);
    expect(within(kinds).getByRole('button', { name: 'Chaque semaine' })).toHaveAttribute('aria-pressed', 'true');
    const days = within(dialog).getByRole('group', { name: 'Jours' });
    expect(
      within(days)
        .getAllByRole('button')
        .map((b) => b.textContent),
    ).toEqual(['L', 'M', 'M', 'J', 'V', 'S', 'D']);
    expect(within(days).getByRole('button', { name: 'lundi' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(dialog).getByLabelText('Heure')).toHaveValue('07:00');
    expect(within(dialog).getByRole('checkbox', { name: /Lancer au retour d’Escouade, une seule fois/ })).toBeChecked();
    expect(dialog).toHaveTextContent('Jamais deux à la fois : si la précédente tourne encore, l’échéance passe son tour.');
    await waitFor(() =>
      expect(dialog).toHaveTextContent('Prochaines : lun. 2 nov. 7:00 · lun. 9 nov. 7:00 · lun. 16 nov. 7:00 · 0 7 * * 1'),
    );
    // The prompt, the criteria and the account, model and effort of a ticket, under it.
    expect(within(dialog).getByRole('textbox', { name: 'Consigne' })).toBeInTheDocument();
    expect(within(dialog).getByRole('group', { name: 'Modèle' })).toBeInTheDocument();
    for (const gone of [/^Branche/, /Joindre/]) expect(within(dialog).queryByRole('button', { name: gone })).toBeNull();
    expect(within(dialog).queryByRole('switch')).toBeNull();
  });

  it('schedules the task with its rhythm, its prompt and its catch-up', async () => {
    const b = backend();
    show();
    const dialog = screen.getByRole('dialog');
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Titre du ticket' }), 'Revue des dépendances');
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Consigne' }), 'Mets à jour les dépendances.');
    await userEvent.click(within(dialog).getByRole('button', { name: 'jeudi' }));
    await userEvent.click(within(dialog).getByRole('checkbox'));
    await waitFor(() => expect(submit()).toBeEnabled());
    await userEvent.click(submit());
    await waitFor(() => expect(b.called('scheduled_create')).toHaveLength(1));
    const { projectId, draft } = b.called('scheduled_create')[0].args;
    expect(projectId).toBe('p1');
    expect(draft).toMatchObject({
      title: 'Revue des dépendances',
      description: 'Mets à jour les dépendances.',
      rhythm: { kind: 'weekly', days: [1, 4], hour: 7, minute: 0 },
      catchUp: false,
    });
    expect(app.modal).toBeNull();
  });

  it('writes a cron expression from the simple rhythm, and says why one is refused', async () => {
    backend((r) => (r.kind === 'cron' && r.cron.startsWith('0 0 31 2') ? 'Ce rythme ne tombe jamais.' : null));
    show();
    const dialog = screen.getByRole('dialog');
    await userEvent.type(within(dialog).getByRole('textbox', { name: 'Titre du ticket' }), 'Jamais');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Cron' }));
    const cron = within(dialog).getByRole('textbox', { name: 'Expression' });
    expect(cron).toHaveValue('0 7 * * 1');
    expect(within(dialog).queryByLabelText('Heure')).toBeNull();
    await userEvent.clear(cron);
    await userEvent.type(cron, '0 0 31 2 *');
    await waitFor(() => expect(dialog).toHaveTextContent('Ce rythme ne tombe jamais.'));
    expect(submit()).toBeDisabled();
  });

  it('edits a task as it is, and saves it in place', async () => {
    const b = backend();
    const task: ScheduledTask = {
      id: 's1',
      projectId: 'p1',
      title: 'Purge des comptes inactifs',
      description: 'Purge-les.',
      criteria: ['Rien de cassé'],
      maxLoops: 3,
      engine: null,
      account: '',
      model: '',
      effort: '',
      rhythm: { kind: 'monthly', days: [], day: 1, hour: 9, minute: 0, cron: '' },
      catchUp: false,
      paused: false,
      createdAt: 1,
      since: 1,
      lastDue: null,
      lastTicket: null,
      lastRunAt: null,
      lastNote: null,
      next: MON,
    };
    show(task);
    const dialog = screen.getByRole('dialog', { name: 'Modifier la tâche programmée' });
    expect(within(dialog).getByRole('button', { name: 'Chaque mois' })).toHaveAttribute('aria-pressed', 'true');
    expect(within(dialog).getByLabelText('Jour du mois')).toHaveValue(1);
    expect(within(dialog).getByLabelText('Heure')).toHaveValue('09:00');
    expect(within(dialog).getByRole('checkbox')).not.toBeChecked();
    expect(within(dialog).getByRole('textbox', { name: 'Consigne' })).toHaveValue('Purge-les.');
    const save = within(dialog).getByRole('button', { name: 'Enregistrer' });
    await waitFor(() => expect(save).toBeEnabled());
    await userEvent.click(save);
    await waitFor(() => expect(b.called('scheduled_update')).toHaveLength(1));
    expect(b.called('scheduled_update')[0].args).toMatchObject({
      id: 's1',
      draft: {
        title: 'Purge des comptes inactifs',
        criteria: ['Rien de cassé'],
        maxLoops: 3,
        rhythm: { kind: 'monthly', day: 1, hour: 9 },
      },
    });
  });
});

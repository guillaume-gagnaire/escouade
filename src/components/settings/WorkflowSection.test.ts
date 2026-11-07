import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { Project, WorkflowFix } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import { gsdScan, gsdWorkflow } from '../../test/workflows';
import SettingsModal from '../modals/SettingsModal.svelte';

vi.mock('../../lib/terminals', () => ({ launchLog: () => ({}), disposeLog() {} }));

const save = () => userEvent.click(screen.getByRole('button', { name: /^(Enregistrer|Save)$/ }));
const section = () => document.querySelector<HTMLElement>('#sg-workflow')!;
const sentFixes = (backend: ReturnType<typeof fakeBackend>) =>
  backend.called('update_project').at(-1)!.args.project.workflowFixes as WorkflowFix[];

function open(over: Partial<Project> = {}, scan = gsdScan(), handlers: Record<string, (args: any) => unknown> = {}) {
  resetApp({ projects: [project(over)] });
  app.modal = { kind: 'settings' };
  const backend = fakeBackend({
    get_workflows: () => scan,
    refresh_workflows: () => ({ ...scan, readAt: scan.readAt + 60_000 }),
    ...handlers,
  });
  render(SettingsModal, { tab: 'projects', projectId: 'p1' });
  return backend;
}

describe('Réglages › Projets › Workflow (ESC-195)', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date(2026, 9, 26, 11, 0));
  });

  it('says what the rules read and when, reads again on « Relire », and lists what was found', async () => {
    const backend = open();
    const s = within(section());
    expect(await s.findByText('Lu par les règles à 10:12')).toBeInTheDocument();
    expect(s.getByText('~/.claude/commands/gsd')).toHaveTextContent('~/.claude/commands/gsd 4');
    expect(s.getByText('~/.claude/get-shit-done/workflows')).toHaveTextContent('2 par @');
    expect(s.getByText('.planning/STATE.md')).toBeInTheDocument();
    // A file gone is not said.
    expect(s.queryByText('.planning/GONE.md')).toBeNull();
    // Found: GSD, and a command alone.
    const gsd = s.getByRole('button', { name: 'Corriger les étapes de GSD' });
    expect(gsd).toHaveTextContent('4 étapes · 6 portes · 4 sous-agents');
    expect(gsd).toHaveTextContent('lu par les règles');
    expect(s.getByText('/commit').closest('.rrow')).toHaveTextContent('une commande seule, sans suitepas un workflow');
    await userEvent.click(s.getByRole('button', { name: 'Relire' }));
    expect(backend.called('refresh_workflows')).toHaveLength(1);
    expect(await s.findByText('Lu par les règles à 10:13')).toBeInTheDocument();
  });

  const plan = {
    workflows: 1,
    files: 6,
    harness: 'claude',
    model: 'Claude Haiku',
    account: 'Principal',
    accountId: 'principal',
    inputTokens: 5200,
    outputTokens: 700,
    cost: 0.0087,
    private: false,
  };

  it('says the model, the account and the cost before « Affiner… », and asks nothing until « Lancer »', async () => {
    const backend = open({}, gsdScan(), {
      workflow_refine_plan: () => plan,
      refine_workflows: () => ({
        scan: gsdScan(),
        refined: [{ id: 'g', name: 'GSD', added: ['Revue'], described: 3, transitions: 1 }],
        rejected: ['GSD : l’étape « Fantôme » n’a pas de fichier connu, refusée.'],
      }),
    });
    const s = within(section());
    await userEvent.click(await s.findByRole('button', { name: 'Affiner…' }));
    const said = await s.findByTestId('refine-plan');
    expect(said).toHaveTextContent('Modèle : Claude Haiku · compte : Principal');
    expect(said).toHaveTextContent(/jetons envoyés, .*0,009.* au tarif public/);
    expect(said).toHaveTextContent('1 workflow sur 6 fichiers');
    expect(backend.called('refine_workflows')).toHaveLength(0);
    await userEvent.click(within(said).getByRole('button', { name: 'Lancer' }));
    expect(backend.called('refine_workflows')).toHaveLength(1);
    expect(await s.findByText('Workflows affinés : 1 étape devinée, 3 descriptions, 1 passage.')).toBeInTheDocument();
    // What the rules refused is told.
    expect(s.getByText(/Fantôme.*refusée/)).toBeInTheDocument();
    expect(s.queryByTestId('refine-plan')).toBeNull();
  });

  it('says it for a private project, and lets « Annuler » ask nothing', async () => {
    const backend = open({}, gsdScan(), { workflow_refine_plan: () => ({ ...plan, private: true, cost: null }) });
    const s = within(section());
    await userEvent.click(await s.findByRole('button', { name: 'Affiner…' }));
    const said = await s.findByTestId('refine-plan');
    expect(said).toHaveTextContent('Projet privé : un agent du projet, sur un compte qui lui est réservé.');
    expect(said).toHaveTextContent('coût non estimé');
    await userEvent.click(within(said).getByRole('button', { name: 'Annuler' }));
    expect(s.queryByTestId('refine-plan')).toBeNull();
    expect(backend.called('refine_workflows')).toHaveLength(0);
  });

  it('says why when there is no account to ask, or when the agent fails', async () => {
    let planned = false;
    open({}, gsdScan(), {
      workflow_refine_plan: () => {
        if (planned) return plan;
        planned = true;
        throw new Error('Aucun compte ne sert ce projet.');
      },
      refine_workflows: () => {
        throw new Error('L’agent n’a pas répondu dans le schéma attendu : rien n’a changé.');
      },
    });
    const s = within(section());
    await userEvent.click(await s.findByRole('button', { name: 'Affiner…' }));
    expect(await s.findByText(/Affiner a échoué : .*Aucun compte/)).toBeInTheDocument();
    await userEvent.click(s.getByRole('button', { name: 'Affiner…' }));
    await userEvent.click(await s.findByRole('button', { name: 'Lancer' }));
    expect(await s.findByText(/rien n’a changé/)).toBeInTheDocument();
  });

  it('turns the reading again by itself off', async () => {
    const backend = open();
    await userEvent.click(within(section()).getByRole('switch', { name: 'Relire quand les fichiers changent' }));
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ workflowsAutoRead: false });
  });

  it('renames, merges, removes, reorders and adds steps, the corrections saved with the project', async () => {
    const backend = open();
    const s = within(section());
    const name = (n: number) => s.getByRole('textbox', { name: `Nom de l’étape ${n}` });
    await userEvent.clear(await s.findByRole('textbox', { name: 'Nom de l’étape 1' }));
    await userEvent.type(name(1), 'Mise au point');
    expect(s.getByRole('button', { name: 'Corriger les étapes de GSD' })).toHaveTextContent('corrigé');
    // Planifier and Exécuter as one.
    await userEvent.click(s.getByRole('button', { name: 'Fusionner l’étape 2 avec la suivante' }));
    expect(s.getAllByRole('textbox', { name: /^Nom de l’étape/ })).toHaveLength(3);
    // Vérifier first, from the keyboard.
    name(3).focus();
    await userEvent.keyboard('{Alt>}{ArrowUp}{ArrowUp}{/Alt}');
    expect(name(1)).toHaveValue('gsd:verify-work');
    expect(name(1)).toHaveFocus();
    // The way back said with the names as corrected.
    expect(section().querySelector('.wf-er.loop')).toHaveTextContent('gsd:verify-work revient à gsd:plan-phase');
    // A step added, told by a file written.
    await userEvent.click(s.getByRole('button', { name: 'Ajouter une étape' }));
    expect(name(4)).toHaveFocus();
    await userEvent.type(name(4), 'Livrer');
    await userEvent.selectOptions(s.getByRole('combobox', { name: 'Ce qui signale l’étape 4' }), 'Fichier écrit');
    await userEvent.type(s.getByRole('textbox', { name: 'Valeur de ce qui signale l’étape 4' }), 'SHIPPED.md');
    // And one removed.
    await userEvent.click(s.getByRole('button', { name: 'Retirer l’étape 2' }));
    await save();
    expect(sentFixes(backend)).toEqual([
      {
        workflow: '|claude|gsd',
        steps: [
          { name: 'gsd:verify-work', from: ['gsd:verify-work'] },
          { name: 'gsd:plan-phase', from: ['gsd:plan-phase', 'gsd:execute-phase'] },
          { name: 'Livrer', signals: [{ file: 'SHIPPED.md' }] },
        ],
        removed: ['gsd:discuss-phase'],
      },
    ]);
  });

  it('moves a step dragged by its number', async () => {
    const backend = open();
    const s = within(section());
    await s.findByRole('textbox', { name: 'Nom de l’étape 1' });
    const rows = section().querySelectorAll('.wf-er:not(.loop)');
    await fireEvent.dragStart(rows[3].querySelector('.no')!);
    await fireEvent.dragOver(rows[0]);
    await fireEvent.drop(rows[0]);
    expect(s.getByRole('textbox', { name: 'Nom de l’étape 1' })).toHaveValue('gsd:verify-work');
    await save();
    expect(sentFixes(backend)[0].steps?.map((x) => x.from?.[0])).toEqual([
      'gsd:verify-work',
      'gsd:discuss-phase',
      'gsd:plan-phase',
      'gsd:execute-phase',
    ]);
  });

  it('keeps or removes a step the reading guessed', async () => {
    const guessed = gsdWorkflow({
      steps: [...gsdWorkflow().steps, { ...gsdWorkflow().steps[0], name: 'gsd:ship', command: '/gsd:ship', guessed: true }],
    });
    const backend = open({}, gsdScan({ workflows: [guessed] }));
    const s = within(section());
    await s.findByRole('textbox', { name: 'Nom de l’étape 5' });
    const row = section().querySelectorAll('.wf-er')[4] as HTMLElement;
    expect(row).toHaveClass('guess');
    expect(row).toHaveTextContent('deviné · /gsd:ship');
    await userEvent.click(within(row).getByRole('button', { name: 'Garder' }));
    expect(section().querySelectorAll('.wf-er')[4]).not.toHaveClass('guess');
    await save();
    expect(sentFixes(backend)[0].steps?.at(-1)).toEqual({ name: 'gsd:ship', from: ['gsd:ship'] });
  });

  it('says a correction that no longer matches anything', async () => {
    open({ workflowFixes: [{ workflow: '|claude|gsd', steps: [{ name: 'Ancienne', from: ['gsd:gone'] }] }] });
    expect(await within(section()).findByText('ne correspond plus à rien : la lecture n’a plus gsd:gone')).toBeInTheDocument();
  });

  it('sets a workflow apart, then brings it back', async () => {
    const backend = open();
    const s = within(section());
    await userEvent.click(await s.findByRole('button', { name: 'Ce n’est pas un workflow' }));
    expect(s.queryByText('Étapes')).toBeNull();
    expect(s.getByText('GSD').closest('.rrow')).toHaveTextContent('pas un workflow');
    await save();
    expect(sentFixes(backend)).toEqual([{ workflow: '|claude|gsd', dismissed: true }]);
    await userEvent.click(s.getByRole('button', { name: 'Rétablir' }));
    expect(s.getByText('Étapes')).toBeInTheDocument();
  });

  it('keeps the workflows in the repository by default, or on this machine when asked', async () => {
    const backend = open();
    const s = within(section());
    expect(await s.findByRole('radio', { name: 'Dans le dépôt' })).toBeChecked();
    expect(s.getByText(/^Les workflows tirés de la config du dépôt/)).toHaveTextContent('.escouade/workflows/');
    expect(s.getByText(/ne les commite jamais/)).toBeInTheDocument();
    await userEvent.click(s.getByRole('radio', { name: 'Sur cette machine' }));
    expect(s.getByRole('radio', { name: 'Sur cette machine' })).toBeChecked();
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ workflowsInRepo: false });
  });

  it('says a file that cannot be read, one updated by the reading, and one of the team’s alone that it can remove', async () => {
    const scan = gsdScan({
      files: [{ id: '|claude|gsd', path: '.escouade/workflows/claude-gsd.yml', local: false, inRepo: true, updated: null }],
      unreadable: [{ path: '.escouade/workflows/broken.yml', why: 'version manque' }],
    });
    const backend = open({}, scan);
    const s = within(section());
    expect(await s.findByText('.escouade/workflows/broken.yml illisible : version manque')).toBeInTheDocument();
    expect(s.getByRole('button', { name: 'Corriger les étapes de GSD' })).toHaveTextContent('dans le dépôt');
    expect(s.getByRole('button', { name: 'Corriger les étapes de GSD' })).toHaveTextContent('du dépôt');
    expect(
      s.getByText('Cette machine ne le lit pas : il vient de .escouade/workflows/claude-gsd.yml, du dépôt.', { exact: false }),
    ).toBeInTheDocument();
    await userEvent.click(s.getByRole('button', { name: 'Retirer du dépôt' }));
    expect(backend.called('remove_workflow_file')).toEqual([expect.objectContaining({ args: { id: 'p1', workflow: '|claude|gsd' } })]);
  });

  it('says what the reading changed of a workflow', async () => {
    open(
      {},
      gsdScan({
        files: [
          {
            id: '|claude|gsd',
            path: '~/.escouade/projects/p1/workflows/claude-gsd.yml',
            local: true,
            inRepo: false,
            updated: 'étapes ajoutées : gsd:ship',
          },
        ],
      }),
    );
    expect(await within(section()).findByText('mis à jour par la relecture')).toBeInTheDocument();
    // Of the user's own folders: on this machine, outside the repository.
    expect(within(section()).getByRole('button', { name: 'Corriger les étapes de GSD' })).toHaveTextContent(
      'sur cette machine (hors du dépôt)',
    );
  });

  it('says it in English', async () => {
    setLang('en');
    open();
    const s = within(section());
    expect(await s.findByText(/^Read by the rules at 10:12/)).toBeInTheDocument();
    expect(s.getByRole('button', { name: 'Correct the steps of GSD' })).toHaveTextContent('4 steps · 6 entries · 4 subagents');
    expect(s.getByRole('button', { name: 'Merge step 1 with the next' })).toBeInTheDocument();
    expect(s.getByRole('radio', { name: 'In the repository' })).toBeChecked();
    expect(s.getByRole('button', { name: 'Add a step' })).toBeInTheDocument();
  });
});

describe('Réglages › Projets › Workflow, supprimer et « Pas de workflow » (ESC-309)', () => {
  it('deletes a workflow at once and says the reading will not make it again', async () => {
    const backend = open({}, gsdScan(), {
      set_workflow_removed: () => gsdScan({ workflows: [], removed: [{ id: '|claude|gsd', name: 'GSD', changed: false }] }),
    });
    const s = within(section());
    await userEvent.click(await s.findByRole('button', { name: 'Supprimer ce workflow' }));
    expect(backend.called('set_workflow_removed')).toEqual([
      expect.objectContaining({ args: { id: 'p1', workflow: '|claude|gsd', removed: true } }),
    ]);
    const row = await s.findByTestId('wf-removed');
    expect(row).toHaveTextContent('GSDsupprimé, ne sera pas recréé');
    expect(s.getByText('Supprimés')).toBeInTheDocument();
    expect(s.queryByRole('button', { name: 'Corriger les étapes de GSD' })).toBeNull();
    expect(app.toasts.at(-1)?.text).toBe(
      'GSD supprimé : la relecture ne le recréera pas. « Rétablir » sous « Supprimés » pour le défaire.',
    );
  });

  it('proposes again a workflow deleted whose finding changed, kept deleted or taken back', async () => {
    const removed = (changed: boolean) => gsdScan({ workflows: [], removed: [{ id: '|claude|gsd', name: 'GSD', changed }] });
    const backend = open({}, removed(true), {
      set_workflow_removed: ({ removed: r }) => (r ? removed(false) : gsdScan()),
    });
    const s = within(section());
    expect(await s.findByTestId('wf-removed')).toHaveTextContent('supprimé, mais ce qui l’a fait trouver a changé : le reprendre ?');
    await userEvent.click(s.getByRole('button', { name: 'Laisser supprimé' }));
    expect(await s.findByText('supprimé, ne sera pas recréé')).toBeInTheDocument();
    await userEvent.click(s.getByRole('button', { name: 'Rétablir' }));
    expect(backend.called('set_workflow_removed').map((c) => c.args.removed)).toEqual([true, false]);
    expect(await s.findByRole('button', { name: 'Corriger les étapes de GSD' })).toBeInTheDocument();
    expect(s.queryByTestId('wf-removed')).toBeNull();
  });

  it('sets « Pas de workflow pour ce projet », says what it does and how to undo it, and saves it', async () => {
    const backend = open();
    const s = within(section());
    expect(await s.findByRole('radio', { name: 'Lire les workflows' })).toBeChecked();
    await userEvent.click(s.getByRole('radio', { name: 'Pas de workflow pour ce projet' }));
    const note = s.getByTestId('workflows-off');
    expect(note).toHaveTextContent('Escouade ne lit ni n’écrit ses workflows');
    expect(note).toHaveTextContent('.escouade/workflows/ reste tel quel');
    expect(note).toHaveTextContent('Pour le défaire, choisis « Lire les workflows » et enregistre.');
    // Nothing of the workflows is offered while it is chosen.
    expect(s.queryByRole('button', { name: 'Relire' })).toBeNull();
    expect(s.queryByRole('button', { name: 'Corriger les étapes de GSD' })).toBeNull();
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ workflowsEnabled: false });
  });

  it('says a project without workflows as such, and reads them again once undone', async () => {
    const backend = open({ workflowsEnabled: false }, { ...gsdScan({ workflows: [], sources: [] }), off: true });
    const s = within(section());
    expect(await s.findByRole('radio', { name: 'Pas de workflow pour ce projet' })).toBeChecked();
    expect(s.getByTestId('workflows-off')).toBeInTheDocument();
    await userEvent.click(s.getByRole('radio', { name: 'Lire les workflows' }));
    expect(s.getByTestId('workflows-off')).toHaveTextContent('Enregistre pour lire à nouveau les workflows du projet.');
    await save();
    expect(backend.called('update_project')[0].args.project).toMatchObject({ workflowsEnabled: true });
  });

  it('says it in English', async () => {
    setLang('en');
    open({ workflowsEnabled: false }, { ...gsdScan({ workflows: [] }), off: true });
    const s = within(section());
    expect(await s.findByRole('radio', { name: 'No workflow for this project' })).toBeChecked();
    expect(s.getByTestId('workflows-off')).toHaveTextContent(
      'No workflow for this project: Escouade neither reads nor writes its workflows and follows no agent in them; .escouade/workflows/ is left as it is. To undo it, choose “Read the workflows” and save.',
    );
    setLang('fr');
  });
});

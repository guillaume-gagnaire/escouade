import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { tick } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { inbox } from '../../lib/inbox/inbox.svelte';
import { menu } from '../../lib/menu.svelte';
import { app } from '../../lib/state.svelte';
import type { WindowAsk } from '../../lib/types';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { askFile, inboxFileAnswer, renders, windowAsk } from '../../test/inbox';
import ContextMenu from '../ContextMenu.svelte';
import InboxQueue from './InboxQueue.svelte';
import InboxScreen from './InboxScreen.svelte';

// The PDF read in place: pdf.js does not run under jsdom; the reader is tried against a document of three pages.
const pdf = vi.hoisted(() => ({ draws: [] as number[], finds: [] as string[], password: false }));
vi.mock('../../lib/previews/pdf', () => ({
  PdfPasswordError: class extends Error {},
  openPdf: async () => {
    if (pdf.password) {
      const { PdfPasswordError } = await import('../../lib/previews/pdf');
      throw new PdfPasswordError();
    }
    return {
      pages: 3,
      draw: async (n: number, c: HTMLCanvasElement) => {
        if (c.classList.contains('page')) pdf.draws.push(n);
      },
      find: async (q: string) => {
        pdf.finds.push(q);
        return q === 'coûte' ? [3] : [];
      },
      dispose() {},
    };
  },
}));

/** The box with `asks`, shown: its queue and its panel. */
async function show(asks: WindowAsk[], handlers: Record<string, (a: any) => unknown> = {}) {
  const ipc = fakeBackend({
    inbox_list: () => ({ enabled: true, asks, open: asks.length }),
    inbox_file: inboxFileAnswer,
    inbox_answer: (a: { id: string; answer: unknown }) => ({
      ...asks.find((x) => x.id === a.id)!,
      state: 'answered',
      answer: { ...(a.answer as object), at: Date.now(), sendAt: Date.now() + 10_000, sentAt: null, by: null, byName: null },
    }),
    inbox_snooze: (a: { id: string; until: number }) => ({ ...asks.find((x) => x.id === a.id)!, state: 'snoozed', snoozedUntil: a.until }),
    inbox_reopen: (a: { id: string }) => ({ ...asks.find((x) => x.id === a.id)!, state: 'open', answer: null }),
    inbox_delegate: (a: { id: string; agent: string }) => ({
      ...asks.find((x) => x.id === a.id)!,
      delegatedTo: a.agent,
      delegatedName: 'chef-atlas',
    }),
    ...handlers,
  });
  app.settings.mcpEnabled = true;
  app.ui.view = 'inbox';
  await inbox.load();
  render(InboxQueue);
  render(InboxScreen);
  render(ContextMenu);
  await tick();
  return ipc;
}

const pane = () => screen.getByTestId('ask-pane');
const answered = (ipc: ReturnType<typeof fakeBackend>) => ipc.called('inbox_answer').map((c) => c.args);

describe('the box « À toi »', () => {
  beforeEach(() => {
    resetApp({ agents: [agent({ id: 'a1', name: 'design-tableau' })] });
    menu.close();
    pdf.draws = [];
    pdf.finds = [];
    pdf.password = false;
  });

  it('lists who asks, what, since when, its files and the renders of a choice, and opens the first that waits', async () => {
    await show([
      windowAsk({ files: renders() }),
      windowAsk({
        id: 'a-2',
        kind: 'confirm',
        title: 'Publier atlas-web 1.4.0 ?',
        urgency: 'today',
        author: { agent: null, name: 'chef-atlas', chef: true },
      }),
    ]);
    const queue = screen.getByTestId('inbox-queue');
    const rows = within(queue).getAllByTestId('inbox-row');
    // « Aujourd'hui »: the urgent one first.
    expect(rows[0]).toHaveTextContent('Publier atlas-web 1.4.0 ?');
    expect(rows[0]).toHaveTextContent('chef-atlas');
    expect(rows[1]).toHaveTextContent('Choisis la direction du tableau de bord');
    expect(rows[1]).toHaveTextContent('design-tableau');
    expect(rows[1]).toHaveTextContent('6 images');
    expect(rows[1]).toHaveTextContent('40 min');
    await waitFor(() => expect(within(rows[1]).getAllByRole('img')).toHaveLength(3));
    expect(within(queue).getByTestId('inbox-scoped-count')).toHaveTextContent('2');
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 sur 2');
    expect(pane()).toHaveTextContent('Publier atlas-web 1.4.0 ?');
  });

  it('shows the box empty, saying what comes there', async () => {
    await show([]);
    expect(screen.getByTestId('inbox-empty')).toHaveTextContent('Rien ne t’attend');
    expect(screen.getByTestId('inbox-empty')).toHaveTextContent('Un choix');
    expect(screen.getByTestId('inbox-empty')).toHaveTextContent('Une tâche');
  });

  it('filters the box to the project on screen', async () => {
    await show([windowAsk(), windowAsk({ id: 'a-2', projectId: 'p2', title: 'Ailleurs' })]);
    expect(screen.getAllByTestId('inbox-row')).toHaveLength(2);
    await userEvent.click(screen.getByRole('radio', { name: 'Ce projet' }));
    expect(screen.getAllByTestId('inbox-row')).toHaveLength(1);
    expect(screen.getByTestId('inbox-scoped-count')).toHaveTextContent('1');
  });

  describe('a choice', () => {
    it('sets the recommendation forward without choosing it: Entrée sends the option in focus, which moves', async () => {
      const ipc = await show([windowAsk({ files: renders() })]);
      expect(pane()).toHaveTextContent('Recommandé : B · Cartes empilées.');
      expect(pane()).toHaveTextContent('Ta réponse débloque : DEM-13 et DEM-14');
      expect(screen.getByTestId('ask-choose')).toHaveTextContent('Choisir B');
      // The seal of the recommendation, on its plate.
      expect(screen.getAllByRole('img', { name: 'Recommandé par l’agent' })).toHaveLength(1);
      await userEvent.keyboard('1');
      expect(screen.getByTestId('ask-choose')).toHaveTextContent('Choisir A');
      await userEvent.type(screen.getByTestId('ask-note'), 'garde la courbe');
      await userEvent.keyboard('{Enter}');
      await waitFor(() => expect(answered(ipc)).toHaveLength(1));
      expect(answered(ipc)[0].answer).toEqual({ options: ['a'], note: 'garde la courbe', via: 'keyboard' });
      // Its 10 seconds, and « Annuler » takes it back.
      expect(await screen.findByTestId('inbox-sending')).toHaveTextContent('part dans 10 s');
      await userEvent.click(screen.getByTestId('inbox-undo'));
      await waitFor(() => expect(ipc.called('inbox_reopen')).toHaveLength(1));
    });

    it('compares the renders full size: ← →, 1 2 3, Espace alone, Échap, and chooses from there', async () => {
      const ipc = await show([windowAsk({ files: renders() })]);
      await userEvent.click(screen.getByTestId('ask-compare'));
      const dialog = screen.getByRole('dialog', { name: 'Comparer les rendus' });
      expect(within(dialog).getAllByTestId('ask-plate')).toHaveLength(3);
      // Dark and light joined: « Sombre · Clair · Les deux ».
      await userEvent.click(within(dialog).getByRole('radio', { name: 'Les deux' }));
      await waitFor(() => expect(within(dialog).getAllByTestId('ask-plate')[0].querySelectorAll('img')).toHaveLength(2));
      fireEvent.keyDown(dialog, { key: 'ArrowRight' });
      await tick();
      expect(within(dialog).getByTestId('compare-choose')).toHaveTextContent('Choisir C');
      fireEvent.keyDown(dialog, { key: '1' });
      await tick();
      expect(within(dialog).getByTestId('compare-choose')).toHaveTextContent('Choisir A');
      fireEvent.keyDown(dialog, { key: 'Escape' });
      await tick();
      expect(screen.queryByRole('dialog', { name: 'Comparer les rendus' })).toBeNull();
      // Counted in the statistics, its name only.
      expect(ipc.called('inbox_track').map((c) => c.args)).toEqual([{ event: 'compare', from: null, scope: null, kind: null }]);
      await userEvent.click(screen.getByTestId('ask-compare'));
      await userEvent.click(screen.getByTestId('compare-choose'));
      await waitFor(() => expect(answered(ipc)[0]?.answer).toMatchObject({ options: ['a'] }));
    });

    it('chooses several when the agent asks for several', async () => {
      const ipc = await show([windowAsk({ multi: true })]);
      await userEvent.keyboard('1');
      await userEvent.keyboard('3');
      await userEvent.click(screen.getByTestId('ask-choose'));
      await waitFor(() => expect(answered(ipc)[0]?.answer).toMatchObject({ options: ['a', 'c'], via: 'click' }));
    });
  });

  it('a go-ahead: its labels, « Non », and what it says of the gate', async () => {
    const ipc = await show([
      windowAsk({
        kind: 'confirm',
        options: [],
        recommend: null,
        gate: true,
        yesLabel: 'Oui, publie',
        noLabel: 'Non, pas encore',
        author: { agent: 'a1', name: 'chef-atlas', chef: true },
      }),
    ]);
    expect(screen.getByTestId('ask-yes')).toHaveTextContent('Oui, publie');
    expect(pane()).toHaveTextContent('Un feu vert : toi seul y réponds.');
    await userEvent.click(screen.getByTestId('ask-no'));
    await waitFor(() => expect(answered(ipc)[0]?.answer).toMatchObject({ yes: false }));
  });

  it('an answer: the text the agent proposes, changed, sent by Ctrl Entrée', async () => {
    const ipc = await show([
      windowAsk({ kind: 'answer', options: [], recommend: { option: null, text: 'Bienvenue au club', why: 'Court.' } }),
    ]);
    const field = screen.getByTestId('ask-text');
    expect(field).toHaveValue('Bienvenue au club');
    await userEvent.type(field, ' !{Enter}');
    expect(answered(ipc)).toHaveLength(0);
    await userEvent.type(field, '{Control>}{Enter}{/Control}');
    await waitFor(() => expect((answered(ipc)[0]?.answer as { text: string }).text).toBe('Bienvenue au club !'));
  });

  it('a task: its steps ticked, « C’est fait » or « Je ne le ferai pas »', async () => {
    const ipc = await show([windowAsk({ kind: 'task', options: [], recommend: null, steps: ['Lance l’aperçu.', 'Scanne le QR code.'] })]);
    const steps = screen.getAllByTestId('ask-step');
    await userEvent.click(steps[0]);
    await userEvent.click(screen.getByTestId('ask-done'));
    await waitFor(() => expect(answered(ipc)[0]?.answer).toMatchObject({ done: true, steps: [true, false] }));
  });

  it('J and K go from one request to the next, L puts one off until a time it offers', async () => {
    const now = Date.now();
    const ipc = await show([
      windowAsk({ id: 'a-1', createdAt: now - 3000 }),
      windowAsk({ id: 'a-2', title: 'La deuxième', createdAt: now - 2000 }),
    ]);
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 sur 2');
    await userEvent.keyboard('j');
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('2 sur 2');
    expect(pane()).toHaveTextContent('La deuxième');
    await userEvent.keyboard('k');
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 sur 2');
    await userEvent.keyboard('l');
    expect(menu.open?.items.map((i) => i.label)).toContain('Demain, 9:00');
    menu.open!.items.find((i) => i.label === 'Demain, 9:00')!.onClick!();
    await waitFor(() => expect(ipc.called('inbox_snooze')).toHaveLength(1));
    const until = new Date(ipc.called('inbox_snooze')[0].args.until);
    expect(until.getHours()).toBe(9);
    // Put off, it leaves the queue for « Plus tard », and the next one opens.
    await waitFor(() => expect(pane()).toHaveTextContent('La deuxième'));
  });

  it('leaves a request to the squad lead when the setting allows it, never a go-ahead', async () => {
    const ipc = await show([windowAsk({ delegates: [['c1', 'chef-atlas']] })]);
    await userEvent.click(screen.getByTestId('ask-delegate'));
    await waitFor(() => expect(ipc.called('inbox_delegate')[0]?.args).toEqual({ id: 'a-1', agent: 'c1' }));
    expect(await screen.findByText(/Laissée à chef-atlas/)).toBeInTheDocument();
    resetApp({ agents: [agent({ id: 'a1' })] });
    document.body.innerHTML = '';
    await show([windowAsk({ kind: 'confirm', gate: true, options: [], delegates: [['c1', 'chef-atlas']] })]);
    expect(screen.queryByTestId('ask-delegate')).toBeNull();
  });

  it('goes to the agent that asks', async () => {
    await show([windowAsk()]);
    await userEvent.click(screen.getByTestId('inbox-to-agent'));
    expect(app.ui.view).toBe('project');
    expect(app.ui.selectedAgent.p1).toBe('a1');
  });

  describe('its files', () => {
    it('reads a PDF in place, page by page, and looks for a word in it', async () => {
      await show([
        windowAsk({ files: [askFile({ file: 'p1/etude.pdf', name: 'etude.pdf', kind: 'pdf', mediaType: 'application/pdf', size: 13 })] }),
      ]);
      const reader = await screen.findByTestId('pdf-reader');
      await waitFor(() => expect(within(reader).getByTestId('pdf-canvas')).toBeInTheDocument());
      expect(reader).toHaveTextContent('sur 3');
      await userEvent.click(within(reader).getByTestId('pdf-next'));
      expect(within(reader).getByTestId('pdf-page')).toHaveValue('2');
      await userEvent.click(within(reader).getByRole('button', { name: 'Page 3' }));
      expect(within(reader).getByTestId('pdf-canvas').dataset.page).toBe('3');
      await userEvent.type(within(reader).getByRole('searchbox', { name: 'Chercher dans le PDF' }), 'rien{Enter}');
      expect(await within(reader).findByText('Rien trouvé')).toBeInTheDocument();
      // Full screen, then back by Échap.
      await userEvent.click(within(reader).getByTestId('pdf-full'));
      expect(screen.getByRole('dialog', { name: 'Lecture de etude.pdf' })).toBeInTheDocument();
      fireEvent.keyDown(screen.getByRole('dialog', { name: 'Lecture de etude.pdf' }), { key: 'Escape' });
      await tick();
      expect(screen.queryByRole('dialog', { name: 'Lecture de etude.pdf' })).toBeNull();
    });

    it('says a PDF protected by a password does not show', async () => {
      pdf.password = true;
      await show([windowAsk({ files: [askFile({ file: 'p1/x.pdf', name: 'x.pdf', kind: 'pdf', size: 13 })] })]);
      expect(await screen.findByRole('alert')).toHaveTextContent('protégé par un mot de passe');
    });

    it('opens an image in the viewer, draws a Markdown file, offers « Ouvrir » and « Enregistrer sous… » for the rest', async () => {
      const ipc = await show(
        [
          windowAsk({
            kind: 'confirm',
            options: [],
            files: [
              askFile({ file: 'i1/couts.png', name: 'couts.png' }),
              askFile({ file: 'm1/notes.md', name: 'notes.md', kind: 'markdown', mediaType: 'text/markdown', size: 20 }),
              askFile({ file: 'w1/etude.docx', name: 'etude.docx', kind: 'word', mediaType: 'application/zip', size: 48_000 }),
              askFile({ file: 'x1/setup.exe', name: 'setup.exe', kind: 'other', mediaType: 'application/octet-stream', size: 1000 }),
            ],
          }),
        ],
        { inbox_save_file: () => true },
      );
      const files = screen.getByTestId('ask-files');
      // The Word document is a file of its own (no preview on 2.2): its name, its size, « Ouvrir », « Enregistrer sous… ».
      const word = within(files).getByText('etude.docx').closest('li')!;
      expect(word).toHaveTextContent('47 Ko');
      await userEvent.click(within(word).getByTestId('ask-open'));
      expect(ipc.called('inbox_open_file')[0].args).toEqual({ id: 'a-1', file: 'w1/etude.docx', reveal: false });
      await userEvent.click(within(word).getByTestId('ask-save'));
      await waitFor(() => expect(ipc.called('inbox_save_file')[0]?.args).toEqual({ id: 'a-1', file: 'w1/etude.docx' }));
      // A program is never opened: shown in its folder.
      const exe = within(files).getByText('setup.exe').closest('li')!;
      expect(within(exe).queryByTestId('ask-open')).toBeNull();
      await userEvent.click(within(exe).getByRole('button', { name: 'Afficher dans le dossier' }));
      expect(ipc.called('inbox_open_file')[1].args.reveal).toBe(true);
      // The Markdown, drawn.
      expect(await screen.findByRole('heading', { name: 'Notes' })).toBeInTheDocument();
      // The image, in the viewer.
      await userEvent.click(within(files).getByTestId('ask-image'));
      expect(await screen.findByRole('dialog')).toBeInTheDocument();
      expect(ipc.called('inbox_track').map((c) => c.args.kind)).toEqual(['word', 'image']);
    });
  });

  describe('the trace', () => {
    it('shows what was answered recently, by whom and when, read or not, and takes back an answer not read', async () => {
      const at = Date.now() - 60_000;
      const ipc = await show([
        windowAsk({
          id: 'a-9',
          state: 'answered',
          answer: {
            options: ['c'],
            yes: null,
            text: null,
            done: null,
            steps: [],
            note: null,
            by: null,
            byName: null,
            via: 'click',
            at,
            sendAt: null,
            sentAt: at,
          },
        }),
        windowAsk({
          id: 'a-8',
          title: 'Nom de la commande',
          kind: 'answer',
          state: 'read',
          answer: {
            options: [],
            yes: null,
            text: 'export:csv',
            done: null,
            steps: [],
            note: null,
            by: 'c1',
            byName: 'chef-atlas',
            via: 'delegated',
            at,
            sendAt: null,
            sentAt: at,
          },
        }),
      ]);
      const recent = screen.getAllByTestId('inbox-recent-row');
      expect(recent[0]).toHaveTextContent('C · Tableau dense · toi');
      expect(recent[1]).toHaveTextContent('« export:csv » · chef-atlas, délégué');
      expect(recent[1]).toHaveTextContent('lue par design-tableau');
      await userEvent.click(recent[0]);
      expect(screen.getByTestId('ask-trace')).toHaveTextContent('Envoyée à design-tableau');
      await userEvent.click(screen.getByTestId('ask-reopen'));
      await waitFor(() => expect(ipc.called('inbox_reopen')[0]?.args).toEqual({ id: 'a-9' }));
      await userEvent.click(screen.getAllByTestId('inbox-recent-row').find((r) => r.textContent?.includes('Nom de la commande'))!);
      expect(screen.getByTestId('ask-trace')).toHaveTextContent('Lue par design-tableau');
      // Read: « Changer d'avis… » writes a message to its agent.
      await userEvent.click(screen.getByTestId('ask-change-mind'));
      expect(app.ui.selectedAgent.p1).toBe('a1');
    });
  });

  it('is written in English too', async () => {
    setLang('en');
    await show([windowAsk({ files: renders() })]);
    expect(screen.getByTestId('ask-choose')).toHaveTextContent('Choose B');
    expect(screen.getByTestId('inbox-nth')).toHaveTextContent('1 of 1');
    expect(pane()).toHaveTextContent('Your answer unblocks:');
    setLang('fr');
  });
});

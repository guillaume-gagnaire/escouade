import { createEvent, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fileOrigins } from '../lib/file-origin';
import { draftGallery } from '../lib/gallery';
import { app } from '../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import Composer from './Composer.svelte';

// ESC-555: an image attached to the message, seen full size in the conversation's viewer before it is sent.
vi.mock('../lib/file-origin', () => ({ fileOrigins: vi.fn() }));
beforeEach(() => {
  resetApp();
  vi.mocked(fileOrigins).mockImplementation(async (files) => files.map(() => null));
});

const png = (name: string, byte: number) => new File([new Uint8Array([137, 80, 78, 71, byte])], name, { type: 'image/png' });
const text = (name: string) => new File(['hello'], name, { type: 'text/plain' });

async function setup(files: File[]) {
  const a = agent({ id: `img${Math.random()}` });
  resetApp({ agents: [a] });
  const backend = fakeBackend({ get_conversation: () => [] });
  render(Composer, { agent: app.agents[a.id] });
  const textarea = screen.getByRole('textbox') as HTMLTextAreaElement;
  await userEvent.type(textarea, 'Regarde ça');
  fireEvent(textarea, createEvent.paste(textarea, { clipboardData: { files, types: ['Files'] } }));
  for (const f of files.filter((f) => f.type.startsWith('image/'))) await screen.findByRole('button', { name: `Agrandir ${f.name}` });
  return { backend, textarea };
}

const viewer = () => screen.queryByRole('dialog', { name: /image/i });

describe('Composer: an attached image seen full size', () => {
  it('opens the viewer on the image clicked, with the other images of the message in the field’s order', async () => {
    const { backend } = await setup([png('un.png', 1), text('notes.txt'), png('deux.png', 2), png('trois.png', 3)]);
    await userEvent.click(screen.getByRole('button', { name: 'Agrandir deux.png' }));
    const dialog = await screen.findByRole('dialog');
    // The image clicked first, among the three images (the text file is not one of them).
    expect(within(dialog).getByText('2 sur 3')).toBeInTheDocument();
    expect(dialog.querySelector('.bar .name')).toHaveTextContent('deux.png');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Image suivante' }));
    expect(dialog.querySelector('.bar .name')).toHaveTextContent('trois.png');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Image précédente' }));
    await userEvent.click(within(dialog).getByRole('button', { name: 'Image précédente' }));
    expect(dialog.querySelector('.bar .name')).toHaveTextContent('un.png');
    // Not sent yet: nothing to open nor to show in its folder; copying and saving stay.
    expect(within(dialog).queryByRole('button', { name: /^Ouvrir/ })).toBeNull();
    expect(within(dialog).queryByRole('button', { name: /dossier/ })).toBeNull();
    expect(within(dialog).getByRole('button', { name: 'Copier l’image' })).toBeInTheDocument();
    // Nothing more read: the viewer shows the field's own `data:` URLs.
    expect(backend.calls.filter((c) => c.cmd !== 'get_conversation' && c.cmd !== 'attach_discard').map((c) => c.cmd)).toEqual([]);
  });

  it('opens by Entrée and Espace, and gives the focus back to the thumbnail with the text untouched', async () => {
    const { textarea } = await setup([png('un.png', 1), png('deux.png', 2)]);
    const thumb = screen.getByRole('button', { name: 'Agrandir un.png' });
    thumb.focus();
    await userEvent.keyboard('{Enter}');
    expect(await screen.findByRole('dialog')).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(thumb).toHaveFocus();
    expect(textarea.value).toBe('Regarde ça');
    await userEvent.keyboard(' ');
    const dialog = await screen.findByRole('dialog');
    expect(dialog.querySelector('.bar .name')).toHaveTextContent('un.png');
    await userEvent.click(within(dialog).getByRole('button', { name: 'Fermer (Échap)' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(thumb).toHaveFocus();
    expect(textarea.value).toBe('Regarde ça');
  });

  it('takes the image away by its « × » without opening the viewer', async () => {
    await setup([png('un.png', 1), png('deux.png', 2)]);
    await userEvent.click(screen.getByRole('button', { name: 'Retirer un.png' }));
    expect(screen.queryByRole('button', { name: 'Agrandir un.png' })).toBeNull();
    expect(screen.queryByRole('dialog')).toBeNull();
    // The one left opens alone, with no previous nor next.
    await userEvent.click(screen.getByRole('button', { name: 'Agrandir deux.png' }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).queryByRole('button', { name: 'Image suivante' })).toBeNull();
  });

  it('says what the thumbnail does in its tip', async () => {
    await setup([png('un.png', 1)]);
    const thumb = screen.getByRole('button', { name: 'Agrandir un.png' });
    expect(thumb).toHaveAttribute('aria-label', 'Agrandir un.png');
    expect(viewer()).toBeNull();
  });
});

describe('draftGallery', () => {
  it('keeps the images read, in their order, from the one asked, with no source', () => {
    const list = [{ name: 'a', url: 'data:a' }, { name: 'b' }, { name: 'c', url: 'data:c' }];
    expect(draftGallery(list, 2)).toEqual({
      images: [
        { name: 'a', url: 'data:a', source: null },
        { name: 'c', url: 'data:c', source: null },
      ],
      index: 1,
    });
  });
});

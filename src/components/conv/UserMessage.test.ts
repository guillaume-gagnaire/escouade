import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import UserMessage from './UserMessage.svelte';

const item = (queued: boolean) => ({ kind: 'user' as const, id: 'u1', text: 'Ajoute aussi les tests', images: 0, ts: 1, queued });

describe('UserMessage', () => {
  it('says a message sent during a turn was passed on to Claude, which takes it at its next step', () => {
    render(UserMessage, { item: item(true) });
    expect(screen.getByText('transmis pendant le tour')).toBeInTheDocument();
    expect(screen.queryByText(/en file/)).not.toBeInTheDocument();
  });

  it('shows nothing more for a message sent between turns', () => {
    render(UserMessage, { item: item(false) });
    expect(screen.queryByText('transmis pendant le tour')).not.toBeInTheDocument();
  });
});

describe('UserMessage with attachments', () => {
  it('shows the images count and each attached file by name', () => {
    render(UserMessage, { item: { ...item(false), images: 2, files: ['rapport.pdf', 'notes.md'] } });
    expect(screen.getByText('🖼 2 images')).toBeInTheDocument();
    expect(screen.getByText('📄 rapport.pdf')).toBeInTheDocument();
    expect(screen.getByText('📄 notes.md')).toBeInTheDocument();
  });

  it('shows a message saved before files could be attached as before', () => {
    render(UserMessage, { item: item(false) });
    expect(screen.queryByText(/📄/)).not.toBeInTheDocument();
  });
});

describe('UserMessage from claude.ai', () => {
  it('says the message was sent from claude.ai or the Claude app', () => {
    render(UserMessage, { item: { ...item(false), origin: 'remote' as const } });
    expect(screen.getByText('depuis claude.ai')).toBeInTheDocument();
  });
});

describe('UserMessage in English', () => {
  it('counts the images with a plural, and words the notes in English', () => {
    setLang('en');
    const { unmount } = render(UserMessage, { item: { ...item(true), images: 1, origin: 'remote' as const } });
    expect(screen.getByText('🖼 1 image')).toBeInTheDocument();
    expect(screen.getByText('sent during the turn')).toHaveAttribute('title', 'Claude takes it into account at its next step');
    expect(screen.getByText('from claude.ai')).toBeInTheDocument();
    unmount();
    render(UserMessage, { item: { ...item(false), images: 2 }, waiting: true });
    expect(screen.getByText('🖼 2 images')).toBeInTheDocument();
    expect(screen.getByText('Waiting for the setup…')).toBeInTheDocument();
  });
});

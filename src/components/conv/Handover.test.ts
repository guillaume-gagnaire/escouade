import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { EngineDecl, HandoverItem, UserItem } from '../../lib/types';
import { BARE_ENGINE, CLAUDE_ENGINE, resetApp } from '../../test/ipc';
import Handover from './Handover.svelte';
import UserMessage from './UserMessage.svelte';

// The change of harness in the conversation (ESC-88): its separator and the handover file, folded.

const CODEX: EngineDecl = { ...BARE_ENGINE, id: 'codex', name: 'Codex', speaker: 'Codex' };
const UNTIL = new Date(2026, 9, 10, 18, 0).getTime();

const sep = (over: Partial<HandoverItem> = {}): HandoverItem => ({
  kind: 'handover',
  id: 'h1',
  ts: 1,
  from: 'claude',
  to: 'codex',
  limited: true,
  until: UNTIL,
  ...over,
});

describe('the change of harness in the conversation', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, CODEX];
    setLang('fr');
  });

  it('draws a hairline on each side, the harness it goes on on in bold', () => {
    render(Handover, { item: sep() });
    expect(screen.getByRole('separator').querySelector('b')).toHaveTextContent(/^Suite sur Codex$/);
  });

  it('says where the agent goes on, why, and until when', () => {
    const { unmount } = render(Handover, { item: sep() });
    expect(screen.getByRole('separator')).toHaveTextContent('Suite sur Codex · quota Claude Code épuisé jusqu’à 18:00');
    unmount();
    const unknown = render(Handover, { item: sep({ until: null }) });
    expect(screen.getByRole('separator')).toHaveTextContent(/^Suite sur Codex · quota Claude Code épuisé$/);
    unknown.unmount();
    render(Handover, { item: sep({ from: 'codex', to: 'claude', limited: false, until: null }) });
    expect(screen.getByRole('separator')).toHaveTextContent('Suite sur Claude Code · l’agent était sur Codex');
  });

  it('says it in English', () => {
    setLang('en');
    render(Handover, { item: sep() });
    expect(screen.getByRole('separator')).toHaveTextContent('Continued on Codex · Claude Code quota spent until 6:00 PM');
  });

  it('says a passage from a teammate: « Passation » with the handover file, « Reprise de la session » with its own (TS-17b)', () => {
    const passed = render(Handover, { item: sep({ from: 'claude', to: 'claude', limited: false, until: null, by: 'Alice Martin' }) });
    expect(screen.getByRole('separator')).toHaveTextContent(/^Passé par Alice Martin · passation, suite sur Claude Code$/);
    passed.unmount();
    const resumed = render(Handover, {
      item: sep({ from: 'claude', to: 'claude', limited: false, until: null, by: 'Alice Martin', native: true }),
    });
    expect(screen.getByRole('separator')).toHaveTextContent(/^Passé par Alice Martin · reprise de la session, suite sur Claude Code$/);
    expect(screen.getByRole('separator').querySelector('b')).toHaveTextContent(/^Passé par Alice Martin$/);
    resumed.unmount();
    setLang('en');
    render(Handover, { item: sep({ from: 'claude', to: 'claude', limited: false, until: null, by: 'Alice Martin', native: true }) });
    expect(screen.getByRole('separator')).toHaveTextContent('Passed by Alice Martin · session resumed, continued on Claude Code');
  });

  it('folds the first message of a session resumed under its own title', async () => {
    const first: UserItem = { kind: 'user', id: 'u2', text: '# Reprise de ta session', images: 0, ts: 1, queued: false, origin: 'resume' };
    render(UserMessage, { item: first, engine: 'claude' });
    const fold = screen.getByRole('button', { name: /Reprise de la session/ });
    expect(fold).toHaveTextContent('Écrit par Escouade : la session continue ici, sur Claude Code, avec tout son contexte.');
    expect(screen.queryByText('# Reprise de ta session')).not.toBeInTheDocument();
    await userEvent.click(fold);
    expect(screen.getByText('# Reprise de ta session')).toBeInTheDocument();
  });

  it('folds the handover file under its title', async () => {
    const file: UserItem = { kind: 'user', id: 'u1', text: '# Passation : suite…', images: 0, ts: 1, queued: false, origin: 'handover' };
    render(UserMessage, { item: file, engine: 'codex' });
    const fold = screen.getByRole('button', { name: /Dossier de passation/ });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(fold).toHaveTextContent('Écrit par Escouade depuis la conversation : le premier message de l’agent sur Codex.');
    expect(screen.queryByText('# Passation : suite…')).not.toBeInTheDocument();
    await userEvent.click(fold);
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('# Passation : suite…')).toBeInTheDocument();
  });
});

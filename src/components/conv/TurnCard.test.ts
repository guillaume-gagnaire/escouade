import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TurnItem } from '../../lib/types';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import TurnCard from './TurnCard.svelte';

const turn = (over: Partial<TurnItem> = {}): TurnItem => ({
  kind: 'turn',
  id: 'r1',
  ts: 1,
  durationMs: 151_000,
  cost: 2.84,
  tokens: 182_400,
  isError: false,
  interrupted: false,
  error: null,
  ...over,
});

describe('TurnCard', () => {
  beforeEach(() => resetApp());

  it('ends the last turn of a finished agent with the recap of the files it edited, and nothing but the files to click', () => {
    const a = agent({ status: 'done', worktree: { path: 'C:\\code\\.claude\\worktrees\\x', branch: 'ccm/x', baseBranch: 'main' } });
    const edits = [
      { path: 'src/auth.ts', add: 12, del: 3 },
      { path: 'notes.md', add: 4, del: 0 },
    ];
    render(TurnCard, { item: turn(), agent: a, last: true, edits });
    expect(screen.getByText('Tâche terminée')).toBeInTheDocument();
    expect(screen.getByText('2m 31s')).toBeInTheDocument();
    expect(screen.getByText('2 fichiers modifiés')).toBeInTheDocument();
    const recap = screen.getAllByRole('listitem');
    expect(recap.map((li) => li.textContent)).toEqual(['src/auth.ts+12−3', 'notes.md+4−0']);
    // No commit, merge nor review to propose: the files are the only buttons.
    expect(screen.getAllByRole('button').map((b) => b.textContent)).toEqual(['src/auth.ts', 'notes.md']);
  });

  it('opens an edited file of the recap in the editor, on the worktree of the agent that edited it', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api/.claude/worktrees/x', files: [], truncated: false }) });
    const a = agent({
      status: 'done',
      cwd: 'C:\\code\\demo-api\\.claude\\worktrees\\x',
      worktree: { path: 'C:\\code\\demo-api\\.claude\\worktrees\\x', branch: 'ccm/x', baseBranch: 'main' },
    });
    render(TurnCard, { item: turn(), agent: a, last: true, edits: [{ path: 'src/auth.ts', add: 12, del: 3 }] });
    await userEvent.click(screen.getByRole('button', { name: 'src/auth.ts' }));
    await waitFor(() => expect(app.editor.p1?.places.a1?.active).toBe('src/auth.ts'));
    expect(app.editor.p1).toMatchObject({ on: true, source: 'a1' });
  });

  it('opens an edited file of the recap on the project checkout for an agent without a worktree', async () => {
    fakeBackend({ fs_tree: () => ({ root: 'C:/code/demo-api', files: [], truncated: false }) });
    render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: true, edits: [{ path: 'notes.md', add: 4, del: 0 }] });
    await userEvent.click(screen.getByRole('button', { name: 'notes.md' }));
    await waitFor(() => expect(app.editor.p1?.places.project?.active).toBe('notes.md'));
    expect(app.editor.p1.source).toBe('project');
  });

  it('opens a recap file outside the agent’s folder by its own path, not under the folder', async () => {
    const open = vi.spyOn(app, 'openEditor').mockResolvedValue();
    try {
      const edits = [
        { path: 'C:/Users/guill/.claude/plans/plan.md', add: 9, del: 0 },
        { path: '/tmp/notes.md', add: 1, del: 0 },
      ];
      render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: true, edits });
      await userEvent.click(screen.getByRole('button', { name: 'C:/Users/guill/.claude/plans/plan.md' }));
      await userEvent.click(screen.getByRole('button', { name: '/tmp/notes.md' }));
      expect(open.mock.calls.map(([r]) => r.abs)).toEqual(['C:/Users/guill/.claude/plans/plan.md', '/tmp/notes.md']);
    } finally {
      open.mockRestore();
    }
  });

  it('says when the turn edited no file', () => {
    render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: true });
    expect(screen.getByText('0 fichier modifié')).toBeInTheDocument();
    expect(screen.queryByRole('list')).toBeNull();
  });

  it('is a discreet separator for earlier turns', () => {
    render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: false });
    expect(screen.queryByText('Tâche terminée')).not.toBeInTheDocument();
    expect(screen.getByText(/182,4 k tokens/)).toBeInTheDocument();
  });

  it('tells when an agent stopped by the usage limit resumes by itself, and cancels it', async () => {
    const backend = fakeBackend();
    app.now = new Date(2026, 8, 30, 12, 0).getTime();
    const a = agent({ status: 'error', resumeAt: new Date(2026, 8, 30, 15, 0).getTime() });
    render(TurnCard, { item: turn({ isError: true, error: "You've hit your limit · resets 3pm" }), agent: a, last: true });
    expect(screen.getByText('Reprise automatique à 15:00')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Annuler la reprise' }));
    expect(backend.called('cancel_resume')[0].args).toEqual({ id: 'a1' });
  });

  it('shows the error of a failed turn', () => {
    render(TurnCard, { item: turn({ isError: true, error: 'Rate limit reached' }), agent: agent({ status: 'error' }), last: true });
    expect(screen.getByText("Le tour s'est terminé en erreur")).toBeInTheDocument();
    expect(screen.getByText('Rate limit reached')).toBeInTheDocument();
  });

  it('marks an interrupted turn', () => {
    render(TurnCard, { item: turn({ interrupted: true }), agent: agent({ status: 'done' }), last: true });
    expect(screen.queryByText('Tâche terminée')).not.toBeInTheDocument();
    expect(screen.getByText(/Interrompu/)).toBeInTheDocument();
  });
});

describe('TurnCard in English', () => {
  beforeEach(() => resetApp());

  it('recaps a finished task in English, with the figures written as in English', () => {
    setLang('en');
    const edits = [
      { path: 'src/auth.ts', add: 12, del: 3 },
      { path: 'notes.md', add: 4, del: 0 },
    ];
    const { unmount } = render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: true, edits });
    expect(screen.getByText('Task done')).toBeInTheDocument();
    expect(screen.getByText('182.4k tokens')).toBeInTheDocument();
    expect(screen.getByText('$2.84')).toBeInTheDocument();
    expect(screen.getByText('2 files edited')).toBeInTheDocument();
    expect(screen.getByRole('list', { name: 'Edited files' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'src/auth.ts' })).toHaveAttribute('title', 'Open in editor');
    unmount();
    render(TurnCard, { item: turn(), agent: agent({ status: 'done' }), last: true, edits: edits.slice(0, 1) });
    expect(screen.getByText('1 file edited')).toBeInTheDocument();
  });

  it('says in English that a turn failed, can be resumed, or was interrupted', () => {
    setLang('en');
    const { unmount } = render(TurnCard, {
      item: turn({ isError: true, error: 'Rate limit reached' }),
      agent: agent({ status: 'error', resumeAt: Date.now() + 3600_000 }),
      last: true,
    });
    expect(screen.getByText('The turn ended with an error')).toBeInTheDocument();
    expect(screen.getByText(/^Auto-resume (at|on) /)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Cancel auto-resume' })).toBeInTheDocument();
    unmount();
    render(TurnCard, { item: turn({ interrupted: true }), agent: agent({ status: 'done' }), last: true });
    expect(screen.getByText(/Interrupted · 182\.4k tokens · \$2\.84/)).toBeInTheDocument();
  });
});

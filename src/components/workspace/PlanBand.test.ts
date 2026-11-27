import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import { agent, fakeBackend, resetApp } from '../../test/ipc';
import { planWith } from '../../test/workspace';
import PlanBand from './PlanBand.svelte';

describe('the plan of an agent on the workspace', () => {
  beforeEach(() => {
    setLang('fr');
    fakeBackend();
    resetApp();
  });

  it("shows the conversation's banner: its count, its share and a row per task", () => {
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'done', 'done', 'inProgress', 'pending', 'pending', 'pending']) }) });
    expect(screen.getAllByText('3/7 tâches').length).toBeGreaterThan(0); // its count, and what a screen reader is told
    expect(screen.getByText('43 %')).toBeTruthy();
    expect(screen.getAllByRole('listitem')).toHaveLength(7);
    expect(screen.getByText('04')).toBeTruthy();
  });

  it('keeps every task of a long plan, in its list that scrolls', () => {
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'inProgress', ...Array(10).fill('pending')]) }) });
    expect(screen.getAllByRole('listitem')).toHaveLength(12);
  });

  it('keeps only its title and its gauge when small, and unfolds on a click', async () => {
    const onunfold = vi.fn();
    render(PlanBand, { agent: agent({ plan: planWith(['done', 'pending']) }), mini: true, onunfold });
    expect(screen.getAllByText('1/2 tâches').length).toBeGreaterThan(0);
    expect(screen.getByText('50 %')).toBeTruthy();
    expect(screen.queryAllByRole('listitem')).toHaveLength(0);
    await fireEvent.click(screen.getByRole('button', { expanded: false }));
    expect(onunfold).toHaveBeenCalled();
    expect(app.planOpen['a1']).toBeUndefined();
  });
});

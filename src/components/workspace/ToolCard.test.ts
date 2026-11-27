import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import ToolCard from './ToolCard.svelte';

describe('the card of a tool on the workspace', () => {
  beforeEach(() => setLang('fr'));

  it('says a skill, its calls and that it runs', () => {
    const { container } = render(ToolCard, {
      tool: { kind: 'skill', name: 'tdd', calls: 3, running: true, runningIn: null, firstAt: 1, lastAt: 1 },
    });
    expect(screen.getByRole('article', { name: 'Skill tdd, 3 appels, en cours' })).toBeTruthy();
    expect(screen.getByText('en cours')).toBeTruthy();
    expect(container.querySelector('article.skill.running')).toBeTruthy();
  });

  it('says a plugin, and nothing runs', () => {
    render(ToolCard, { tool: { kind: 'plugin', name: 'github', calls: 1, running: false, runningIn: null, firstAt: 1, lastAt: 1 } });
    expect(screen.getByRole('article', { name: 'Plugin github, 1 appel' })).toBeTruthy();
    expect(screen.queryByText('en cours')).toBeNull();
  });

  it('shows the server of a plugin of Claude Code, its whole name in the tooltip', () => {
    render(ToolCard, {
      tool: { kind: 'plugin', name: 'plugin_superpowers_chrome', calls: 2, running: false, runningIn: null, firstAt: 1, lastAt: 1 },
    });
    expect(screen.getByRole('article', { name: 'Plugin superpowers · chrome, 2 appels' })).toBeTruthy();
    expect(screen.getByText('superpowers · chrome')).toBeTruthy();
  });
});

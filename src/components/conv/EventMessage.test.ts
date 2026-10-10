import { render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import EventMessage from './EventMessage.svelte';

const NOTIFICATION =
  '<task-notification>\n<task-id>b1</task-id>\n<status>completed</status>\n<summary>Background command "npm test" completed (exit code 0)</summary>\n</task-notification>';
const REPORT =
  '<agent-message from="a42">\n[Subagent hand-back] … The report follows:\n  **Cause trouvée** dans `Composer.svelte`.\n</agent-message>';

describe('EventMessage', () => {
  it('shows a background task that ended as a discreet line, not as a message of the user', () => {
    const { container } = render(EventMessage, { source: 'task', text: NOTIFICATION });
    expect(screen.getByText('Tâche de fond terminée')).toBeInTheDocument();
    expect(screen.getByText('Background command "npm test" completed (exit code 0)')).toBeInTheDocument();
    expect(container.querySelector('.bubble')).toBeNull();
    expect(container).not.toHaveTextContent('<task-notification>');
  });

  it('says when a background task failed', () => {
    render(EventMessage, { source: 'task', text: NOTIFICATION.replace('completed</status>', 'failed</status>') });
    expect(screen.getByText('Tâche de fond en échec')).toBeInTheDocument();
  });

  it('shows the report of a subagent under its task, rendered', () => {
    const { container } = render(EventMessage, { source: 'agent', text: REPORT, label: 'Investigate PDF upload bug' });
    expect(screen.getByText('Rapport du sous-agent « Investigate PDF upload bug »')).toBeInTheDocument();
    expect(screen.getByText('Cause trouvée').tagName).toBe('STRONG');
    expect(container).not.toHaveTextContent('Subagent hand-back');
    expect(container.querySelector('.bubble')).toBeNull();
  });

  it('names an unknown subagent plainly, and tells a message from another session', () => {
    render(EventMessage, { source: 'agent', text: REPORT });
    expect(screen.getByText('Rapport d’un sous-agent')).toBeInTheDocument();
    render(EventMessage, { source: 'agent', text: '<agent-message from="b2">\nOn se synchronise ?\n</agent-message>' });
    expect(screen.getByText('Message d’une autre session Claude')).toBeInTheDocument();
    expect(screen.getByText('On se synchronise ?')).toBeInTheDocument();
  });

  it('does not call ended a notification that says nothing of it, and shows what it says', () => {
    render(EventMessage, {
      source: 'task',
      text: '<task-notification>\n<kind>scheduled-trigger</kind>\n<message>Relance quotidienne</message>\n</task-notification>',
    });
    expect(screen.queryByText(/terminée/)).not.toBeInTheDocument();
    expect(screen.getByText('Notification')).toBeInTheDocument();
    expect(screen.getByText('scheduled-trigger Relance quotidienne')).toBeInTheDocument();
  });

  it('shows what other sources pass on, and a long summary in full on hover', () => {
    render(EventMessage, { source: 'channel', text: 'Nouveau ticket #42' });
    expect(screen.getByText('Message de Claude Code (channel)')).toBeInTheDocument();
    expect(screen.getByText('Nouveau ticket #42')).toBeInTheDocument();
    render(EventMessage, { source: 'task', text: NOTIFICATION });
    const sum = screen.getByText('Background command "npm test" completed (exit code 0)');
    expect(sum).toHaveAttribute('title', 'Background command "npm test" completed (exit code 0)');
  });
});

describe('EventMessage in English', () => {
  it('words a background task and a subagent in English', () => {
    setLang('en');
    const { unmount } = render(EventMessage, { source: 'task', text: NOTIFICATION });
    expect(screen.getByText('Background task completed')).toBeInTheDocument();
    unmount();
    const { unmount: failed } = render(EventMessage, {
      source: 'task',
      text: NOTIFICATION.replace('completed</status>', 'failed</status>'),
    });
    expect(screen.getByText('Background task failed')).toBeInTheDocument();
    failed();
    const { unmount: report } = render(EventMessage, { source: 'agent', text: REPORT, label: 'Investigate PDF upload bug' });
    expect(screen.getByText('Report from the subagent “Investigate PDF upload bug”')).toBeInTheDocument();
    report();
    render(EventMessage, { source: 'channel', text: 'New ticket #42' });
    expect(screen.getByText('Message from Claude Code (channel)')).toBeInTheDocument();
  });
});

import { describe, expect, it } from 'vitest';
import { newPermissions, permissionWords } from '../../server/utils/author';
import { authorMail, type AuthorEvent } from '../../server/utils/mail';

// What the author's space says without a base: the permissions a new version adds, and the e-mails of each event, in
// both languages, each with the account's page and the warning for an action the author did not do.
describe('the author’s space', () => {
  it('flattens the permissions and finds what a version asks more', () => {
    expect(permissionWords({ permissions: { network: ['b.dev', 'a.dev'], decide: true, events: ['agent.created'] } })).toEqual([
      'decide',
      'events:agent.created',
      'network:a.dev',
      'network:b.dev',
    ]);
    expect(newPermissions({ permissions: { network: ['a.dev'] } }, { permissions: { network: ['a.dev'], secrets: ['TOKEN'] } })).toEqual([
      'secrets:TOKEN',
    ]);
    expect(newPermissions(null, { permissions: {} })).toEqual([]);
  });

  it('writes every event’s e-mail in French and in English, with the page and the warning', () => {
    const events: AuthorEvent[] = [
      'ready',
      'auto-published',
      'published',
      'yanked',
      'unlisted',
      'relisted',
      'transfer-offered',
      'transfer-asked',
      'transfer-refused',
      'transferred-in',
      'transferred-out',
      'deleted',
      'restored',
    ];
    const subjects = new Set<string>();
    for (const e of events) {
      const m = authorMail('lea@example.dev', e, {
        plugin: 'dev.lea.slack',
        version: '1.3.0',
        other: 'sroux',
        until: new Date('2026-11-05T10:00:00Z'),
        page: 'https://store.escouade.dev/account?plugin=dev.lea.slack',
      });
      expect(m.to).toBe('lea@example.dev');
      expect(m.subject).toContain('dev.lea.slack');
      expect(m.text).toContain('https://store.escouade.dev/account?plugin=dev.lea.slack');
      expect(m.text).toContain('Si ce n');
      expect(m.text).toContain('If it was not you');
      expect(m.text).not.toContain('undefined');
      subjects.add(m.subject);
    }
    expect(subjects.size).toBe(events.length);
    const frozen = authorMail('x@y.dev', 'transferred-in', {
      plugin: 'p',
      other: 'lea',
      until: new Date('2026-11-05T10:00:00Z'),
      page: 'u',
    });
    expect(frozen.text).toContain('5 novembre 2026');
    expect(frozen.text).toContain('5 November 2026');
  });
});

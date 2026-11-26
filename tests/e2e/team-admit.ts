// The admission of a device to a team (SR2-08), for the specs that make and join their team by the backend's commands:
// the inviting client waits for the request, checks its six words are the invitee's, and confirms them (« Les mots sont
// les mêmes »), the only way in. The screens of it are `team.spec.ts`'s.

import type { Page } from '@playwright/test';
import { expect } from './fixture';

interface Admission {
  invitation: string;
  words: string[];
}

function ipc<T>(page: Page, cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  return page.evaluate(
    ([c, a]) =>
      (window as unknown as { __TAURI_INTERNALS__: { invoke(c: string, a?: unknown): Promise<unknown> } }).__TAURI_INTERNALS__.invoke(c, a),
    [cmd, args] as const,
  ) as Promise<T>;
}

/** The inviter's client (`page`) lets in the invitee whose screen shows `words` (the `joinWords` of its `team_join`). */
export async function letIn(page: Page, words: string[]): Promise<void> {
  expect(words).toHaveLength(6);
  let found: { team: string; admission: Admission } | null = null;
  await expect
    .poll(
      async () => {
        const list = await ipc<{ id: string; admissions?: Admission[] }[]>(page, 'team_list');
        for (const t of list) {
          const a = (t.admissions ?? []).find((x) => x.words.join(' ') === words.join(' '));
          if (a) found = { team: t.id, admission: a };
        }
        return found !== null;
      },
      { timeout: 60_000, message: 'the request shows on the inviting client with the invitee’s six words' },
    )
    .toBe(true);
  const { team, admission } = found!;
  await ipc(page, 'team_admit', { team, invitation: admission.invitation, words: admission.words });
}

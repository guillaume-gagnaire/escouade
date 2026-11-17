import type { TeamDocsView } from '../lib/team-docs';

// A view of a team project's docs (TS-9) for the tests: on the server, in step, nothing to show.
export function teamDocsView(over: Partial<TeamDocsView> = {}): TeamDocsView {
  return {
    canChoose: true,
    onServer: true,
    journal: '00112233445566778899aabbccddeeff',
    online: true,
    pending: 0,
    conflicts: [],
    decisionConflicts: [],
    repoFiles: [],
    tooLarge: [],
    waitingNumbers: [],
    maxDocument: 20 * 1024 * 1024,
    ...over,
  };
}

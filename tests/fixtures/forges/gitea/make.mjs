// Writes the answers of the fake Gitea / Forgejo (see README.md): `node make.mjs`.
// Written from the swagger documents of Forgejo 16 and Gitea's main branch, not recorded from an instance.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const out = path.dirname(fileURLToPath(import.meta.url));
const w = (f, v) => fs.writeFileSync(path.join(out, f), typeof v === 'string' ? v : JSON.stringify(v, null, 2) + '\n');
const user = (login, id) => ({
  id,
  login,
  login_name: '',
  full_name: login,
  email: `${login}@noreply.codeberg.org`,
  avatar_url: `https://codeberg.org/avatars/${id}`,
  html_url: `https://codeberg.org/${login}`,
  is_admin: false,
  created: '2024-01-02T10:00:00Z',
});
const sha = '3f2a9c1d5e7b84a6f0c2d9e8b7a6543210fedcba';
const baseSha = '0a1b2c3d4e5f60718293a4b5c6d7e8f901234567';
const repo = (id, full) => ({
  id,
  name: full.split('/')[1],
  full_name: full,
  owner: user(full.split('/')[0], id),
  fork: id !== 5,
  html_url: `https://codeberg.org/${full}`,
  default_branch: 'main',
  has_actions: true,
  has_pull_requests: true,
});
const branch = (ref, repoId, full, s) => ({ label: ref, ref, sha: s, repo_id: repoId, repo: repo(repoId, full) });
const pull = (extra = {}) => ({
  id: 101,
  url: 'https://codeberg.org/ada/notes/pulls/7',
  number: 7,
  user: user('ada', 2),
  title: 'Add search',
  body: 'Adds the search page.\n\nCloses #3',
  labels: [{ id: 1, name: 'agent', color: '00aabb' }],
  state: 'open',
  draft: false,
  is_locked: false,
  comments: 1,
  review_comments: 2,
  additions: 120,
  deletions: 4,
  changed_files: 3,
  html_url: 'https://codeberg.org/ada/notes/pulls/7',
  diff_url: 'https://codeberg.org/ada/notes/pulls/7.diff',
  patch_url: 'https://codeberg.org/ada/notes/pulls/7.patch',
  mergeable: true,
  merged: false,
  merged_at: null,
  merged_by: null,
  merge_commit_sha: null,
  merge_base: baseSha,
  allow_maintainer_edit: false,
  requested_reviewers: [user('linus', 4)],
  requested_reviewers_teams: [],
  head: branch('feat/search', 5, 'ada/notes', sha),
  base: branch('main', 5, 'ada/notes', baseSha),
  created_at: '2026-10-30T09:00:00Z',
  updated_at: '2026-10-31T16:20:00Z',
  closed_at: null,
  due_date: null,
  pin_order: 0,
  ...extra,
});
w('pull.json', pull({ content_version: 3 }));
w('forgejo-pull.json', pull({ flow: 0 }));
w(
  'pull-merged.json',
  pull({
    state: 'closed',
    merged: true,
    mergeable: false,
    merged_at: '2026-10-31T17:00:00Z',
    closed_at: '2026-10-31T17:00:00Z',
    merged_by: user('ada', 2),
    merge_commit_sha: 'bb11cc22dd33ee44ff5566778899aabbccddeeff',
  }),
);
w('pull-draft.json', pull({ title: 'WIP: Add search', draft: true }));
w('pull-from-fork.json', pull({ head: branch('feat/search', 9, 'bob/notes', sha) }));
w('pulls-all.json', [
  pull(),
  pull({
    number: 5,
    id: 99,
    title: 'Old attempt',
    state: 'closed',
    html_url: 'https://codeberg.org/ada/notes/pulls/5',
    head: branch('feat/search', 9, 'bob/notes', 'c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00'),
  }),
  pull({ number: 6, id: 100, title: 'Other', head: branch('fix/typo', 5, 'ada/notes', 'd00dfeed') }),
]);
const review = (id, login, uid, state, count, body = '') => ({
  id,
  user: user(login, uid),
  team: null,
  state,
  body,
  commit_id: sha,
  stale: false,
  official: true,
  dismissed: false,
  comments_count: count,
  submitted_at: '2026-10-31T10:00:00Z',
  updated_at: '2026-10-31T10:00:00Z',
  html_url: `https://codeberg.org/ada/notes/pulls/7#issuecomment-${id}`,
  pull_request_url: 'https://codeberg.org/ada/notes/pulls/7',
});
w('reviews.json', [
  review(11, 'grace', 3, 'APPROVED', 0, 'LGTM'),
  review(12, 'linus', 4, 'REQUEST_CHANGES', 2, 'Fix the loop'),
  review(13, 'grace', 3, 'COMMENT', 0, 'One more nit'),
  review(14, 'mallory', 6, 'PENDING', 1),
]);
const rc = (id, login, uid, position, resolver) => ({
  id,
  body: `comment ${id}`,
  user: user(login, uid),
  resolver: resolver ? user(resolver, 2) : null,
  pull_request_review_id: 12,
  path: 'src/search.rs',
  commit_id: sha,
  original_commit_id: sha,
  diff_hunk: '@@ -1,3 +1,5 @@',
  position,
  original_position: position,
  html_url: `https://codeberg.org/ada/notes/pulls/7#issuecomment-${id}`,
  pull_request_url: 'https://codeberg.org/ada/notes/pulls/7',
  created_at: '2026-10-31T10:00:00Z',
  updated_at: '2026-10-31T10:00:00Z',
});
w('review-12-comments.json', [rc(201, 'linus', 4, 14, null), rc(202, 'linus', 4, 20, 'ada')]);
w('issue-comments.json', [
  {
    id: 301,
    html_url: 'https://codeberg.org/ada/notes/pulls/7#issuecomment-301',
    pull_request_url: 'https://codeberg.org/ada/notes/pulls/7',
    issue_url: 'https://codeberg.org/ada/notes/issues/7',
    user: user('ada', 2),
    original_author: '',
    original_author_id: 0,
    body: 'Rebased on main.',
    assets: [],
    created_at: '2026-10-31T12:00:00Z',
    updated_at: '2026-10-31T12:00:00Z',
  },
]);
const status = (id, context, st, url, at) => ({
  id,
  status: st,
  target_url: url,
  description: '',
  url: `https://codeberg.org/api/v1/repos/ada/notes/statuses/${sha}`,
  context,
  creator: user('ada', 2),
  created_at: at,
  updated_at: at,
});
const runUrl = 'https://codeberg.org/ada/notes/actions/runs/41';
const statuses = [
  status(5, 'CI / build (pull_request)', 'success', `${runUrl}/jobs/0`, '2026-10-31T15:05:00Z'),
  status(4, 'CI / test (pull_request)', 'failure', `${runUrl}/jobs/1`, '2026-10-31T15:09:00Z'),
  status(3, 'lint/external', 'pending', 'https://ci.acme.dev/builds/9', '2026-10-31T15:00:30Z'),
  status(2, 'CI / build (pull_request)', 'pending', `${runUrl}/jobs/0`, '2026-10-31T15:00:00Z'),
];
w('statuses.json', statuses);
w('combined-status.json', {
  state: 'failure',
  sha,
  total_count: 3,
  statuses: statuses.slice(0, 3),
  repository: repo(5, 'ada/notes'),
  commit_url: `https://codeberg.org/api/v1/repos/ada/notes/git/commits/${sha}`,
  url: `https://codeberg.org/api/v1/repos/ada/notes/commits/${sha}/status`,
});
w('combined-status-empty.json', { state: 'pending', sha, total_count: 0, statuses: [], repository: repo(5, 'ada/notes') });
w('gitea-runs.json', {
  total_count: 1,
  workflow_runs: [
    {
      id: 900,
      run_number: 41,
      display_title: 'Add search',
      event: 'pull_request',
      head_branch: 'feat/search',
      head_sha: sha,
      path: 'ci.yaml@refs/pull/7/head',
      status: 'completed',
      conclusion: 'failure',
      run_attempt: 1,
      created_at: '2026-10-31T15:00:00Z',
      started_at: '2026-10-31T15:00:10Z',
      completed_at: '2026-10-31T15:09:00Z',
      html_url: runUrl,
      jobs_url: 'https://codeberg.org/api/v1/repos/ada/notes/actions/runs/900/jobs',
    },
  ],
});
const gjob = (id, name, conclusion, from, to, index) => ({
  id,
  run_id: 900,
  name,
  head_sha: sha,
  head_branch: 'feat/search',
  status: 'completed',
  conclusion,
  run_attempt: 1,
  started_at: from,
  completed_at: to,
  labels: ['ubuntu-latest'],
  steps: [],
  html_url: `${runUrl}/jobs/${index}`,
});
w('gitea-run-jobs.json', {
  total_count: 2,
  jobs: [
    gjob(5001, 'build', 'success', '2026-10-31T15:00:10Z', '2026-10-31T15:05:00Z', 0),
    gjob(5002, 'test', 'failure', '2026-10-31T15:05:10Z', '2026-10-31T15:09:00Z', 1),
  ],
});
w('forgejo-runs.json', {
  total_count: 1,
  workflow_runs: [
    {
      id: 900,
      index_in_repo: 41,
      title: 'Add search',
      workflow_id: 'ci.yaml',
      event: 'pull_request',
      trigger_event: 'pull_request',
      commit_sha: sha,
      prettyref: 'feat/search',
      status: 'failure',
      started: '2026-10-31T15:00:10Z',
      stopped: '2026-10-31T15:09:00Z',
      created: '2026-10-31T15:00:00Z',
      html_url: runUrl,
      repository: repo(5, 'ada/notes'),
    },
  ],
});
const fjob = (id, name, status, needs, task) => ({
  id,
  run_id: 900,
  repo_id: 5,
  owner_id: 2,
  name,
  needs,
  runs_on: ['docker'],
  status,
  attempt: 1,
  task_id: task,
  handle: name,
});
w('forgejo-run-jobs.json', [fjob(5001, 'build', 'success', [], 7001), fjob(5002, 'test', 'failure', ['build'], 7002)]);
w(
  'job-log.txt',
  [
    '2026-10-31T15:05:12.1Z ##[group]Run cargo test',
    '2026-10-31T15:07:40.8Z test search::finds_notes ... FAILED',
    "2026-10-31T15:07:41.0Z thread 'search::finds_notes' panicked at src/search.rs:14:5:",
    '2026-10-31T15:09:00.2Z ##[error]Process completed with exit code 101.',
    '',
  ].join('\n'),
);
const br = (name, isProtected) => ({
  name,
  commit: { id: sha, message: 'wip', url: `https://codeberg.org/ada/notes/commit/${sha}` },
  protected: isProtected,
  required_approvals: isProtected ? 1 : 0,
  enable_status_check: isProtected,
  status_check_contexts: isProtected ? ['CI / test (pull_request)'] : [],
  user_can_push: !isProtected,
  user_can_merge: true,
  effective_branch_protection_name: isProtected ? name : '',
});
w('branch-main.json', br('main', true));
w('branch-dev.json', br('dev', false));
w('repo-fork.json', { ...repo(9, 'bob/notes'), parent: repo(5, 'ada/notes') });
w('repo.json', repo(5, 'ada/notes'));
w('labels.json', [
  { id: 1, name: 'agent', color: '00aabb', description: '' },
  { id: 2, name: 'bug', color: 'ee0000', description: '' },
]);
w('user.json', user('ada', 2));

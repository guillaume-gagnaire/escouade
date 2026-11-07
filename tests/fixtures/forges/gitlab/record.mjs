// Records GitLab's answers for the tests of the plugin « GitLab » (`src-tauri/src/forge/gitlab_tests.rs`), from a
// merged merge request of a public project in a subgroup of gitlab.com: `node record.mjs [group/subgroup/project] [iid]`.
// Without a token, gitlab.com answers the project, the merge request, its approvals, reviewers, pipelines, jobs and a
// job's raw log, but not its discussions nor the deployments (401): with `GITLAB_TOKEN` (a personal token, `read_api`)
// these and the log by `/jobs/:id/trace` are recorded; else `discussions.json` and `deployments.json` stay as written from GitLab's documentation.
// Fields the client does not read (avatars, `_links`, `references`…) are cut out, which keeps the files small; the
// people are taken out (`anonymize.mjs`: aliases, e-mails at example.com), the kept files with them, so that an alias
// is the same in every file; the rest is as GitLab sent it.
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { anonymize } from './anonymize.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const [project = 'gitlab-org/api/client-go', iid = '3101'] = process.argv.slice(2);
const base = `https://gitlab.com/api/v4/projects/${encodeURIComponent(project)}`;
const token = process.env.GITLAB_TOKEN;

const get = async (path, text = false) => {
  const r = await fetch(base + path, { headers: token ? { 'PRIVATE-TOKEN': token } : {} });
  if (!r.ok) throw new Error(`${path}: ${r.status}`);
  return text ? r.text() : r.json();
};
const drop = new Set([
  'avatar_url',
  'permissions',
  'container_expiration_policy',
  '_links',
  'references',
  'time_stats',
  'task_completion_status',
  'artifacts',
  'runner',
  'runner_manager',
  'commit',
  'artifacts_file',
  'project',
  'pipeline',
  'description_html',
]);
const trim = (v) => {
  if (Array.isArray(v)) return v.map(trim);
  if (v && typeof v === 'object') {
    const out = {};
    for (const [k, x] of Object.entries(v)) {
      if (drop.has(k)) continue;
      if ((k === 'description' || k === 'body') && typeof x === 'string') {
        out[k] = x.slice(0, 400);
        continue;
      }
      out[k] = trim(x);
    }
    return out;
  }
  return v;
};
/** What is recorded, by file: written once every person is known (`anonymize`). */
const files = new Map();
const save = (file, data) => files.set(file, trim(data));

save('project.json', await get(''));
const mr = await get(`/merge_requests/${iid}`);
save('merge-request.json', mr);
// A merge request from a fork: its source project says where it was forked from.
if (mr.source_project_id !== mr.target_project_id) {
  const r = await fetch(`https://gitlab.com/api/v4/projects/${mr.source_project_id}`);
  save('fork-project.json', await r.json());
}
save('approvals.json', await get(`/merge_requests/${iid}/approvals`));
save('reviewers.json', await get(`/merge_requests/${iid}/reviewers`));
save('mr-pipelines.json', await get(`/merge_requests/${iid}/pipelines`));
save(
  'branch-mrs.json',
  await get(`/merge_requests?source_branch=${encodeURIComponent(mr.source_branch)}&order_by=created_at&sort=desc&per_page=10`),
);
const jobs = await get(`/pipelines/${mr.head_pipeline.id}/jobs?per_page=100`);
save('jobs.json', jobs);
const job = jobs.find((j) => j.status === 'success') ?? jobs[0];
writeFileSync(join(here, 'job-id.txt'), `${job.id}\n`);
// The API's `/jobs/:id/trace` asks for a token on gitlab.com; the page's raw log is the same text.
const log = token
  ? await get(`/jobs/${job.id}/trace`, true)
  : await (await fetch(`https://gitlab.com/${project}/-/jobs/${job.id}/raw`)).text();
files.set('job-log.txt', log.slice(0, 3000));
// Without a token, the files written from the documentation are kept, and their people taken out with the others.
for (const [file, path] of [
  ['discussions.json', `/merge_requests/${iid}/discussions?per_page=100`],
  ['deployments.json', `/deployments?order_by=updated_at&sort=desc&per_page=20`],
]) {
  save(file, token ? await get(path) : JSON.parse(readFileSync(join(here, file), 'utf8')));
}
const names = [...files.keys()];
anonymize([...files.values()]).forEach((v, i) =>
  writeFileSync(join(here, names[i]), typeof v === 'string' ? v : JSON.stringify(v, null, 2) + '\n'),
);
console.log(
  `recorded ${project}!${iid}, pipeline ${mr.head_pipeline.id}, job ${job.id}${token ? '' : ' (no token: discussions and deployments kept)'}`,
);

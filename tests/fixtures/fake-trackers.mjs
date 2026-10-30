#!/usr/bin/env node
// Test double of Jira Cloud, Trello, GitHub Issues, ClickUp, a self-hosted GitLab and Azure DevOps on one local HTTP server, for the end-to-end
// tests and to try the integrations without real accounts:
//   node tests/fixtures/fake-trackers.mjs --port 4101
// then, in Escouade: Jira's site = http://127.0.0.1:4101 (any e-mail, any token but "bad"), and the
// app started with ESCOUADE_TRELLO_API=http://127.0.0.1:4101/1 and
// ESCOUADE_GITHUB_API=http://127.0.0.1:4101/github, ESCOUADE_CLICKUP_API=http://127.0.0.1:4101/clickup/api/v2
// (any key or token but "bad"); GitLab: connect a code host of kind GitLab at http://127.0.0.1:4101 (its API at
// the root, /api/v4), its projects then GitLab Issues' sources; Azure DevOps: the app started with
// ESCOUADE_AZURE_DEVOPS_API=http://127.0.0.1:4101/azure, then a code host of kind Azure DevOps at dev.azure.com/acme.
// What the app changed is kept in memory: GET /__writes lists the writes received.

import http from 'node:http';
import { pathToFileURL } from 'node:url';

function seed() {
  const adf = (...blocks) => ({ type: 'doc', version: 1, content: blocks });
  const para = (text) => ({ type: 'paragraph', content: [{ type: 'text', text }] });
  const bullets = (...items) => ({
    type: 'bulletList',
    content: items.map((t) => ({ type: 'listItem', content: [para(t)] })),
  });
  return {
    jira: {
      statuses: [
        { id: '1', name: 'To Do' },
        { id: '3', name: 'In Progress' },
        { id: '10', name: 'In Review' },
        { id: '5', name: 'Done' },
      ],
      issues: [
        {
          key: 'ATL-1287',
          summary: 'Rafraîchir le token silencieusement côté client',
          type: 'Story',
          priority: 'High',
          assignee: 'Ada Lovelace',
          status: '1',
          labels: ['claude-ready'],
          description: adf(
            para('Le token expire pendant la saisie : le client doit le renouveler avant.'),
            para('Acceptance criteria:'),
            bullets('Refresh avant expiration', 'Aucune erreur 401 visible'),
          ),
        },
        {
          key: 'ATL-1290',
          summary: "Erreur 500 quand l'email contient un « + »",
          type: 'Bug',
          priority: 'Highest',
          assignee: null,
          status: '1',
          labels: [],
          description: adf(para('Reproduit avec ada+test@atlas.dev.')),
        },
      ],
    },
    trello: {
      lists: [
        { id: 'l1', name: 'Prêt pour Claude' },
        { id: 'l2', name: 'En cours' },
        { id: 'l3', name: 'À tester' },
        { id: 'l4', name: 'Terminé' },
      ],
      cards: [
        {
          id: 'c151',
          idShort: 151,
          name: "Exporter le journal d'audit en CSV",
          desc: 'Les administrateurs exportent le journal.',
          idList: 'l1',
          labels: [{ name: 'claude-ready' }],
          idMembers: ['m1'],
          checklists: [{ name: "Critères d'acceptation", items: ['Filtre par date', 'Colonnes horodatage, IP, résultat'] }],
        },
        {
          id: 'c156',
          idShort: 156,
          name: 'Email de bienvenue',
          desc: '## Critères\n- Template HTML\n- Envoi à la création du compte',
          idList: 'l1',
          labels: [],
          idMembers: [],
          checklists: [],
        },
      ],
    },
    github: {
      labels: ['in-progress', 'à tester', 'claude-ready', 'bug'],
      issues: [
        {
          number: 42,
          title: 'Mode sombre du site',
          body: '- [ ] Bascule persistée\n- [ ] Contrastes AA',
          state: 'open',
          labels: ['claude-ready'],
          assignee: 'ada',
        },
        {
          number: 43,
          title: 'Lien cassé dans le footer',
          body: 'Le lien « Contact » mène à une 404.',
          state: 'open',
          labels: ['bug'],
          assignee: null,
        },
      ],
      // The pull requests opened by the app (ESC-64), from #101: open, their CI running, until POST
      // /__github/pulls/<n> sets what a test wants of them ({ checks: 'FAILURE', changesRequested: true, … }), the
      // repository it is in (`repo`, acme/demo by default) and the one of its branch (`headRepo`: a fork's, ESC-238).
      pulls: [],
      // The forks and the repository each was forked from (`repository.parent`).
      forks: { 'ada/demo': 'acme/demo', 'bob/demo': 'acme/demo' },
    },
    // Azure DevOps (ESC-241): the pull requests opened by the app on acme/demo/demo, from #101, until POST
    // /__azure/pulls/<n> sets what a test wants of them ({ checks: 'FAILURE', changesRequested: true, … }).
    // Azure Boards (ESC-242): the work items of the project demo (process Agile), their states changed by PATCH.
    azure: {
      pulls: [],
      items: [
        {
          id: 101,
          type: 'User Story',
          title: 'Factures en PDF',
          state: 'New',
          tags: 'facturation',
          assigned: 'Ada Lovelace',
          description: '<div>Exporter les factures du mois.</div>',
          acceptance: '<ul><li>50 factures par page</li><li>Un PDF par facture</li></ul>',
        },
        {
          id: 102,
          type: 'Bug',
          title: 'Le panier se vide',
          state: 'Active',
          tags: '',
          assigned: null,
          reproSteps: '<p>Ajouter un article, recharger la page.</p>',
        },
        { id: 103, type: 'Task', title: 'Ancienne tâche', state: 'Closed', tags: '', assigned: 'Ada Lovelace' },
      ],
    },
    // ClickUp: the workspace Atlas, its space Produit, the list « Sprint 12 » in the folder Sprints
    // (its own statuses) and the list « Backlog » outside any folder.
    clickup: {
      statuses: [
        { id: 'sc_open', status: 'to do', type: 'open', orderindex: 0 },
        { id: 'sc_doing', status: 'in progress', type: 'custom', orderindex: 1 },
        { id: 'sc_review', status: 'review', type: 'custom', orderindex: 2 },
        { id: 'sc_closed', status: 'complete', type: 'closed', orderindex: 3 },
      ],
      tasks: [
        {
          id: '86abx3',
          custom_id: 'ABC-12',
          name: "Exporter le journal d'audit en CSV",
          markdown_description: 'Les administrateurs exportent le journal.',
          status: 'to do',
          tags: ['claude-ready'],
          assignees: [183],
          checklists: [{ name: "Critères d'acceptation", items: ['Filtre par date', 'Colonnes horodatage, IP, résultat'] }],
        },
        {
          id: '86abx4',
          custom_id: 'ABC-13',
          name: 'Page de connexion plus rapide',
          markdown_description: '## Critères\n- Moins de 1 s sur 4G',
          status: 'in progress',
          tags: [],
          assignees: [],
          checklists: [],
        },
        {
          id: '86abx5',
          custom_id: 'ABC-9',
          name: 'Ancienne tâche finie',
          markdown_description: '',
          status: 'complete',
          tags: [],
          assignees: [183],
          checklists: [],
        },
      ],
    },
    // GitLab: the project acme/api of the instance, its labels, a board's lists, and its issues.
    gitlab: {
      projects: ['acme/api', 'acme/web/app'],
      labels: ['bug', 'claude-ready', 'Doing', 'To review'],
      lists: ['Doing', 'To review'],
      issues: [
        {
          iid: 7,
          title: 'Paginer la liste des factures',
          description:
            'La liste charge tout d’un coup.\n\n## Critères d’acceptation\n- [ ] 50 factures par page\n- [ ] Un lien vers la page suivante',
          state: 'opened',
          labels: ['claude-ready'],
          author: 'grace',
          assignee: null,
        },
        {
          iid: 8,
          title: 'Exporter les factures en PDF',
          description: '',
          state: 'opened',
          labels: ['bug'],
          author: 'ada',
          assignee: 'ada',
        },
        { iid: 3, title: 'Ancienne issue fermée', description: '', state: 'closed', labels: [], author: 'ada', assignee: null },
      ],
      notes: [],
    },
    writes: [],
  };
}

const JIRA_TYPES = { Story: '10001', Bug: '10004' };

/** `adjust` changes the data it starts from (the website's screenshots give their own issues). */
export function startFakeTrackers(port = 0, adjust = (db) => db) {
  let db = adjust(seed());
  const server = http.createServer((req, res) => {
    let body = '';
    req.on('data', (d) => (body += d));
    req.on('end', () => {
      const url = new URL(req.url, 'http://x');
      const send = (status, value) => {
        const text = value === undefined ? '' : JSON.stringify(value);
        res.writeHead(status, { 'Content-Type': 'application/json' });
        res.end(text);
      };
      // A text as it is (a job's log).
      send.text = (status, text) => {
        res.writeHead(status, { 'Content-Type': 'text/plain' });
        res.end(text);
      };
      // With headers of its own (GitLab's pages).
      send.with = (status, value, headers) => {
        res.writeHead(status, { 'Content-Type': 'application/json', ...headers });
        res.end(JSON.stringify(value));
      };
      const json = () => {
        try {
          return JSON.parse(body || 'null');
        } catch {
          return null;
        }
      };
      // Jira's search is a POST that changes nothing.
      if (req.method !== 'GET' && !url.pathname.startsWith('/__') && url.pathname !== '/rest/api/3/search/jql') {
        db.writes.push({ method: req.method, path: url.pathname, query: Object.fromEntries(url.searchParams), body: json() });
      }
      try {
        route(req.method, url, json, send, req.headers);
      } catch (e) {
        send(500, { message: String(e) });
      }
    });
  });

  /** The token "bad", however it is sent, is refused. */
  const bad = (headers, url) => {
    const auth = headers.authorization ?? '';
    const basic = auth.startsWith('Basic ') ? Buffer.from(auth.slice(6), 'base64').toString().split(':').pop() : null;
    return (
      basic === 'bad' ||
      auth === 'Bearer bad' ||
      auth === 'bad' ||
      headers['private-token'] === 'bad' ||
      url.searchParams.get('token') === 'bad'
    );
  };

  function route(method, url, json, send, headers) {
    const p = url.pathname;
    if (p === '/__writes') return send(200, db.writes);
    if (p === '/__reset') {
      db = seed();
      return send(200, {});
    }
    if (bad(headers, url))
      return send(
        401,
        p.startsWith('/1/')
          ? 'invalid token'
          : p.startsWith('/clickup/')
            ? { err: 'Token invalid', ECODE: 'OAUTH_025' }
            : { message: 'Bad credentials', errorMessages: [] },
      );

    // ---------- GitLab (a self-hosted instance: its API at the root) ----------
    if (p === '/api/v4/version') return send(headers['private-token'] ? 200 : 401, { message: '401 Unauthorized' });
    if (p === '/api/v4/user') return send(200, { username: 'ada' });
    // A token that may only read: the API right is missing.
    if (p === '/api/v4/personal_access_tokens/self') return send(200, { scopes: ['read_api', 'read_user'] });
    // GitLab Issues: its projects (their path encoded as one segment), labels, boards and issues.
    const gl = db.gitlab;
    const glIssue = (i, path) => ({
      iid: i.iid,
      title: i.title,
      description: i.description,
      state: i.state,
      labels: i.labels,
      author: { username: i.author },
      assignees: i.assignee ? [{ username: i.assignee }] : [],
      issue_type: 'issue',
      web_url: `https://gitlab.acme.dev/${path}/-/issues/${i.iid}`,
    });
    if (p === '/api/v4/projects')
      return send(
        200,
        gl.projects.map((path, i) => ({ id: i + 1, path_with_namespace: path })),
      );
    let g4 = p.match(/^\/api\/v4\/projects\/([^/]+)(\/.*)?$/);
    if (g4) {
      const path = decodeURIComponent(g4[1]);
      const r = g4[2] ?? '';
      if (!gl.projects.includes(path)) return send(404, { message: '404 Project Not Found' });
      if (r === '/labels')
        return send(
          200,
          gl.labels.map((name, id) => ({ id, name })),
        );
      if (r === '/boards')
        return send(200, [
          { id: 1, name: 'Development', lists: gl.lists.map((name, position) => ({ id: position, position, label: { name } })) },
        ]);
      if (r === '/issues') {
        const q = (k) => url.searchParams.get(k);
        const text = (q('search') ?? '').toLowerCase();
        const all = gl.issues
          .filter((i) => !q('state') || i.state === q('state'))
          .filter((i) => !q('assignee_username') || i.assignee === q('assignee_username'))
          .filter(
            (i) =>
              !q('labels') ||
              q('labels')
                .split(',')
                .every((l) => i.labels.includes(l)),
          )
          .filter((i) => !text || `${i.title} ${i.description}`.toLowerCase().includes(text));
        const per = Number(q('per_page') ?? 20);
        const page = Number(q('page') ?? 1);
        const items = all.slice((page - 1) * per, page * per);
        const next = page * per < all.length ? String(page + 1) : '';
        return send.with(
          200,
          items.map((i) => glIssue(i, path)),
          { 'X-Total': String(all.length), 'X-Next-Page': next, 'X-Page': String(page) },
        );
      }
      g4 = r.match(/^\/issues\/(\d+)(\/notes)?$/);
      if (g4) {
        const issue = gl.issues.find((i) => i.iid === Number(g4[1]));
        if (!issue) return send(404, { message: '404 Issue Not Found' });
        if (g4[2]) {
          gl.notes.push({ iid: issue.iid, body: json()?.body });
          return send(201, { id: gl.notes.length, body: json()?.body });
        }
        if (method === 'PUT') {
          const b = json() ?? {};
          if (b.state_event === 'close') issue.state = 'closed';
          if (b.state_event === 'reopen') issue.state = 'opened';
          const off = (b.remove_labels ?? '').split(',').filter(Boolean);
          issue.labels = issue.labels.filter((l) => !off.includes(l));
          for (const l of (b.add_labels ?? '').split(',').filter(Boolean)) if (!issue.labels.includes(l)) issue.labels.push(l);
        }
        return send(200, glIssue(issue, path));
      }
      return send(404, { message: `404 Not Found: ${method} ${p}` });
    }

    // ---------- Jira ----------
    const j = db.jira;
    const status = (id) => j.statuses.find((s) => s.id === id);
    const issueOf = (key) => j.issues.find((i) => i.key === key);
    if (p === '/rest/api/3/myself') return send(200, { accountId: 'acc-ada', emailAddress: 'ada@atlas.dev', displayName: 'Ada Lovelace' });
    if (p === '/rest/api/3/project/search')
      return send(200, {
        isLast: true,
        values: [
          { key: 'ATL', name: 'Atlas' },
          { key: 'MOB', name: 'Mobile' },
        ],
      });
    let m = p.match(/^\/rest\/api\/3\/project\/([^/]+)\/statuses$/);
    if (m) return send(200, [{ name: 'Story', statuses: j.statuses }]);
    if (p === '/rest/api/3/search/jql' && method === 'POST') {
      const jql = json()?.jql ?? '';
      const label = jql.match(/labels = "([^"]+)"/)?.[1];
      const key = jql.match(/key = "([^"]+)"/)?.[1];
      const text = jql.match(/text ~ "([^"]+)"/)?.[1]?.toLowerCase();
      const issues = j.issues
        .filter((i) => i.key.startsWith('ATL-') && jql.includes('project = "ATL"'))
        .filter((i) => status(i.status).name !== 'Done')
        .filter((i) => !jql.includes('assignee = currentUser()') || i.assignee === 'Ada Lovelace')
        .filter((i) => !label || i.labels.includes(label))
        .filter((i) => !key || i.key === key)
        .filter((i) => !text || i.summary.toLowerCase().includes(text))
        .map((i) => ({
          id: i.key,
          key: i.key,
          fields: {
            summary: i.summary,
            issuetype: { id: JIRA_TYPES[i.type], name: i.type },
            priority: { name: i.priority },
            assignee: i.assignee ? { displayName: i.assignee } : null,
            status: status(i.status),
            labels: i.labels,
            description: i.description,
          },
        }));
      return send(200, { issues });
    }
    m = p.match(/^\/rest\/api\/3\/issue\/([^/]+)(\/transitions|\/comment)?$/);
    if (m) {
      const issue = issueOf(decodeURIComponent(m[1]));
      if (!issue) return send(404, { errorMessages: ["Le ticket n'existe pas."] });
      if (!m[2]) return send(200, { key: issue.key, fields: { status: status(issue.status) } });
      if (m[2] === '/comment') return send(201, { id: String(db.writes.length) });
      if (method === 'GET') return send(200, { transitions: j.statuses.map((s) => ({ id: `t${s.id}`, name: s.name, to: s })) });
      const to = json()?.transition?.id?.replace(/^t/, '');
      if (!status(to)) return send(400, { errorMessages: ['Transition inconnue.'] });
      issue.status = to;
      return send(204);
    }

    // ---------- Trello ----------
    const t = db.trello;
    if (p === '/1/members/me') return send(200, { id: 'm1', username: 'ada', fullName: 'Ada Lovelace' });
    if (p === '/1/members/me/boards') return send(200, [{ id: 'b1', name: 'Atlas — Backlog produit' }]);
    if (p === '/1/boards/b1/lists') return send(200, t.lists);
    if (p === '/1/boards/b1/cards/open')
      return send(
        200,
        t.cards.map(({ checklists, ...c }) => ({ ...c, shortUrl: `https://trello.com/c/${c.id}` })),
      );
    if (p === '/1/boards/b1/checklists')
      return send(
        200,
        t.cards.flatMap((c) =>
          c.checklists.map((l, i) => ({ id: `${c.id}-${i}`, idCard: c.id, name: l.name, checkItems: l.items.map((name) => ({ name })) })),
        ),
      );
    m = p.match(/^\/1\/cards\/([^/]+)(\/actions\/comments)?$/);
    if (m) {
      const card = t.cards.find((c) => c.id === m[1]);
      if (!card) return send(404, 'model not found');
      if (m[2]) return send(200, { id: String(db.writes.length) });
      const list = url.searchParams.get('idList');
      if (list) card.idList = list;
      return send(200, { id: card.id, idList: card.idList });
    }

    // ---------- ClickUp (API v2) ----------
    const cu = db.clickup;
    const cuStatus = (name) => cu.statuses.find((s) => s.status === name);
    const cuTask = (t) => ({
      id: t.id,
      custom_id: t.custom_id,
      name: t.name,
      markdown_description: t.markdown_description,
      status: { status: t.status, type: cuStatus(t.status)?.type ?? 'open' },
      priority: null,
      assignees: t.assignees.map((id) => ({ id, username: 'Ada Lovelace' })),
      tags: t.tags.map((name) => ({ name })),
      checklists: t.checklists.map((c, i) => ({
        id: `${t.id}-${i}`,
        name: c.name,
        items: c.items.map((name, orderindex) => ({ id: `${t.id}-${i}-${orderindex}`, name, orderindex, resolved: false })),
      })),
      url: `https://app.clickup.com/t/${t.id}`,
      list: { id: 'l1' },
    });
    if (p.startsWith('/clickup/api/v2/')) {
      const r = p.slice('/clickup/api/v2'.length);
      if (r === '/user') return send(200, { user: { id: 183, username: 'Ada Lovelace', email: 'ada@atlas.dev' } });
      if (r === '/team') return send(200, { teams: [{ id: '9001', name: 'Atlas' }] });
      if (r === '/team/9001/space') return send(200, { spaces: [{ id: '790', name: 'Produit' }] });
      if (r === '/space/790/folder')
        return send(200, { folders: [{ id: '457', name: 'Sprints', lists: [{ id: 'l1', name: 'Sprint 12' }] }] });
      if (r === '/space/790/list') return send(200, { lists: [{ id: 'l0', name: 'Backlog' }] });
      if (r === '/list/l1') return send(200, { id: 'l1', name: 'Sprint 12', statuses: cu.statuses });
      if (r === '/list/l1/task') {
        const all = (k) => url.searchParams.getAll(k);
        const tasks = cu.tasks
          .filter((t) => !['closed', 'done'].includes(cuStatus(t.status)?.type))
          .filter((t) => !all('assignees[]').length || t.assignees.some((a) => all('assignees[]').includes(String(a))))
          .filter((t) => !all('statuses[]').length || all('statuses[]').includes(t.status))
          .filter((t) => !all('tags[]').length || t.tags.some((g) => all('tags[]').includes(g)))
          .map(cuTask);
        return send(200, { tasks, last_page: true });
      }
      m = r.match(/^\/list\/([^/]+)/);
      if (m) return send(404, { err: 'List not found', ECODE: 'ITEM_013' });
      m = r.match(/^\/task\/([^/]+)(\/comment)?$/);
      if (m) {
        const id = decodeURIComponent(m[1]);
        const task = cu.tasks.find((t) => (url.searchParams.get('custom_task_ids') === 'true' ? t.custom_id === id : t.id === id));
        if (!task) return send(404, { err: 'Task not found', ECODE: 'ITEM_013' });
        if (m[2]) return send(200, { id: db.writes.length, hist_id: String(db.writes.length), date: Date.now() });
        if (method === 'PUT') {
          const to = json()?.status;
          if (!cuStatus(to)) return send(400, { err: 'Status does not exist', ECODE: 'ITEM_062' });
          task.status = to;
        }
        return send(200, cuTask(task));
      }
      return send(404, { err: `Route not found: ${method} ${r}`, ECODE: 'APP_001' });
    }

    // ---------- Azure DevOps (ESC-241): the organization acme, its project and repository demo ----------
    const az = db.azure;
    m = p.match(/^\/__azure\/pulls\/(\d+)$/);
    if (m) {
      const pull = az.pulls.find((x) => x.id === Number(m[1]));
      if (!pull) return send(404, { message: 'TF401180: The requested pull request was not found.' });
      Object.assign(pull, json() ?? {});
      return send(200, pull);
    }
    if (p.startsWith('/azure/')) {
      const a = p.slice('/azure'.length).toLowerCase();
      const web = 'https://dev.azure.com/acme/demo/_git/demo';
      const azPull = (x) => ({
        pullRequestId: x.id,
        status: x.status,
        isDraft: x.draft,
        title: x.title,
        description: x.description,
        sourceRefName: `refs/heads/${x.head}`,
        targetRefName: `refs/heads/${x.base}`,
        mergeStatus: 'succeeded',
        lastMergeSourceCommit: { commitId: `sha${x.id}` },
        reviewers: x.changesRequested ? [{ id: 'bob-id', displayName: 'Bob Martin', uniqueName: 'bob@acme.dev', vote: -5 }] : [],
        autoCompleteSetBy: null,
        repository: { id: 'repo-demo', name: 'demo', webUrl: web, project: { id: 'project-demo', name: 'demo' } },
      });
      if (a === '/acme/_apis/connectiondata')
        return send(200, {
          authenticatedUser: {
            id: 'ada-id',
            descriptor: 'Microsoft.IdentityModel.Claims.ClaimsIdentity;acme\\ada@acme.dev',
            providerDisplayName: 'Ada Lovelace',
            properties: { Account: { $type: 'System.String', $value: 'ada@acme.dev' } },
          },
        });
      const repo = '/acme/demo/_apis/git/repositories/demo';
      if (a === `${repo}/pullrequests` && method === 'POST') {
        const b = json();
        const pull = {
          id: 101 + az.pulls.length,
          title: b.title,
          description: b.description,
          head: b.sourceRefName.replace('refs/heads/', ''),
          base: b.targetRefName.replace('refs/heads/', ''),
          draft: !!b.isDraft,
          status: 'active',
          checks: 'PENDING',
          changesRequested: false,
        };
        az.pulls.push(pull);
        return send(201, azPull(pull));
      }
      if (a === `${repo}/pullrequests`) {
        const ref = url.searchParams.get('searchCriteria.sourceRefName');
        return send(200, { value: az.pulls.filter((x) => `refs/heads/${x.head}` === ref).map(azPull) });
      }
      m = a.match(/^\/acme\/demo\/_apis\/git\/repositories\/demo\/pullrequests\/(\d+)(\/threads|\/statuses)?$/);
      if (m) {
        const pull = az.pulls.find((x) => x.id === Number(m[1]));
        if (!pull) return send(404, { message: 'TF401180: The requested pull request was not found.' });
        if (m[2] === '/statuses') return send(200, { value: [] });
        if (m[2] === '/threads') {
          const thread = {
            id: 1,
            status: 'active',
            threadContext: { filePath: '/src/app.ts', rightFileStart: { line: 1, offset: 1 } },
            comments: [{ id: 1, author: { displayName: 'Bob Martin' }, content: 'Ce nom est peu clair.', commentType: 'text' }],
          };
          return send(200, { value: pull.changesRequested ? [thread] : [] });
        }
        return send(200, azPull(pull));
      }
      if (a === '/acme/demo/_apis/policy/evaluations') {
        const n = Number(url.searchParams.get('artifactId')?.split('/').pop());
        const pull = az.pulls.find((x) => x.id === n);
        const status = { FAILURE: 'rejected', SUCCESS: 'approved' }[pull?.checks] ?? 'running';
        return send(200, {
          value: [
            {
              configuration: {
                type: { id: '0609b952-1397-4640-95ec-e00a01b2c241', displayName: 'Build' },
                settings: { displayName: 'test', buildDefinitionId: 3 },
              },
              status,
              context: { buildId: 77 },
            },
          ],
        });
      }
      if (a === '/acme/demo/_apis/build/builds') return send(200, { value: [] });
      if (a === '/acme/demo/_apis/build/builds/77/timeline')
        return send(200, { records: [{ type: 'Task', name: 'npm test', result: 'failed', log: { id: 5 } }] });
      if (a === '/acme/demo/_apis/build/builds/77/logs/5') return send.text(200, 'npm test\nFAIL src/app.test.ts\n  expected 2, got 1\n');

      // ---------- Azure Boards (ESC-242): the projects of acme, the work items of demo ----------
      if (a === '/acme/_apis/projects')
        return send(200, {
          count: 2,
          value: [
            { id: 'project-demo', name: 'demo', state: 'wellFormed' },
            { id: 'project-mobile', name: 'Mobile', state: 'wellFormed' },
          ],
        });
      const st = (name, category) => ({ name, color: '007acc', category });
      const agile = [
        st('New', 'Proposed'),
        st('Active', 'InProgress'),
        st('Resolved', 'Resolved'),
        st('Closed', 'Completed'),
        st('Removed', 'Removed'),
      ];
      if (a === '/acme/demo/_apis/wit/workitemtypes')
        return send(200, {
          count: 3,
          value: ['User Story', 'Bug', 'Task'].map((name) => ({ name, isDisabled: false, states: agile })),
        });
      if (a === '/acme/demo/_apis/wit/tags') return send(200, { value: [{ id: 't1', name: 'facturation' }] });
      const person = (name) => (name ? { displayName: name, uniqueName: 'ada@acme.dev' } : undefined);
      const azItem = (i) => ({
        id: i.id,
        rev: 1,
        fields: {
          'System.Id': i.id,
          'System.WorkItemType': i.type,
          'System.State': i.state,
          'System.Title': i.title,
          'System.Tags': i.tags,
          'System.AssignedTo': person(i.assigned),
          'System.CreatedBy': person('Grace Hopper'),
          'System.Description': i.description,
          'Microsoft.VSTS.TCM.ReproSteps': i.reproSteps,
          'Microsoft.VSTS.Common.AcceptanceCriteria': i.acceptance,
        },
      });
      if (a === '/acme/demo/_apis/wit/wiql' && method === 'POST') {
        const q = json()?.query ?? '';
        const closed = (q.match(/NOT IN \(([^)]*)\)/)?.[1] ?? '').split(', ');
        const types = q.match(/\[System\.WorkItemType\] IN \(([^)]*)\)/)?.[1].split(', ');
        const id = q.match(/\[System\.Id\] = (\d+)/)?.[1];
        const title = q
          .match(/\[System\.Title\] CONTAINS '((?:[^']|'')*)'/)?.[1]
          .replaceAll("''", "'")
          .toLowerCase();
        const found = az.items.filter(
          (i) =>
            !closed.includes(`'${i.state}'`) &&
            (!types || types.includes(`'${i.type}'`)) &&
            (!q.includes('@Me') || i.assigned === 'Ada Lovelace') &&
            (!id || i.id === Number(id)) &&
            (!title || i.title.toLowerCase().includes(title)),
        );
        return send(200, { queryType: 'flat', workItems: found.map((i) => ({ id: i.id })) });
      }
      if (a === '/acme/demo/_apis/wit/workitems') {
        const ids = (url.searchParams.get('ids') ?? '').split(',').map(Number);
        return send(200, { value: az.items.filter((i) => ids.includes(i.id)).map(azItem) });
      }
      m = a.match(/^\/acme\/demo\/_apis\/wit\/workitems\/(\d+)(\/comments)?$/);
      if (m) {
        const item = az.items.find((i) => i.id === Number(m[1]));
        if (!item) return send(404, { message: `TF401232: Work item ${m[1]} does not exist.` });
        if (m[2]) return send(200, { workItemId: item.id, commentId: db.writes.length, text: json()?.text });
        if (method === 'PATCH') {
          for (const op of json() ?? []) if (op.path === '/fields/System.State') item.state = op.value;
        }
        return send(200, azItem(item));
      }
      return send(404, { message: `Not Found: ${method} ${p}` });
    }

    // ---------- GitHub ----------
    const g = db.github;
    const ghIssue = (i) => ({
      number: i.number,
      title: i.title,
      body: i.body,
      state: i.state,
      html_url: `https://github.com/acme/demo/issues/${i.number}`,
      labels: i.labels.map((name) => ({ name })),
      assignee: i.assignee ? { login: i.assignee } : null,
    });
    // ---------- GitHub's pull requests (ESC-64) ----------
    m = p.match(/^\/__github\/pulls\/(\d+)$/);
    if (m) {
      const pull = g.pulls.find((x) => x.number === Number(m[1]));
      if (!pull) return send(404, { message: 'Not Found' });
      Object.assign(pull, json() ?? {});
      return send(200, pull);
    }
    const node = (x) => ({
      number: x.number,
      url: `https://github.com/${x.repo ?? 'acme/demo'}/pull/${x.number}`,
      title: x.title,
      state: x.state,
      isDraft: x.draft,
      headRefName: x.head,
      baseRefName: x.base,
      mergeable: 'MERGEABLE',
      isCrossRepository: (x.headRepo ?? x.repo ?? 'acme/demo') !== (x.repo ?? 'acme/demo'),
      headRepository: { nameWithOwner: x.headRepo ?? x.repo ?? 'acme/demo' },
      autoMergeRequest: null,
      author: { login: x.author ?? 'ada' },
      createdAt: x.createdAt ?? new Date(Date.now() - 2 * 3600_000).toISOString(),
      latestOpinionatedReviews: { nodes: x.changesRequested ? [{ state: 'CHANGES_REQUESTED' }] : [] },
      reviewThreads: { nodes: x.changesRequested ? [{ isResolved: false }] : [] },
      commits: { nodes: [{ commit: { statusCheckRollup: { state: x.checks } } }] },
    });
    if (p === '/github/graphql') {
      const q = json() ?? {};
      if (q.operationName === 'PullsOfBranch')
        return send(200, {
          data: {
            repository: {
              pullRequests: {
                nodes: g.pulls
                  .filter((x) => x.head === q.variables.branch && (x.repo ?? 'acme/demo') === `${q.variables.owner}/${q.variables.name}`)
                  .reverse()
                  .map(node),
              },
            },
          },
        });
      // The project's open pull requests (ESC-305), the most recent first.
      if (q.operationName === 'OpenPulls')
        return send(200, {
          data: {
            repository: {
              pullRequests: {
                nodes: g.pulls
                  .filter((x) => x.state === 'OPEN' && (x.repo ?? 'acme/demo') === `${q.variables.owner}/${q.variables.name}`)
                  .reverse()
                  .map(node),
              },
            },
          },
        });
      if (q.operationName === 'RepoParent') {
        const parent = g.forks[`${q.variables.owner}/${q.variables.name}`];
        const [owner, name] = parent ? parent.split('/') : [];
        return send(200, { data: { repository: { parent: parent ? { name, owner: { login: owner } } : null } } });
      }
      if (q.operationName === 'Threads') {
        const x = g.pulls.find((y) => y.number === q.variables.number);
        const thread = {
          isResolved: false,
          path: 'src/app.ts',
          line: 1,
          comments: { nodes: [{ databaseId: 1, author: { login: 'bob' }, body: 'Ce nom est peu clair.' }] },
        };
        return send(200, { data: { repository: { pullRequest: { reviewThreads: { nodes: x?.changesRequested ? [thread] : [] } } } } });
      }
      return send(200, { data: null, errors: [{ message: `unknown operation ${q.operationName}` }] });
    }
    if (p === '/github/repos/acme/demo/pulls' && method === 'POST') {
      const b = json();
      const pull = {
        number: 101 + g.pulls.length,
        title: b.title,
        body: b.body,
        head: b.head,
        base: b.base,
        draft: !!b.draft,
        state: 'OPEN',
        checks: 'PENDING',
        changesRequested: false,
      };
      g.pulls.push(pull);
      return send(201, {
        number: pull.number,
        html_url: `https://github.com/acme/demo/pull/${pull.number}`,
        title: pull.title,
        state: 'open',
        draft: pull.draft,
        head: { ref: pull.head },
        base: { ref: pull.base },
      });
    }
    m = p.match(/^\/github\/repos\/acme\/demo\/pulls\/(\d+)(\/reviews)?$/);
    if (m) {
      const pull = g.pulls.find((x) => x.number === Number(m[1]));
      if (!pull) return send(404, { message: 'Not Found' });
      if (m[2])
        return send(200, pull.changesRequested ? [{ user: { login: 'bob' }, state: 'CHANGES_REQUESTED', body: 'Ajoute un test.' }] : []);
      return send(200, { number: pull.number, head: { sha: `sha${pull.number}`, ref: pull.head } });
    }
    m = p.match(/^\/github\/repos\/acme\/demo\/commits\/sha(\d+)\/(check-runs|status)$/);
    if (m) {
      const pull = g.pulls.find((x) => x.number === Number(m[1]));
      if (m[2] === 'status') return send(200, { statuses: [] });
      const failed = pull?.checks === 'FAILURE';
      const running = pull?.checks === 'PENDING';
      // When they ran (ESC-305): `test` 2 min 30 s, `lint` 12 s.
      const at = (s) => new Date(Date.parse('2026-10-30T10:00:00Z') + s * 1000).toISOString();
      return send(200, {
        check_runs: [
          {
            id: 77,
            name: 'test',
            status: running ? 'in_progress' : 'completed',
            conclusion: running ? null : failed ? 'failure' : 'success',
            started_at: at(0),
            completed_at: running ? null : at(150),
            html_url: 'https://github.com/acme/demo/actions/runs/7/job/77',
            app: { slug: 'github-actions' },
          },
          {
            id: 78,
            name: 'lint',
            status: 'completed',
            conclusion: 'success',
            started_at: at(0),
            completed_at: at(12),
            html_url: 'https://github.com/acme/demo/actions/runs/7/job/78',
            app: { slug: 'github-actions' },
          },
        ],
      });
    }
    if (p === '/github/repos/acme/demo/actions/jobs/77/logs')
      return send.text(200, 'npm test\nFAIL src/app.test.ts\n  expected 2, got 1\n');
    // « Relancer les échecs » (ESC-305): the run of the failed job, run again.
    m = p.match(/^\/github\/repos\/acme\/demo\/actions\/jobs\/(\d+)$/);
    if (m) return send(200, { id: Number(m[1]), run_id: 7 });
    if (p === '/github/repos/acme/demo/actions/runs/7/rerun-failed-jobs' && method === 'POST') return send(201, {});
    // The deployments of a branch (ESC-305): a preview of each pull request's.
    if (p === '/github/repos/acme/demo/deployments') {
      const ref = url.searchParams.get('ref');
      const pull = g.pulls.find((x) => x.head === ref);
      return send(200, pull ? [{ id: pull.number, environment: 'Preview' }] : []);
    }
    m = p.match(/^\/github\/repos\/acme\/demo\/deployments\/(\d+)\/statuses$/);
    if (m) return send(200, [{ state: 'success', environment_url: `https://demo-pr-${m[1]}.vercel.app` }]);
    m = p.match(/^\/github\/repos\/acme\/demo\/issues\/(\d+)\/comments$/);
    if (m && method === 'GET' && g.pulls.some((x) => x.number === Number(m[1]))) return send(200, []);

    if (p === '/github/user') return send(200, { login: 'ada' });
    if (p === '/github/user/repos') return send(200, [{ full_name: 'acme/demo' }, { full_name: 'ada/blog' }]);
    if (p === '/github/repos/acme/demo/labels')
      return send(
        200,
        g.labels.map((name) => ({ name })),
      );
    if (p === '/github/repos/acme/demo/issues') return send(200, g.issues.filter((i) => i.state === 'open').map(ghIssue));
    if (p === '/github/search/issues') {
      const q = url.searchParams.get('q') ?? '';
      const label = q.match(/label:"([^"]+)"/)?.[1];
      const words = q
        .replace(/repo:\S+|is:\S+|assignee:\S+|label:"[^"]+"/g, '')
        .trim()
        .toLowerCase();
      const items = g.issues
        .filter((i) => i.state === 'open')
        .filter((i) => !q.includes('assignee:@me') || i.assignee === 'ada')
        .filter((i) => !label || i.labels.includes(label))
        .filter((i) => !words || i.title.toLowerCase().includes(words))
        .map(ghIssue);
      return send(200, { items });
    }
    m = p.match(/^\/github\/repos\/acme\/demo\/issues\/(\d+)(\/labels(?:\/(.+))?|\/comments)?$/);
    if (m) {
      const issue = g.issues.find((i) => i.number === Number(m[1]));
      if (!issue) return send(404, { message: 'Not Found' });
      if (m[2] === '/comments') return send(201, { id: db.writes.length });
      if (m[2]?.startsWith('/labels')) {
        if (method === 'DELETE') issue.labels = issue.labels.filter((l) => l !== decodeURIComponent(m[3]));
        else issue.labels = [...new Set([...issue.labels, ...(json()?.labels ?? [])])];
        return send(
          200,
          issue.labels.map((name) => ({ name })),
        );
      }
      if (method === 'PATCH') issue.state = json()?.state ?? issue.state;
      return send(200, ghIssue(issue));
    }
    send(404, { message: `Not Found: ${method} ${p}` });
  }

  return new Promise((resolve) =>
    server.listen(port, '127.0.0.1', () => {
      const url = `http://127.0.0.1:${server.address().port}`;
      resolve({ url, close: () => new Promise((r) => server.close(r)) });
    }),
  );
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  const i = process.argv.indexOf('--port');
  const port = Number(i > 0 ? process.argv[i + 1] : process.env.PORT || 4101);
  startFakeTrackers(port).then(({ url }) => {
    console.log(`fake Jira, Trello, GitHub, ClickUp, GitLab and Azure DevOps on ${url}`);
    console.log(
      `  Jira : site ${url} · Trello : ESCOUADE_TRELLO_API=${url}/1 · GitHub : ESCOUADE_GITHUB_API=${url}/github · ClickUp : ESCOUADE_CLICKUP_API=${url}/clickup/api/v2 · GitLab : hébergeur GitLab à ${url} · Azure DevOps : ESCOUADE_AZURE_DEVOPS_API=${url}/azure`,
    );
  });
}

#!/usr/bin/env node
// Test double of Jira Cloud, Trello and GitHub Issues on one local HTTP server, for the end-to-end
// tests and to try the integrations without real accounts:
//   node tests/fixtures/fake-trackers.mjs --port 4101
// then, in Escouade: Jira's site = http://127.0.0.1:4101 (any e-mail, any token but "bad"), and the
// app started with ESCOUADE_TRELLO_API=http://127.0.0.1:4101/1 and
// ESCOUADE_GITHUB_API=http://127.0.0.1:4101/github (any key or token but "bad").
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
    },
    writes: [],
  };
}

const JIRA_TYPES = { Story: '10001', Bug: '10004' };

export function startFakeTrackers(port = 0) {
  let db = seed();
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
    return basic === 'bad' || auth === 'Bearer bad' || url.searchParams.get('token') === 'bad';
  };

  function route(method, url, json, send, headers) {
    const p = url.pathname;
    if (p === '/__writes') return send(200, db.writes);
    if (p === '/__reset') {
      db = seed();
      return send(200, {});
    }
    if (bad(headers, url)) return send(401, p.startsWith('/1/') ? 'invalid token' : { message: 'Bad credentials', errorMessages: [] });

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
    console.log(`fake Jira, Trello and GitHub on ${url}`);
    console.log(`  Jira : site ${url} · Trello : ESCOUADE_TRELLO_API=${url}/1 · GitHub : ESCOUADE_GITHUB_API=${url}/github`);
  });
}

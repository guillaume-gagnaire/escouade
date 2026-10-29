// A fake GitHub for the store's end-to-end tests, and a fake SMTP server: never the real App (docs/STORE.md, §1.5).
// It answers what the store asks of GitHub (the authorization page, the code's exchange, /user, the installations
// and their repositories, the token's revocation), installs the App as GitHub does (with or without the store's
// state), and signs the installation's webhook as GitHub does. The SMTP server keeps the mails it receives.
// The repositories it installs on hold test plugins, served as the App reads them (tests/fake/repos.mjs);
// `POST /_fake/tag` pushes a tag (a new one, or one moved; `extra` files, `manifest` fields) and sends its `create` webhook. What it saw is read at
// /_fake/state.
//
//   FAKE_GITHUB_PORT, FAKE_SMTP_PORT: its ports; STORE_URL: the store, for the webhooks;
//   GITHUB_APP_CLIENT_ID, GITHUB_APP_CLIENT_SECRET, GITHUB_WEBHOOK_SECRET: the App's, as the store has them.
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { createServer } from 'node:http';
import { SMTPServer } from 'smtp-server';
import { pushTag, repoOf, seedRepo, serveRepos } from './repos.mjs';

const port = Number(process.env.FAKE_GITHUB_PORT ?? 3010);
const smtpPort = Number(process.env.FAKE_SMTP_PORT ?? 3025);
const store = (process.env.STORE_URL ?? 'http://localhost:3000').replace(/\/$/, '');
const clientId = process.env.GITHUB_APP_CLIENT_ID ?? '';
const clientSecret = process.env.GITHUB_APP_CLIENT_SECRET ?? '';
const hookSecret = process.env.GITHUB_WEBHOOK_SECRET ?? '';

const users = new Map(); // login -> { id, login, name, avatar_url }
const codes = new Map(); // code -> login
const tokens = new Map(); // token -> login
const installations = new Map(); // id -> { id, account, repository_selection, repos: [{ id, full_name, private }] }
const revoked = [];
const mails = [];
const exchanged = [];
// The ids do not start over when the fake does: a base kept between runs (a developer's) must not hand an account
// of an earlier run to another login. A user's id comes from their login, the others from the clock.
let nextInstall = Number(String(Date.now()).slice(-8));
let nextRepo = nextInstall * 10;

function user(login) {
  if (!users.has(login)) {
    const id = 100_000 + (parseInt(createHash('sha256').update(login).digest('hex').slice(0, 8), 16) % 900_000_000);
    users.set(login, { id, login, name: login.replace(/-/g, ' '), avatar_url: `https://avatars.githubusercontent.com/u/${id}` });
  }
  return users.get(login);
}

const html = (res, body) => {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
  res.end(`<!doctype html><meta charset="utf-8"><title>Fake GitHub</title><body>${body}</body>`);
};
const json = (res, status, body) => {
  res.writeHead(status, { 'content-type': 'application/json' });
  res.end(body === undefined ? '' : JSON.stringify(body));
};
const redirect = (res, to) => {
  res.writeHead(302, { location: to });
  res.end();
};
const esc = (s) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
const hidden = (q, names) => names.map((n) => (q.has(n) ? `<input type="hidden" name="${n}" value="${esc(q.get(n))}">` : '')).join('');

async function body(req) {
  const chunks = [];
  for await (const c of req) chunks.push(c);
  return Buffer.concat(chunks).toString('utf8');
}

function back(redirectUri, params) {
  const url = new URL(redirectUri);
  for (const [k, v] of Object.entries(params)) if (v !== undefined && v !== null) url.searchParams.set(k, v);
  return url.toString();
}

async function sendWebhook(event, payload) {
  const raw = JSON.stringify(payload);
  const signature = `sha256=${createHmac('sha256', hookSecret).update(raw).digest('hex')}`;
  try {
    await fetch(`${store}/api/github/webhook`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-github-event': event,
        'x-github-delivery': randomUUID(),
        'x-hub-signature-256': signature,
      },
      body: raw,
    });
  } catch (e) {
    console.error('[fake github] webhook not delivered:', e.message);
  }
}

const bearer = (req) => tokens.get((req.headers.authorization ?? '').replace(/^Bearer /, '')) ?? null;
const callback = `${store}/api/auth/github/callback`;

const server = createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  const q = url.searchParams;
  const path = url.pathname;

  if (path === '/login/oauth/authorize' && req.method === 'GET') {
    if (q.get('client_id') !== clientId) return json(res, 400, { error: 'bad client_id' });
    return html(
      res,
      `<h1>Authorize Escouade Store</h1>
      <form action="/login/oauth/authorize/approve">${hidden(q, ['redirect_uri', 'state'])}
        <label>Login <input name="login" value="lea-martin"></label>
        <button type="submit">Authorize Escouade Store</button>
      </form>
      <a href="${esc(back(q.get('redirect_uri'), { error: 'access_denied', state: q.get('state') }))}">Cancel</a>`,
    );
  }
  if (path === '/login/oauth/authorize/approve') {
    const code = randomBytes(10).toString('hex');
    codes.set(code, user(q.get('login')).login);
    return redirect(res, back(q.get('redirect_uri'), { code, state: q.get('state') }));
  }
  if (path === '/apps/escouade-store/installations/new') {
    return html(
      res,
      `<h1>Install Escouade Store</h1>
      <form action="/apps/escouade-store/installations/approve">${hidden(q, ['state'])}
        <label>Login <input name="login" value="lea-martin"></label>
        <label>Repositories <input name="repos" value="escouade-slack"></label>
        <button type="submit">Install &amp; Authorize</button>
      </form>`,
    );
  }
  if (path === '/apps/escouade-store/installations/approve') {
    const who = user(q.get('login'));
    const repos = (q.get('repos') ?? '')
      .split(',')
      .map((r) => r.trim())
      .filter(Boolean)
      .map((r) => ({ id: nextRepo++, full_name: `${who.login}/${r}`, private: r.endsWith('-private') }));
    for (const r of repos) seedRepo(r);
    const inst = { id: nextInstall++, account: { id: who.id, login: who.login, type: 'User' }, repository_selection: 'selected', repos };
    installations.set(inst.id, inst);
    const { repos: _r, ...installation } = inst;
    await sendWebhook('installation', { action: 'created', installation, repositories: repos, sender: { id: who.id } });
    const code = randomBytes(10).toString('hex');
    codes.set(code, who.login);
    return redirect(res, back(callback, { code, installation_id: inst.id, setup_action: 'install', state: q.get('state') }));
  }
  if (path === '/login/oauth/access_token' && req.method === 'POST') {
    // As GitHub: JSON only for who asks for application/json.
    if (req.headers.accept !== 'application/json') return json(res, 406, { error: 'the fake answers application/json only' });
    const b = JSON.parse((await body(req)) || '{}');
    if (b.client_id !== clientId || b.client_secret !== clientSecret) return json(res, 200, { error: 'incorrect_client_credentials' });
    const login = codes.get(b.code);
    codes.delete(b.code);
    if (!login) return json(res, 200, { error: 'bad_verification_code' });
    exchanged.push(b.code);
    const token = `ghu_${randomBytes(18).toString('hex')}`;
    tokens.set(token, login);
    return json(res, 200, {
      access_token: token,
      token_type: 'bearer',
      expires_in: 28800,
      refresh_token: `ghr_${randomBytes(18).toString('hex')}`,
      refresh_token_expires_in: 15897600,
    });
  }
  if (path === '/user') {
    const login = bearer(req);
    return login ? json(res, 200, users.get(login)) : json(res, 401, { message: 'Bad credentials' });
  }
  if (path === '/user/installations') {
    const login = bearer(req);
    if (!login) return json(res, 401, { message: 'Bad credentials' });
    const mine = [...installations.values()].filter((i) => i.account.login === login).map(({ repos, ...i }) => i);
    return json(res, 200, { total_count: mine.length, installations: mine });
  }
  const repos = path.match(/^\/user\/installations\/(\d+)\/repositories$/);
  if (repos) {
    const inst = installations.get(Number(repos[1]));
    if (!bearer(req) || !inst) return json(res, 404, { message: 'Not Found' });
    return json(res, 200, { total_count: inst.repos.length, repositories: inst.repos });
  }
  const revoke = path.match(/^\/applications\/([^/]+)\/token$/);
  if (revoke && req.method === 'DELETE') {
    const basic = Buffer.from(`${clientId}:${clientSecret}`).toString('base64');
    if (decodeURIComponent(revoke[1]) !== clientId || req.headers.authorization !== `Basic ${basic}`)
      return json(res, 401, { message: 'Bad credentials' });
    const { access_token } = JSON.parse((await body(req)) || '{}');
    if (tokens.has(access_token)) revoked.push(tokens.get(access_token));
    tokens.delete(access_token);
    return json(res, 204);
  }
  if (serveRepos(req, res, { path, q, port, installations, json, redirect })) return;
  if (path === '/_fake/tag' && req.method === 'POST') {
    const b = JSON.parse((await body(req)) || '{}');
    const repo = repoOf(b.repo);
    const inst = [...installations.values()].find((i) => i.repos.some((r) => r.full_name === b.repo));
    if (!repo || !inst) return json(res, 404, { message: 'Not Found' });
    const sha = pushTag(b.repo, b.tag, { extra: b.extra ?? {}, manifest: b.manifest ?? {} });
    const { repos: _r, ...installation } = inst;
    await sendWebhook('create', {
      ref: b.tag,
      ref_type: 'tag',
      repository: { id: repo.id, full_name: b.repo, private: repo.private },
      installation,
      sender: { id: inst.account.id },
    });
    return json(res, 200, { sha });
  }
  if (path === '/_fake/state') return json(res, 200, { revoked, mails, exchanged });
  json(res, 404, { message: 'Not Found' });
});

server.listen(port, '127.0.0.1', () => console.log(`[fake github] on ${port}, smtp on ${smtpPort}`));

// The SMTP server: no TLS, no authentication; a mail to an address with "refuse" in it is refused, as a server that
// does not take it.
new SMTPServer({
  secure: false,
  authOptional: true,
  disabledCommands: ['STARTTLS', 'AUTH'],
  onRcptTo(address, _session, cb) {
    cb(address.address.includes('refuse') ? new Error('Mailbox refused') : undefined);
  },
  onData(stream, session, cb) {
    const chunks = [];
    stream.on('data', (c) => chunks.push(c));
    stream.on('end', () => {
      mails.push({
        to: session.envelope.rcptTo.map((r) => r.address),
        from: session.envelope.mailFrom?.address,
        raw: Buffer.concat(chunks).toString('utf8'),
      });
      cb();
    });
  },
}).listen(smtpPort, '127.0.0.1');

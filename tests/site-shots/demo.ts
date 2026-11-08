// The demo the website's screenshots show (`npm run site-shots`): made-up projects, agents on three harnesses, tickets in
// every column, statistics over two weeks, and the turns the fake `claude` plays for the agents at work
// ($FAKE_CLAUDE_DEMO, tests/fixtures/fake-claude.mjs). In the language of the interface; nothing of the machine's.

import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import type { Language } from '../e2e/fixture';
import { makeDemoApi, makeInfra, makeMobileApp, makeStudioWeb, ticketWorktree } from './repos';

const DAY = 86_400_000;
const MIN = 60_000;

/** What the statistics' database is written with: `node:sqlite` (Node 22.5+), newer than this project's @types/node. */
interface Sqlite {
  exec(sql: string): void;
  prepare(sql: string): { run(...values: unknown[]): unknown };
  close(): void;
}
const { DatabaseSync } = createRequire(import.meta.url)('node:sqlite') as { DatabaseSync: new (file: string) => Sqlite };

/** The projects, in the rail's order, with their colour (src/lib/theme.ts's hues). */
export const PROJECTS = {
  api: { id: 'demoapi', name: 'demo-api', hue: 48 },
  studio: { id: 'studioweb', name: 'studio-web', hue: 300 },
  mobile: { id: 'mobileapp', name: 'mobile-app', hue: 200 },
  infra: { id: 'infra', name: 'infra', hue: 150 },
} as const;

/** What each language says: the agents' names, the tickets, the conversations. */
const TEXT = {
  fr: {
    agents: {
      pagination: 'pagination-users',
      auth: 'refacto-auth',
      e2e: 'tests-e2e',
      csv: 'dem-5-export-csv',
      attempts: 'dem-6-limiter-tentatives',
      roles: 'dem-4-filtrer-par-role',
      docs: 'docs-api',
      profile: 'ecran-profil',
      push: 'notifs-push',
      header: 'refonte-header',
      terraform: 'cluster-prod',
    },
    tickets: [
      {
        key: 'DEM-1',
        title: 'Health check pour le load balancer',
        criteria: ['GET /health répond 200 en moins de 50 ms'],
      },
      { key: 'DEM-2', title: 'Comparer les signatures en temps constant', criteria: ['timingSafeEqual partout', 'Un test par cas'] },
      {
        key: 'DEM-3',
        title: 'Pagination de /users',
        criteria: ['Un curseur opaque, 50 par page', 'La réponse reste rétrocompatible', 'Tests de la dernière page'],
      },
      {
        key: 'DEM-4',
        title: 'Filtrer les utilisateurs par rôle',
        description: 'GET /users?role=admin ne renvoie que les administrateurs ; un rôle inconnu répond 400.',
        criteria: ['?role filtre la liste', 'Un rôle inconnu répond 400', 'Tests à jour'],
      },
      {
        key: 'DEM-5',
        title: 'Export CSV des factures',
        description: 'Un bouton « Exporter » sur la page des factures, et GET /invoices.csv côté API.',
        criteria: ['GET /invoices.csv avec les en-têtes', 'Montants au format français', 'Test sur 1 000 lignes'],
      },
      {
        key: 'DEM-6',
        title: 'Limiter les tentatives de connexion',
        description: '5 essais par minute et par adresse, puis 429 avec Retry-After.',
        criteria: ['429 après 5 essais', 'En-tête Retry-After', 'Remise à zéro après une connexion réussie'],
      },
      { key: 'DEM-7', title: 'Webhooks de facturation', criteria: ['Signature HMAC', 'Trois nouvelles tentatives'] },
      { key: 'DEM-8', title: 'Clés d’API par compte', criteria: ['Créer et révoquer une clé', 'Quota par clé'] },
      { key: 'DEM-9', title: 'Documenter l’API v2', criteria: ['Une page OpenAPI à jour'] },
    ],
    progress: {
      csv: ['Route GET /invoices.csv écrite', 'En-têtes et séparateur ; vérifiés'],
      attempts: ['Compteur par adresse en mémoire'],
      roles: ['Filtre ?role ajouté à la route', 'Rôle inconnu : 400 et message', '12 tests passés'],
    },
    pagination: {
      prompt: 'Ajoute une pagination par curseur sur /users, 50 par page par défaut, et garde la réponse rétrocompatible.',
      thinking: 'La route renvoie tout le tableau : un curseur opaque sur l’id garde l’ordre stable et ne casse pas les clients actuels.',
      todos: [
        'Lire la route /users et ses appelants',
        'Écrire paginate() dans src/db',
        'Brancher la route sur paginate()',
        'Écrire les tests de la pagination',
        'Relancer toute la suite',
      ],
      plan: 'Voici le plan : **une limite par défaut**, un curseur opaque, et des tests pour chaque cas.',
      done: 'La pagination est en place :\n\n- `GET /users` renvoie **50 utilisateurs** par défaut, `limit` va jusqu’à 100 ;\n- `next` donne le curseur de la page suivante, `null` à la dernière ;\n- les clients actuels lisent toujours `users`.\n\nLes **43 tests** passent. Je lance les e2e.',
      testOutput:
        ' ✓ tests/users.test.ts (6 tests) 41ms\n ✓ tests/paginate.test.ts (5 tests) 12ms\n\n Test Files  8 passed (8)\n      Tests  43 passed (43)\n   Duration  1.84s',
    },
    auth: {
      prompt: 'Passe les sessions sur des jetons à rotation : 15 minutes, et un jeton de rafraîchissement.',
      text: 'Je raccourcis la durée des jetons et j’ajoute la rotation.',
    },
    e2e: {
      prompt: 'Ajoute un test e2e du parcours de connexion, puis lance la suite.',
      text: 'Le test `login.spec.ts` est écrit : connexion, mauvais mot de passe, déconnexion.',
      question: 'Lancer toute la suite e2e (38 tests) ?',
      header: 'Suite e2e',
      options: [
        { label: 'Oui', description: 'Les 38 tests, environ 2 minutes' },
        { label: 'Seulement auth', description: 'Les 12 tests de connexion' },
      ],
    },
    profile: {
      prompt: 'Refais l’écran de profil : avatar, statistiques et réglages, en clair et en sombre.',
      thinking: 'Un en-tête avec l’avatar, trois chiffres, puis la liste des réglages ; les couleurs viennent du thème.',
      intro: 'Je reprends l’écran en trois blocs : **l’en-tête**, les **statistiques** et les **réglages**.',
      done: 'Voici l’écran, en clair et en sombre. Les tests de rendu passent sur les deux thèmes.',
      caption: 'L’écran de profil, thème clair et thème sombre',
      testOutput:
        ' PASS  src/screens/ProfileScreen.test.tsx\n  ✓ renders the header (24 ms)\n  ✓ follows the dark theme (9 ms)\n\nTests: 7 passed, 7 total',
    },
    header: {
      prompt: 'Ajoute le compteur de projets dans le header, avec sa route côté API.',
      text: 'Le header affiche le nombre de projets, lu sur `GET /projects/count`.',
    },
    remote: 'Ajoute aussi le cas d’un jeton expiré, s’il te plaît.',
    remoteAnswer: 'C’est fait : un jeton expiré répond **401** avec `token_expired`, et le client le rafraîchit une fois.',
  },
  en: {
    agents: {
      pagination: 'paginate-users',
      auth: 'refactor-auth',
      e2e: 'e2e-tests',
      csv: 'dem-5-csv-export',
      attempts: 'dem-6-limit-attempts',
      roles: 'dem-4-filter-by-role',
      docs: 'docs-api',
      profile: 'profile-screen',
      push: 'push-notifs',
      header: 'header-redesign',
      terraform: 'prod-cluster',
    },
    tickets: [
      { key: 'DEM-1', title: 'Health check for the load balancer', criteria: ['GET /health answers 200 in under 50 ms'] },
      { key: 'DEM-2', title: 'Compare signatures in constant time', criteria: ['timingSafeEqual everywhere', 'A test per case'] },
      {
        key: 'DEM-3',
        title: 'Paginate /users',
        criteria: ['An opaque cursor, 50 per page', 'The response stays backward compatible', 'Tests of the last page'],
      },
      {
        key: 'DEM-4',
        title: 'Filter users by role',
        description: 'GET /users?role=admin returns the admins only; an unknown role answers 400.',
        criteria: ['?role filters the list', 'An unknown role answers 400', 'Tests up to date'],
      },
      {
        key: 'DEM-5',
        title: 'CSV export of invoices',
        description: 'An “Export” button on the invoices page, and GET /invoices.csv in the API.',
        criteria: ['GET /invoices.csv with headers', 'Amounts in the account’s locale', 'A test on 1,000 rows'],
      },
      {
        key: 'DEM-6',
        title: 'Limit sign-in attempts',
        description: '5 tries a minute per address, then 429 with Retry-After.',
        criteria: ['429 after 5 tries', 'A Retry-After header', 'Reset after a successful sign-in'],
      },
      { key: 'DEM-7', title: 'Billing webhooks', criteria: ['HMAC signature', 'Three retries'] },
      { key: 'DEM-8', title: 'API keys per account', criteria: ['Create and revoke a key', 'A quota per key'] },
      { key: 'DEM-9', title: 'Document the v2 API', criteria: ['An up-to-date OpenAPI page'] },
    ],
    progress: {
      csv: ['GET /invoices.csv route written', 'Headers and separator checked'],
      attempts: ['In-memory counter per address'],
      roles: ['?role filter added to the route', 'Unknown role: 400 and a message', '12 tests passed'],
    },
    pagination: {
      prompt: 'Add cursor pagination to /users, 50 per page by default, and keep the response backward compatible.',
      thinking: 'The route returns the whole array: an opaque cursor on the id keeps the order stable and breaks no current client.',
      todos: [
        'Read the /users route and its callers',
        'Write paginate() in src/db',
        'Wire the route to paginate()',
        'Write the pagination tests',
        'Run the whole suite again',
      ],
      plan: 'Here is the plan: **a default limit**, an opaque cursor, and tests for each case.',
      done: 'Pagination is in:\n\n- `GET /users` returns **50 users** by default, `limit` goes up to 100;\n- `next` holds the next page’s cursor, `null` on the last one;\n- current clients still read `users`.\n\nAll **43 tests** pass. Running the e2e suite.',
      testOutput:
        ' ✓ tests/users.test.ts (6 tests) 41ms\n ✓ tests/paginate.test.ts (5 tests) 12ms\n\n Test Files  8 passed (8)\n      Tests  43 passed (43)\n   Duration  1.84s',
    },
    auth: {
      prompt: 'Move sessions to rotating tokens: 15 minutes, and a refresh token.',
      text: 'Shortening the tokens’ lifetime and adding rotation.',
    },
    e2e: {
      prompt: 'Add an e2e test of the sign-in flow, then run the suite.',
      text: '`login.spec.ts` is written: sign in, wrong password, sign out.',
      question: 'Run the whole e2e suite (38 tests)?',
      header: 'e2e suite',
      options: [
        { label: 'Yes', description: 'All 38 tests, about 2 minutes' },
        { label: 'Auth only', description: 'The 12 sign-in tests' },
      ],
    },
    profile: {
      prompt: 'Redo the profile screen: avatar, stats and settings, in light and dark.',
      thinking: 'A header with the avatar, three figures, then the settings list; the colours come from the theme.',
      intro: 'I am rebuilding the screen in three blocks: **the header**, the **stats** and the **settings**.',
      done: 'Here is the screen, in light and dark. The render tests pass on both themes.',
      caption: 'The profile screen, light theme and dark theme',
      testOutput:
        ' PASS  src/screens/ProfileScreen.test.tsx\n  ✓ renders the header (24 ms)\n  ✓ follows the dark theme (9 ms)\n\nTests: 7 passed, 7 total',
    },
    header: {
      prompt: 'Add the project count to the header, with its route in the API.',
      text: 'The header shows the number of projects, read from `GET /projects/count`.',
    },
    remote: 'Please also handle an expired token.',
    remoteAnswer: 'Done: an expired token answers **401** with `token_expired`, and the client refreshes it once.',
  },
} as const;

/** The pull requests of the code hosts' screenshots, in each language: what the fake GitHub is given (`pullsOf`). */
const HOSTS = {
  fr: {
    pulls: [
      ['feat/cursor-pagination', 'Pagination de /users par curseur'],
      ['fix/session-expiry', 'Sessions expirées : renvoyer 401 plutôt que 500'],
      ['i18n-es', 'Traductions espagnoles'],
      ['chore/vitest-3', 'Passer à Vitest 3'],
      ['docs/openapi', 'Décrire l’API en OpenAPI'],
      ['feat/invoices-csv', 'Exporter les factures en CSV'],
    ],
    review: {
      summary:
        'La pagination tient, le curseur reste opaque et la limite est bornée. Deux points avant de merger : le tri et la dernière page.',
      comments: [
        { line: 7, body: 'Sans tri stable sur id, deux utilisateurs créés à la même date peuvent sauter d’une page à l’autre.' },
        { line: 11, body: 'La dernière page rend nextCursor même quand il n’y a plus rien : ajoute un test.' },
      ],
    },
  },
  en: {
    pulls: [
      ['feat/cursor-pagination', 'Cursor pagination for /users'],
      ['fix/session-expiry', 'Expired sessions: return 401 rather than 500'],
      ['i18n-es', 'Spanish translations'],
      ['chore/vitest-3', 'Move to Vitest 3'],
      ['docs/openapi', 'Describe the API with OpenAPI'],
      ['feat/invoices-csv', 'Export invoices as CSV'],
    ],
    review: {
      summary:
        'The pagination holds, the cursor stays opaque and the limit is bounded. Two points before merging: the order and the last page.',
      comments: [
        { line: 7, body: 'Without a stable order on id, two users created at the same date can jump from one page to the other.' },
        { line: 11, body: 'The last page returns nextCursor even when nothing is left: add a test.' },
      ],
    },
  },
} as const;
export const hostsOf = (lang: Language) => HOSTS[lang];

/** The file of the pull request the demo's agent reviews. */
export const REVIEWED = {
  path: 'src/db/cursor.ts',
  text: [
    "import { db } from './client';",
    '',
    'const MAX = 50;',
    '',
    'export async function page(cursor: string | null, limit = MAX) {',
    '  const after = cursor ? Buffer.from(cursor, "base64url").toString() : "";',
    '  const users = (await db.users.all()).filter((u: { id: string }) => u.id > after);',
    '  const items = users.slice(0, Math.min(limit, MAX));',
    '  const last = items.at(-1)?.id ?? "";',
    '  return {',
    '    items,',
    '    nextCursor: Buffer.from(last).toString("base64url"),',
    '  };',
    '}',
    '',
  ].join('\n'),
};

export type DemoText = (typeof TEXT)[Language];
export const textOf = (lang: Language): DemoText => TEXT[lang] as DemoText;

/** The Jira issues of the import (tests/fixtures/fake-trackers.mjs, given as its data): three, each with its criteria. */
const JIRA: Record<
  Language,
  { summary: string; type: string; priority: string; assignee: string | null; text: string; criteria: string[] }[]
> = {
  fr: [
    {
      summary: 'Rafraîchir le jeton silencieusement côté client',
      type: 'Story',
      priority: 'High',
      assignee: 'Inès Haddad',
      text: 'Le jeton expire pendant la saisie : le client doit le renouveler avant.',
      criteria: ['Rafraîchi une minute avant son expiration', 'Aucune erreur 401 visible'],
    },
    {
      summary: 'Erreur 500 quand l’e-mail contient un « + »',
      type: 'Bug',
      priority: 'Highest',
      assignee: null,
      text: 'Reproduit avec alex+test@example.com à l’inscription.',
      criteria: ['L’inscription accepte un « + »', 'Un test de régression'],
    },
    {
      summary: 'Exporter le journal d’audit en CSV',
      type: 'Story',
      priority: 'Medium',
      assignee: 'Léo Garnier',
      text: 'Les administrateurs exportent le journal d’une période.',
      criteria: ['Filtre par date', 'Colonnes horodatage, IP, résultat', 'Export de 10 000 lignes en moins de 2 s'],
    },
  ],
  en: [
    {
      summary: 'Refresh the token silently in the client',
      type: 'Story',
      priority: 'High',
      assignee: 'Inès Haddad',
      text: 'The token expires while the user types: the client must renew it first.',
      criteria: ['Refreshed a minute before it expires', 'No 401 error shown'],
    },
    {
      summary: 'Error 500 when the email holds a “+”',
      type: 'Bug',
      priority: 'Highest',
      assignee: null,
      text: 'Reproduced with alex+test@example.com at sign-up.',
      criteria: ['Sign-up accepts a “+”', 'A regression test'],
    },
    {
      summary: 'Export the audit log as CSV',
      type: 'Story',
      priority: 'Medium',
      assignee: 'Léo Garnier',
      text: 'Admins export the log of a period.',
      criteria: ['Filter by date', 'Timestamp, IP and result columns', '10,000 rows exported in under 2 s'],
    },
  ],
};

/** What the fake trackers start from for the import's picture: these issues in Jira, in the language of the demo. */
export function trackersOf(lang: Language) {
  type Db = { jira: { issues: unknown[] } };
  const adf = (...content: unknown[]) => ({ type: 'doc', version: 1, content });
  const para = (text: string) => ({ type: 'paragraph', content: [{ type: 'text', text }] });
  return (db: Db) => {
    db.jira.issues = JIRA[lang].map((i, n) => ({
      key: `ATL-${1287 + n * 3}`,
      summary: i.summary,
      type: i.type,
      priority: i.priority,
      assignee: i.assignee,
      status: '1',
      labels: [],
      description: adf(para(i.text), para('Acceptance criteria:'), {
        type: 'bulletList',
        content: i.criteria.map((c) => ({ type: 'listItem', content: [para(c)] })),
      }),
    }));
    return db;
  };
}

const PAGINATE = `export interface Page<T> {
  items: T[];
  next: string | null;
}

const encode = (id: string) => Buffer.from(JSON.stringify({ id })).toString('base64url');
const decode = (cursor: string) => JSON.parse(Buffer.from(cursor, 'base64url').toString()).id as string;

export async function paginate<T extends { id: string }>(
  query: { after(id: string | null): { take(n: number): Promise<T[]> } },
  { limit = 50, cursor }: { limit?: number; cursor?: string },
): Promise<Page<T>> {
  const rows = await query.after(cursor ? decode(cursor) : null).take(limit + 1);
  const items = rows.slice(0, limit);
  return { items, next: rows.length > limit ? encode(items[items.length - 1].id) : null };
}
`;

/** The turns the fake `claude` plays, by the message that starts them (tests/fixtures/fake-claude.mjs, « The website's screenshots »). */
function turns(t: DemoText) {
  const p = t.pagination;
  const todos = (done: number, doing: number) =>
    p.todos.map((content, i) => ({
      content,
      activeForm: content,
      status: i < done ? 'completed' : i === doing ? 'in_progress' : 'pending',
    }));
  return {
    [p.prompt]: [
      { thinking: p.thinking },
      { tool: 'TodoWrite', todos: todos(0, 0) },
      { tool: 'Read', file: 'src/routes/users.ts', lines: 19 },
      { tool: 'TodoWrite', todos: todos(1, 1) },
      { text: p.plan },
      { tool: 'Write', file: 'src/db/paginate.ts', content: PAGINATE },
      { tool: 'TodoWrite', todos: todos(2, 2) },
      {
        tool: 'Edit',
        file: 'src/routes/users.ts',
        old: "users.get('/users', async (req, res) => {\n  const users = await db.users.all();\n  res.json(users);\n});",
        new: "users.get('/users', async (req, res) => {\n  const limit = Math.min(Number(req.query.limit ?? 50), 100);\n  const page = await paginate(db.users, { limit, cursor: req.query.cursor as string });\n  // `users` stays for the clients that read it.\n  res.json({ users: page.items, next: page.next });\n});",
      },
      {
        tool: 'Edit',
        file: 'src/routes/users.ts',
        old: "import { requireAuth } from '../auth/middleware';",
        new: "import { requireAuth } from '../auth/middleware';\nimport { paginate } from '../db/paginate';",
      },
      { tool: 'TodoWrite', todos: todos(3, 3) },
      {
        tool: 'Edit',
        file: 'tests/users.test.ts',
        old: "  it('refuses a request without a token', async () => {",
        new: "  it('pages with a cursor', async () => {\n    const first = await request.get('/users?limit=2').auth('admin');\n    expect(first.body.users).toHaveLength(2);\n    const next = await request.get(`/users?limit=2&cursor=${first.body.next}`).auth('admin');\n    expect(next.body.users[0].id).not.toBe(first.body.users[0].id);\n  });\n\n  it('refuses a request without a token', async () => {",
      },
      { tool: 'Bash', command: 'npm test', description: 'Run the unit tests', output: p.testOutput },
      { tool: 'TodoWrite', todos: todos(4, 4) },
      { text: p.done },
      { usage: { input: 18_400, output: 9_600, cache: 41_000, cost: 0.32 } },
      { tool: 'Bash', command: 'npm run test:e2e', description: 'Run the e2e suite', hold: true },
    ],
    [t.auth.prompt]: [
      { text: t.auth.text },
      {
        tool: 'Edit',
        file: 'src/auth/session.ts',
        old: 'const HOUR = 3600;',
        new: 'const HOUR = 3600;\nconst ACCESS_TTL = 15 * 60;\nconst REFRESH_TTL = 30 * 24 * HOUR;',
      },
      { usage: { input: 96_000, output: 28_000, cache: 60_000, cost: 2.41 } },
      {
        tool: 'Edit',
        file: 'src/auth/jwt.ts',
        old: "  role: 'admin' | 'member';\n  exp: number;",
        new: "  role: 'admin' | 'member';\n  exp: number;\n  /** The refresh token's id, rotated at each use. */\n  jti?: string;",
        hold: true,
      },
    ],
    [t.e2e.prompt]: [
      { text: t.e2e.text },
      {
        tool: 'Write',
        file: 'tests/e2e/login.spec.ts',
        content:
          "import { expect, test } from '@playwright/test';\n\ntest('signs in and out', async ({ page }) => {\n  await page.goto('/login');\n  await page.getByLabel('Email').fill('alex@example.com');\n  await page.getByRole('button', { name: 'Sign in' }).click();\n  await expect(page.getByText('Projects')).toBeVisible();\n});\n",
      },
      { usage: { input: 52_000, output: 14_000, cache: 30_000, cost: 0.73 } },
      { question: [{ question: t.e2e.question, header: t.e2e.header, multiSelect: false, options: t.e2e.options }] },
      { text: '✓ 38 passed (1.9m)' },
    ],
    [t.profile.prompt]: [
      { thinking: t.profile.thinking },
      { text: t.profile.intro },
      {
        tool: 'Edit',
        file: 'src/screens/ProfileScreen.tsx',
        old: '    <View style={styles.screen}>\n      <Text style={styles.name}>{user.name}</Text>\n      <Text style={styles.email}>{user.email}</Text>',
        new: '    <View style={[styles.screen, theme.surface]}>\n      <Avatar uri={user.avatar} size={72} />\n      <Text style={[styles.name, theme.text]}>{user.name}</Text>\n      <Text style={styles.email}>{user.email}</Text>\n      <Stats items={[user.projects, user.followers, user.following]} />\n      <SettingsList sections={settings} />',
      },
      { tool: 'Bash', command: 'npm test -- ProfileScreen', description: 'Run the screen’s render tests', output: t.profile.testOutput },
      { send: ['.escouade-shots/profile-light.png', '.escouade-shots/profile-dark.png'], caption: t.profile.caption },
      { text: t.profile.done },
      { usage: { input: 120_000, output: 31_000, cache: 90_000, cost: 1.86 } },
    ],
    [t.header.prompt]: [
      { text: t.header.text },
      {
        tool: 'Edit',
        file: 'web/src/components/Header.tsx',
        old: '        <Link to="/team">Team</Link>\n      </nav>',
        new: '        <Link to="/team">Team</Link>\n      </nav>\n      <ProjectCount />',
      },
      {
        tool: 'Edit',
        file: 'api/src/routes/projects.ts',
        old: "projects.get('/projects', async (_req, res) => {\n  res.json(await db.projects.all());\n});",
        new: "projects.get('/projects', async (_req, res) => {\n  res.json(await db.projects.all());\n});\n\nprojects.get('/projects/count', async (_req, res) => {\n  res.json({ count: (await db.projects.all()).length });\n});",
      },
      { usage: { input: 40_000, output: 11_000, cache: 20_000, cost: 0.58 } },
      { tool: 'Bash', command: 'npm run dev', description: 'Start the studio', hold: true },
    ],
    [t.remote]: [{ text: t.remoteAnswer }],
  };
}

/** An agent of the demo, saved as the app saves one (`model::AgentMeta`). */
function agent(o: {
  id: string;
  project: keyof typeof PROJECTS;
  name: string;
  engine?: 'claude' | 'codex' | 'cursor';
  model: string;
  effort?: string;
  tokens: number;
  cost: number;
  minutes: number;
  status?: string;
  ticket?: string;
  root: string;
  ago?: number;
  files?: string[];
  extra?: Record<string, unknown>;
}) {
  const engine = o.engine ?? 'claude';
  const now = Date.now();
  return {
    id: o.id,
    projectId: PROJECTS[o.project].id,
    name: o.name,
    named: true,
    engine,
    model: o.model,
    effort: o.effort ?? 'high',
    mode: 'auto',
    cwd: path.join(o.root, PROJECTS[o.project].name),
    createdAt: now - (o.ago ?? 40) * MIN,
    status: o.status ?? 'idle',
    tokens: o.tokens,
    cost: o.cost,
    activeMs: o.minutes * MIN,
    touchedFiles: o.files ?? [],
    lastActivity: now - 2 * MIN,
    prompts: 3,
    account: engine === 'claude' ? 'principal' : `principal-${engine}`,
    ticketId: o.ticket ?? null,
    ...o.extra,
  };
}

/**
 * A ticket agent « En cours » the app's start leaves alone (`board::recovery`: it waits for its quota) rather than send it
 * the resume, which the fake harnesses would answer without a report: the screenshots start its turn themselves.
 */
const HELD = { resumeAt: Date.now() + 3 * 3_600_000 };

/** The agents' ids, for the screenshots to pick them. */
export const AGENTS = {
  pagination: 'a-pagination',
  auth: 'a-auth',
  e2e: 'a-e2e',
  csv: 'a-csv',
  attempts: 'a-attempts',
  roles: 'a-roles',
  docs: 'a-docs',
  profile: 'a-profile',
  push: 'a-push',
  header: 'a-header',
  terraform: 'a-terraform',
} as const;

/** The logo of studio-web's tile: a made-up mark. */
const STUDIO_LOGO = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#f4f1ff"/><path d="M20 42c6 5 22 5 24-4 2-10-22-6-20-16 2-8 16-8 22-3" fill="none" stroke="#6d4bd8" stroke-width="7" stroke-linecap="round"/></svg>`;
/** And of infra's. */
const INFRA_LOGO = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="14" fill="#1f6f4a"/><path d="M14 44l18-26 18 26z" fill="none" stroke="#d6f5e3" stroke-width="5" stroke-linejoin="round"/><circle cx="32" cy="36" r="4" fill="#d6f5e3"/></svg>`;

/**
 * Makes the demo in `root` (the test's folder, its data in `root/data`), in `lang`, the ticket to test on the ports from
 * `ports` (free ones, found by the screenshots); gives the file of the fake's turns.
 */
/** `hosts`: for the code hosts' screenshots, tickets merged at commits of demo-api and one « En revue » (its pull request `hosts`). */
export function makeDemo(root: string, lang: Language, ports: number, hosts?: { review: number; branch: string }): string {
  const t = textOf(lang);
  const dir = (p: keyof typeof PROJECTS) => path.join(root, PROJECTS[p].name);
  makeDemoApi(dir('api'));
  makeStudioWeb(dir('studio'));
  makeMobileApp(dir('mobile'));
  makeInfra(dir('infra'));

  const data = path.join(root, 'data');
  const logo = (id: string, file: string, svg: string) => {
    fs.mkdirSync(path.join(data, 'projects', id), { recursive: true });
    fs.writeFileSync(path.join(data, 'projects', id, file), svg);
    return file;
  };
  const now = Date.now();
  const color = (hue: number) => `oklch(0.72 0.12 ${hue})`;
  const run = (id: string, name: string, script: string) => ({ id, name, command: `node scripts/${script}.mjs`, shell: '', cwd: '' });
  const projects = [
    {
      ...PROJECTS.api,
      path: dir('api'),
      color: color(PROJECTS.api.hue),
      tileLetters: 'API',
      tileLabel: true,
      createdAt: now - 20 * DAY,
      runCommands: [run('r-web', 'Front', 'web'), run('r-api', 'API', 'api'), run('r-worker', 'Worker', 'worker')],
      board: {
        autopilot: false,
        prefix: 'DEM',
        nextNumber: 10,
        target: 'main',
        maxParallel: hosts ? 4 : 2,
        ...(hosts ? { action: 'pr' } : {}),
      },
    },
    {
      ...PROJECTS.studio,
      path: dir('studio'),
      color: color(PROJECTS.studio.hue),
      logo: logo(PROJECTS.studio.id, 'logo-studio.svg', STUDIO_LOGO),
      tileLabel: true,
      tileLabelText: 'Studio',
      createdAt: now - 18 * DAY,
      board: { autopilot: false },
    },
    {
      ...PROJECTS.mobile,
      path: dir('mobile'),
      color: color(PROJECTS.mobile.hue),
      tileLabel: true,
      tileLabelText: 'Mobile',
      createdAt: now - 15 * DAY,
      board: { autopilot: false },
    },
    {
      ...PROJECTS.infra,
      path: dir('infra'),
      color: color(PROJECTS.infra.hue),
      logo: logo(PROJECTS.infra.id, 'logo-infra.svg', INFRA_LOGO),
      tileLabel: true,
      createdAt: now - 12 * DAY,
      board: { autopilot: false },
    },
  ].map(({ hue: _hue, ...p }) => p);

  // The tickets of demo-api, in every column; the merged ones say it as the app does (`board::merged_outcome`).
  const merged = lang === 'fr' ? '⤵ Mergé dans main · squash' : '⤵ Merged into main · squash';
  const tk = (key: string) => t.tickets.find((x) => x.key === key)!;
  const ticket = (key: string, column: string, o: Record<string, unknown> = {}, ok = 0) => {
    const def = tk(key);
    const n = Number(key.split('-')[1]);
    return {
      id: `t-${n}`,
      projectId: PROJECTS.api.id,
      key,
      title: def.title,
      description: 'description' in def ? def.description : '',
      criteria: def.criteria.map((text, i) => ({ text, ok: i < ok, note: '' })),
      maxLoops: 5,
      column,
      rank: n,
      createdAt: now - (12 - n) * DAY,
      model: '',
      effort: '',
      ...o,
    };
  };
  const tickets = [
    ticket('DEM-1', 'done', { doneAt: now - 9 * DAY, startedAt: now - 10 * DAY, cost: 0.41, loops: 1, iteration: 1, outcome: merged }, 1),
    ticket('DEM-2', 'done', { doneAt: now - 6 * DAY, startedAt: now - 7 * DAY, cost: 0.88, loops: 2, iteration: 2, outcome: merged }, 2),
    ticket('DEM-3', 'done', { doneAt: now - 1 * DAY, startedAt: now - 2 * DAY, cost: 1.74, loops: 2, iteration: 2, outcome: merged }, 3),
    ticket(
      'DEM-4',
      'review',
      {
        agentId: AGENTS.roles,
        branch: `ticket/${t.agents.roles}`,
        iteration: 2,
        loops: 2,
        startedAt: now - 90 * MIN,
        reviewAt: now - 20 * MIN,
        cost: 0.62,
        progress: t.progress.roles,
      },
      3,
    ),
    ticket(
      'DEM-5',
      'doing',
      { agentId: AGENTS.csv, iteration: 2, loops: 1, startedAt: now - 35 * MIN, cost: 0.41, engine: 'codex', progress: t.progress.csv },
      1,
    ),
    ticket('DEM-6', 'doing', {
      agentId: AGENTS.attempts,
      iteration: 1,
      startedAt: now - 12 * MIN,
      cost: 0.1,
      engine: 'cursor',
      progress: t.progress.attempts,
    }),
    hosts
      ? ticket(
          'DEM-7',
          'pr',
          {
            branch: hosts.branch,
            iteration: 1,
            loops: 1,
            startedAt: now - 5 * 60 * MIN,
            reviewAt: now - 3 * 60 * MIN,
            cost: 0.94,
            pr: {
              number: hosts.review,
              url: `https://github.com/acme/demo/pull/${hosts.review}`,
              kind: 'github',
              host: 'github.com',
              namespace: 'acme',
              name: 'demo',
              target: 'main',
              branch: hosts.branch,
              word: 'PR',
              state: 'open',
            },
          },
          1,
        )
      : ticket('DEM-7', 'todo'),
    ticket('DEM-8', 'todo', { after: ['t-6'] }),
    ticket('DEM-9', 'todo'),
  ];

  if (hosts) {
    // Merged where the environments of the fake GitHub find them: DEM-3 at main's head, DEM-2 a little before.
    const at = (ref: string) => execFileSync('git', ['rev-parse', ref], { cwd: dir('api'), encoding: 'utf8' }).trim();
    for (const [key, ref] of [
      ['DEM-2', 'HEAD~2'],
      ['DEM-3', 'HEAD'],
    ])
      Object.assign(
        tickets.find((x) => x.key === key)!,
        { outcomeCommit: at(ref), outcomeUrl: 'https://github.com/acme/demo/pulls' },
      );
  }

  const recipe = {
    prepare: [{ command: 'node scripts/seed.mjs', dir: '' }],
    processes: [
      { name: 'api', command: 'node scripts/api.mjs', dir: '', env: { PORT: String(ports) }, url: `http://localhost:${ports}` },
      { name: 'web', command: 'node scripts/web.mjs', dir: '', env: { PORT: String(ports + 1) }, url: `http://localhost:${ports + 1}` },
    ],
    open: `http://localhost:${ports + 1}/users?role=admin`,
  };
  const a = t.agents;
  // The ticket to test was worked in a worktree of its own, on its branch.
  const rolesTree = ticketWorktree(
    dir('api'),
    a.roles,
    { 'src/routes/roles.ts': "export const ROLES = ['admin', 'member'] as const;\n" },
    'feat(users): filter the list by role',
  );
  const agents = [
    agent({ id: AGENTS.e2e, project: 'api', name: a.e2e, model: 'sonnet', tokens: 96_000, cost: 0.73, minutes: 8, root, ago: 30 }),
    agent({ id: AGENTS.auth, project: 'api', name: a.auth, model: 'opus', tokens: 184_000, cost: 2.41, minutes: 12, root, ago: 50 }),
    agent({
      id: AGENTS.pagination,
      project: 'api',
      name: a.pagination,
      model: 'sonnet',
      tokens: 41_000,
      cost: 0.32,
      minutes: 3,
      root,
      ago: 45,
    }),
    agent({
      id: AGENTS.csv,
      project: 'api',
      name: a.csv,
      engine: 'codex',
      model: 'gpt-5.6-terra',
      tokens: 38_000,
      cost: 0.41,
      minutes: 6,
      ticket: 't-5',
      root,
      extra: HELD,
    }),
    agent({
      id: AGENTS.attempts,
      project: 'api',
      name: a.attempts,
      engine: 'cursor',
      model: 'composer-2.5[fast=true]',
      tokens: 9_000,
      cost: 0.1,
      minutes: 2,
      ticket: 't-6',
      root,
      extra: HELD,
    }),
    agent({
      id: AGENTS.roles,
      project: 'api',
      name: a.roles,
      model: 'sonnet',
      tokens: 51_000,
      cost: 0.62,
      minutes: 9,
      status: 'done',
      ticket: 't-4',
      root,
      extra: { recipe, approvedRecipe: recipe, portBase: ports, cwd: rolesTree.path, worktree: { ...rolesTree, baseBranch: 'main' } },
    }),
    agent({
      id: AGENTS.docs,
      project: 'studio',
      name: a.docs,
      model: 'haiku',
      effort: 'low',
      tokens: 12_000,
      cost: 0.02,
      minutes: 2,
      status: 'done',
      root,
    }),
    agent({ id: AGENTS.profile, project: 'mobile', name: a.profile, model: 'opus', tokens: 240_000, cost: 1.86, minutes: 14, root }),
    agent({
      id: AGENTS.push,
      project: 'mobile',
      // Started after ecran-profil, not at the same millisecond: the bar sorts by start, a tie would swap the two cards.
      ago: 30,
      name: a.push,
      engine: 'codex',
      model: 'gpt-5.6-terra',
      tokens: 22_000,
      cost: 0.19,
      minutes: 4,
      status: 'done',
      root,
    }),
    agent({ id: AGENTS.header, project: 'studio', name: a.header, model: 'sonnet', tokens: 71_000, cost: 0.58, minutes: 5, root }),
    agent({
      id: AGENTS.terraform,
      project: 'infra',
      name: a.terraform,
      engine: 'cursor',
      model: 'composer-2.5[fast=true]',
      tokens: 31_000,
      cost: 0.12,
      minutes: 7,
      status: 'done',
      root,
    }),
  ];
  fs.writeFileSync(
    path.join(data, 'state.json'),
    JSON.stringify({
      projects,
      agents,
      tickets,
      ui: { activeProject: PROJECTS.api.id, view: 'agents', selectedAgent: { [PROJECTS.api.id]: AGENTS.pagination }, layout: '' },
    }),
  );
  stats(data, agents);
  const file = path.join(root, 'demo-turns.json');
  fs.writeFileSync(file, JSON.stringify(turns(t)));
  return file;
}

/** Two weeks of turns in the statistics (stats.db, `stats::SCHEMA`): more on weekdays, three harnesses, four projects. */
function stats(data: string, agents: ReturnType<typeof agent>[]) {
  const db = new DatabaseSync(path.join(data, 'stats.db'));
  db.exec(`CREATE TABLE IF NOT EXISTS turns (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
  model TEXT NOT NULL, input INTEGER NOT NULL, cache INTEGER NOT NULL, output INTEGER NOT NULL, cost REAL NOT NULL,
  account TEXT NOT NULL DEFAULT 'principal', engine TEXT NOT NULL DEFAULT 'claude');
CREATE INDEX IF NOT EXISTS turns_ts ON turns(ts);
CREATE TABLE IF NOT EXISTS prompts (
  id INTEGER PRIMARY KEY, ts INTEGER NOT NULL, agent_id TEXT NOT NULL, project_id TEXT NOT NULL,
  account TEXT NOT NULL DEFAULT 'principal', engine TEXT NOT NULL DEFAULT 'claude');
CREATE INDEX IF NOT EXISTS prompts_ts ON prompts(ts);`);
  const turn = db.prepare(
    'INSERT INTO turns (ts, agent_id, project_id, model, input, cache, output, cost, account, engine) VALUES (?,?,?,?,?,?,?,?,?,?)',
  );
  const prompt = db.prepare('INSERT INTO prompts (ts, agent_id, project_id, account, engine) VALUES (?,?,?,?,?)');
  const MODELS: Record<string, { id: string; usd: number }> = {
    opus: { id: 'claude-opus-5-5', usd: 13 },
    sonnet: { id: 'claude-sonnet-5-5', usd: 7.6 },
    haiku: { id: 'claude-haiku-5-5', usd: 1.7 },
    'gpt-5.6-terra': { id: 'gpt-5.6-terra', usd: 6.2 },
    'composer-2.5[fast=true]': { id: 'composer-2.5', usd: 4.1 },
  };
  // A pseudo-random sequence of its own: the same pictures at each run.
  let seed = 7;
  const rnd = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
  const start = new Date();
  start.setHours(0, 0, 0, 0);
  db.exec('BEGIN');
  for (let d = 13; d >= 0; d--) {
    const day = start.getTime() - d * DAY;
    const weekday = new Date(day).getDay();
    const busy = weekday === 0 || weekday === 6 ? 0.25 : 0.7 + rnd() * 0.6;
    for (const a of agents) {
      const m = MODELS[a.model];
      const n = Math.round((a.engine === 'claude' ? 9 : 5) * busy * (0.5 + rnd()));
      for (let i = 0; i < n; i++) {
        // Whole milliseconds: the app reads them as integers.
        const ts = Math.round(day + (9 + rnd() * 9) * 3_600_000);
        if (ts > Date.now()) continue;
        const input = Math.round(2_000 + rnd() * 9_000);
        const cache = Math.round(20_000 + rnd() * 60_000);
        const output = Math.round(600 + rnd() * 2_400);
        const cost = ((input + output * 4 + cache * 0.1) * m.usd) / 1e6;
        turn.run(ts, a.id, a.projectId, m.id, input, cache, output, cost, a.account, a.engine);
        if (i % 3 === 0) prompt.run(ts, a.id, a.projectId, a.account, a.engine);
      }
    }
  }
  db.exec('COMMIT');
  db.close();
}

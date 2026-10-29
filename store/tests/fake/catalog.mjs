// Published plugins for the end-to-end tests of the public pages (STORE-4), written straight into the base of
// DATABASE_URL, their archives into ARCHIVE_DIR as the checks keep them: the pages read them as they read a version the
// checks published, and render their README the first time (a version published before STORE-4 has none rendered).
// Each run has its own ids and logins (the base keeps the runs before). Nothing here is ever run: they are bytes.
import { createHash, randomBytes } from 'node:crypto';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import pg from 'pg';
import { MIT, png, tarball } from './plugins.mjs';

const sha256 = (b) => createHash('sha256').update(b).digest('hex');

/** The README of the Slack plugin, booby-trapped as the chief of the squad listed (STORE-2's review). */
export const TRAPPED = [
  '# Slack',
  '',
  'Quand un ticket passe **À tester** ou qu’un agent attend ta réponse, le plugin poste un message dans le salon Slack du projet.',
  '',
  '![Le salon](store/1.png)',
  '',
  '<script>window.__pwned = "script"</script>',
  '<style>body { display: none !important }</style>',
  '<img src="x" onerror="window.__pwned = \'onerror\'">',
  '<img src="https://tracker.example/pixel.gif">',
  '',
  '[clique ici](javascript:window.__pwned=1) et <a href="javascript:window.__pwned=2">là</a>',
  '',
  '## Installer',
  '',
  '- Depuis Escouade : Réglages › Plugins › Parcourir le store.',
  '- Voir [la doc](docs/setup.md).',
].join('\n');

async function keep(dir, bytes, ext) {
  const hash = sha256(bytes);
  const path = join(dir, 'sha256', hash.slice(0, 2), `${hash}${ext}`);
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, bytes);
  return hash;
}

/**
 * Seeds an author and three plugins: Slack (a program, two versions, review asked, a trapped README with an image),
 * ntfy (declarative, a webhook), Nord (a theme, asks for nothing, with its icon: PLG-13). Gives their ids and the
 * author's login.
 */
export async function seedCatalog({ url = process.env.DATABASE_URL, archives = process.env.ARCHIVE_DIR ?? './.data/archives' } = {}) {
  const run = randomBytes(3).toString('hex');
  const pool = new pg.Pool({ connectionString: url, max: 2 });
  const q = (sql, params) => pool.query(sql, params);
  const login = `lea-martin-${run}`;
  const githubId = 70_000_000 + Number.parseInt(run, 16);
  const { rows } = await q(
    `insert into accounts (github_id, login, name) values ($1, $2, $3)
     on conflict (github_id) do update set login = excluded.login returning id`,
    [githubId, login, 'Léa Martin'],
  );
  const accountId = rows[0].id;
  const ids = {
    slack: `dev.lmartin${run}.slack`,
    ntfy: `dev.sroux${run}.ntfy`,
    nord: `io.github.amoreau${run}.nord`,
  };
  const plugins = [
    {
      id: ids.slack,
      repo: `${login}/escouade-slack`,
      categories: ['notifications'],
      keywords: ['chat', 'équipe'],
      versions: ['1.1.0', '1.2.0'],
      review: true,
      manifest: {
        name: { fr: `Slack ${run}`, en: `Slack ${run}` },
        description: {
          fr: 'Les tickets à tester et les agents qui attendent, dans un salon Slack : un salon par projet, un fil par ticket.',
          en: 'Tickets to test and agents waiting, in a Slack channel: one channel per project, one thread per ticket.',
        },
        run: { command: 'bin/escouade-slack' },
        permissions: { network: ['hooks.slack.com'], events: ['ticket.column', 'agent.waiting'], secrets: ['webhook'] },
        contributes: { secrets: [{ key: 'webhook', title: { fr: 'Adresse du webhook', en: 'Webhook address' } }] },
      },
      files: { 'README.md': TRAPPED, 'store/1.png': png(120, 60), 'docs/setup.md': '# Régler\n\nLe salon par projet.\n' },
      store: { docs: 'docs' },
    },
    {
      id: ids.ntfy,
      repo: `sroux-${run}/escouade-ntfy`,
      categories: ['notifications'],
      keywords: ['téléphone'],
      versions: ['0.3.0'],
      review: false,
      manifest: {
        name: `ntfy ${run}`,
        description: {
          fr: 'Les agents qui attendent, sur ton téléphone, par ntfy.sh.',
          en: 'Agents waiting, on your phone, through ntfy.sh.',
        },
        permissions: { events: ['agent.waiting'], network: ['ntfy.sh'] },
        contributes: { webhooks: [{ event: 'agent.waiting', url: 'https://ntfy.sh/{topic}' }] },
      },
      files: { 'README.fr.md': '# ntfy\n\nEn français.\n', 'README.en.md': '# ntfy\n\nIn English.\n' },
    },
    {
      id: ids.nord,
      repo: `amoreau-${run}/escouade-nord`,
      categories: ['themes'],
      keywords: [],
      versions: ['2.0.0'],
      review: false,
      manifest: {
        name: `Nord ${run}`,
        description: { fr: 'La palette Nord, en clair et en sombre.', en: 'The Nord palette, light and dark.' },
        contributes: { themes: [{ id: 'nord', name: 'Nord', path: 'nord.json' }] },
      },
      files: { 'README.md': '# Nord\n', 'icon.png': png(512, 512) },
      icon: 'icon.png',
    },
  ];
  const repoBase = 80_000_000 + Number.parseInt(run, 16);
  let n = 0;
  for (const p of plugins) {
    const repoId = repoBase + n++;
    await q(`insert into plugins (id, account_id, repo_id, repo_full_name, categories, keywords) values ($1, $2, $3, $4, $5, $6)`, [
      p.id,
      accountId,
      repoId,
      p.repo,
      JSON.stringify(p.categories),
      JSON.stringify(p.keywords),
    ]);
    for (const [i, version] of p.versions.entries()) {
      const commit = sha256(`${p.id}@${version}@${run}`).slice(0, 40);
      const manifest = {
        $schema: 'https://escouade.dev/schemas/plugin-1.json',
        id: p.id,
        version,
        author: { name: 'Léa Martin' },
        license: 'MIT',
        repository: `https://github.com/${p.repo}`,
        escouade: { api: '1', min: '2.1.0' },
        ...p.manifest,
        ...(p.store ? { store: p.store } : {}),
        ...(p.icon ? { icon: p.icon } : {}),
      };
      const files = { 'escouade-plugin.json': JSON.stringify(manifest), LICENSE: MIT, ...p.files };
      const gz = tarball(files, { commit, top: `${p.repo.replace('/', '-')}-${commit.slice(0, 7)}` });
      const archive = await keep(archives, gz, '.tar.gz');
      const shot = p.files['store/1.png'] ? await keep(archives, p.files['store/1.png'], '.png') : null;
      // The icon as the checks keep it, at 256 and 64 px (its re-encoding is the Vitest's).
      const icon = p.icon ? { 256: await keep(archives, png(256, 256), '.png'), 64: await keep(archives, png(64, 64), '.png') } : null;
      // Published now, one minute apart: the newest of the base, as « Nouveaux » shows them.
      const published = new Date(Date.now() - (p.versions.length - i) * 60_000 - n * 1000);
      await q(
        `insert into versions (plugin_id, account_id, installation_id, repo_id, repo_full_name, tag, "commit", version, state,
           manifest, manifest_sha256, archive_sha256, archive_size, binaries, screenshots, version_json, version_sha256, review,
           checked_at, published_at, icon)
         values ($1, $2, 1, $3, $4, $5, $6, $7, 'published', $8, $9, $10, $11, '{}', $12, '{}', $13, $14, $15, $15, $16)`,
        [
          p.id,
          accountId,
          repoId,
          p.repo,
          `v${version}`,
          commit,
          version,
          JSON.stringify(manifest),
          sha256(JSON.stringify(manifest)),
          archive,
          gz.length,
          JSON.stringify(shot ? [{ path: 'store/1.png', sha256: shot, format: 'png', width: 120, height: 60 }] : []),
          sha256(`${p.id}${version}${run}`),
          p.review,
          published,
          icon ? JSON.stringify(icon) : null,
        ],
      );
    }
  }
  await pool.end();
  return { run, login, ids };
}

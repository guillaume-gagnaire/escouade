import { homedir, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DEFAULT_IMAGE, DEFAULT_TIMEOUT, findCli, findEngine } from './bench';
import { parseTarget } from './download';
import { review } from './review';

// `npm run plugin-review -- <id>@<version>` (from the repository's root or from store/): the local review of a version
// of the store (docs/STORE.md §4.2). `/review-plugin` in Claude Code runs it, then reads the code and writes the
// findings. What it reads from the environment, all optional:
//   ESCOUADE_STORE_URL           the store (https://store.escouade.dev)
//   ESCOUADE_REVIEW_HOME         the reports (~/.escouade-review)
//   ESCOUADE_REVIEW_WORK         where the versions are unpacked (<temp>/escouade-review)
//   ESCOUADE_REVIEW_ENGINE       docker, podman, or none (docker, else podman)
//   ESCOUADE_PLUGIN_LINUX        the Linux x64 `escouade-plugin` mounted in the container
//   ESCOUADE_REVIEW_IMAGE        the container's image (docker.io/library/node:22-trixie-slim)
//   ESCOUADE_REVIEW_GITHUB_API   GitHub's API, and GITHUB_TOKEN or GH_TOKEN for its limits

const USAGE = `npm run plugin-review -- <id>@<version> [--since <version>]

  Télécharge la version depuis le store, vérifie ses empreintes, l'extrait hors du dépôt sans rien exécuter,
  prépare les pistes, passe le banc dans un conteneur sans réseau, fait le diff avec la dernière version vérifiée
  (ou --since), et écrit le rapport dans ~/.escouade-review/<id>/<version>.md.
`;

async function main(argv: string[]): Promise<number> {
  if (argv.includes('-h') || argv.includes('--help')) {
    process.stdout.write(USAGE);
    return 0;
  }
  const sinceAt = argv.indexOf('--since');
  const since = sinceAt >= 0 ? argv[sinceAt + 1] : undefined;
  const target = parseTarget(argv.find((a, i) => !a.startsWith('--') && (sinceAt < 0 || i !== sinceAt + 1)) ?? '');
  if (!target || (sinceAt >= 0 && !since)) {
    process.stderr.write(USAGE);
    return 2;
  }
  const env = process.env;
  const { engine, why } = findEngine(env);
  const here = dirname(fileURLToPath(import.meta.url));
  const done = await review({
    ...target,
    since: since?.replace(/^v/, ''),
    source: {
      store: (env.ESCOUADE_STORE_URL ?? 'https://store.escouade.dev').replace(/\/$/, ''),
      githubApi: (env.ESCOUADE_REVIEW_GITHUB_API ?? 'https://api.github.com').replace(/\/$/, ''),
      githubToken: env.GITHUB_TOKEN ?? env.GH_TOKEN,
    },
    work: resolve(env.ESCOUADE_REVIEW_WORK ?? join(tmpdir(), 'escouade-review')),
    reports: resolve(env.ESCOUADE_REVIEW_HOME ?? join(homedir(), '.escouade-review')),
    bench: { engine, cli: findCli(env), image: env.ESCOUADE_REVIEW_IMAGE ?? DEFAULT_IMAGE, timeout: DEFAULT_TIMEOUT },
    benchWhy: why,
    repo: resolve(here, '../../..'),
  });
  const d = done.data;
  const lines = [`Rapport : ${done.report}`];
  if (d.integrity) lines.push(`Refusé : ${d.integrity}`);
  else {
    lines.push(`Résumé public : ${done.summary}`, `Pistes et données : ${done.json}`, `Plugin extrait : ${d.folder}`);
    if (d.base) lines.push(`Diff depuis la v${d.base.version} : ${d.base.diff}`);
    if (d.bench && !d.bench.played) lines.push(`Banc non lancé : ${d.bench.why}`);
  }
  process.stdout.write(`${lines.join('\n')}\n`);
  return done.code;
}

main(process.argv.slice(2)).then(
  // exitCode, not exit(): on Windows, exit() while fetch closes its sockets trips an assertion of libuv.
  (code) => (process.exitCode = code),
  (e: unknown) => {
    process.stderr.write(`plugin-review : ${e instanceof Error ? e.message : String(e)}\n`);
    process.exitCode = 2;
  },
);

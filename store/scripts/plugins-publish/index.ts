import { spawn } from 'node:child_process';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { firstPartyRepo } from '../../server/utils/config';
import { readToken } from '../plugin-review/admin';
import { publish, Refused, Stop, storeTool } from './publish';

// `npm run plugins:publish -- [--changed] [<court>…]` (from the repository's root or from store/): the first hand's
// plugins to the store, one after the other, each as a third party's (docs/PLUGINS.md §8.6.2). The chief runs it, never
// the agent of a ticket: it tags, pushes, submits, publishes and signs for real. What it reads from the environment, all
// optional, beyond what `plugin-review` reads (docs/STORE.md §4.2):
//   ESCOUADE_STORE_URL        the store (https://store.escouade.dev)
//   ESCOUADE_STORE_TOKEN      the administration token (else the system's keychain, saved by `plugin-review -- token`)
//   ESCOUADE_REVIEW_HOME      the reports (~/.escouade-review)
//   STORE_FIRST_PARTY_REPO    the app's repository on GitHub
//   ESCOUADE_PUBLISH_REPO     its clone (this one)
//   ESCOUADE_PUBLISH_POLL_MS  how often the checks are asked after (5 s)
//   ESCOUADE_PUBLISH_WAIT_MS  how long they are waited for (30 min)

const USAGE = `npm run plugins:publish -- [--changed] [<court>…]

  Fait passer les plugins de la première main au store, comme un tiers, l’un après l’autre : l’étiquette
  plugins/<court>/vX.Y.Z sur le commit de main poussé, la soumission, les contrôles, la publication (non vérifiée), la revue
  locale, puis, sur la ligne **Verdict final : Vérifier** du rapport, report et sign.

  --changed   tous les dossiers de plugins/ dont la version n’est pas encore signée au store
  <court>     ces dossiers-là (claude, trello…)

  Refuse un arbre sale ou un commit qui n’est pas sur origin/main. S’arrête au premier contrôle rouge, au premier rapport
  sans ce verdict : relancée, la commande reprend là, sans rien refaire.

  Code de sortie : 0, tout est fait ; 1, arrêtée, à reprendre ; 2, refusée, rien n’est fait.
`;

const whole = (name: string, fallback: number) => {
  const n = Number(process.env[name]);
  return Number.isFinite(n) && n > 0 ? n : fallback;
};

async function main(argv: string[]): Promise<number> {
  if (argv.includes('-h') || argv.includes('--help')) {
    process.stdout.write(USAGE);
    return 0;
  }
  const names = argv.filter((a) => a !== '--changed');
  if ((!names.length && !argv.includes('--changed')) || names.some((n) => n.startsWith('-'))) {
    process.stderr.write(USAGE);
    return 2;
  }
  const env = process.env;
  const token = readToken(env);
  if (!token) {
    process.stderr.write('Pas de jeton d’administration : crée-le dans ton compte du store, puis npm run plugin-review -- token.\n');
    return 2;
  }
  const here = dirname(fileURLToPath(import.meta.url));
  const store = (env.ESCOUADE_STORE_URL ?? 'https://store.escouade.dev').replace(/\/$/, '');
  const log = (line: string) => process.stdout.write(`${line}\n`);
  try {
    const done = await publish({
      repo: resolve(env.ESCOUADE_PUBLISH_REPO ?? resolve(here, '../../..')),
      repository: firstPartyRepo(env),
      names,
      source: { store, githubApi: (env.ESCOUADE_REVIEW_GITHUB_API ?? 'https://api.github.com').replace(/\/$/, '') },
      reports: resolve(env.ESCOUADE_REVIEW_HOME ?? join(homedir(), '.escouade-review')),
      tool: storeTool(store, token),
      // The command of today, its own process: it reads the signing key, this one never does.
      review: (args) =>
        new Promise((settle, fail) =>
          spawn(process.execPath, ['--import', 'tsx', resolve(here, '../plugin-review/index.ts'), ...args], {
            cwd: resolve(here, '../..'),
            stdio: 'inherit',
          })
            .on('error', fail)
            .on('close', (code) => settle(code ?? 1)),
        ),
      pollMs: whole('ESCOUADE_PUBLISH_POLL_MS', 5_000),
      waitMs: whole('ESCOUADE_PUBLISH_WAIT_MS', 30 * 60_000),
      log,
    });
    log(done ? `Fini : ${done} plugin(s) publié(s) avec leur sceau.` : 'Rien à publier : tout est déjà au store, signé.');
    return 0;
  } catch (e) {
    if (e instanceof Stop) {
      log(`Arrêt : ${e.message}`);
      return 1;
    }
    if (e instanceof Refused) {
      process.stderr.write(`Refusé : ${e.message}\n`);
      return 2;
    }
    throw e;
  }
}

main(process.argv.slice(2)).then(
  // exitCode, not exit(): on Windows, exit() while fetch closes its sockets trips an assertion of libuv.
  (code) => (process.exitCode = code),
  (e: unknown) => {
    process.stderr.write(`plugins:publish : ${e instanceof Error ? e.message : String(e)}\n`);
    process.exitCode = 2;
  },
);

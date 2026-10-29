import { homedir, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DEFAULT_IMAGE, DEFAULT_TIMEOUT, findCli, findEngine } from './bench';
import { parseTarget, type Source } from './download';
import { keysIn, readToken, revoke, saveToken, sendReport, signingKey, signVersion, workDir, type Admin } from './admin';
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
// And for what is sent to the store (report, sign, revoke, lift, revoke-key):
//   ESCOUADE_STORE_TOKEN         the administration token (else the system's keychain, saved by `token`)
//   ESCOUADE_SIGNING_KEY         the store's secret key (~/.escouade-signing/store.key), used by minisign alone
//   ESCOUADE_MINISIGN            the minisign command (minisign)

const USAGE = `npm run plugin-review -- <id>@<version> [--since <version>]
       npm run plugin-review -- report <id>@<version>
       npm run plugin-review -- sign <id>@<version>
       npm run plugin-review -- revoke <id>[@<version>] --disable|--unverify --fr <raison> --en <reason> [--quiet]
       npm run plugin-review -- lift <id>[@<version>] --disable|--unverify
       npm run plugin-review -- revoke-key <id de clé> --fr <raison> --en <reason>
       npm run plugin-review -- token

  Télécharge la version depuis le store, vérifie ses empreintes, l'extrait hors du dépôt sans rien exécuter,
  prépare les pistes, passe le banc dans un conteneur sans réseau, fait le diff avec la dernière version vérifiée
  (ou --since), et écrit le rapport dans ~/.escouade-review/<id>/<version>.md.

  report : envoie le rapport à la file du store (les administrateurs seuls le lisent).
  sign : après ta décision « Vérifier », signe version.json avec la clé du store (minisign demande son mot de
  passe), vérifie la signature avec les clés publiques de store/keys/, puis la dépose avec le résumé public.
  revoke, lift, revoke-key : signent et déposent la liste de révocation suivante (serial + 1).
  token : garde le jeton d’administration dans le trousseau du système (collé, jamais affiché).
`;

async function main(argv: string[]): Promise<number> {
  if (argv.includes('-h') || argv.includes('--help')) {
    process.stdout.write(USAGE);
    return 0;
  }
  if (['report', 'sign', 'revoke', 'lift', 'revoke-key', 'token'].includes(argv[0] ?? '')) return admin(argv);
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

/** A line typed without echo (the token): raw mode on a terminal, else the first line of the standard input. */
function hidden(prompt: string): Promise<string> {
  return new Promise((done) => {
    const input = process.stdin;
    if (!input.isTTY) {
      let all = '';
      input.setEncoding('utf8');
      input.on('data', (c) => (all += c));
      input.on('end', () => done(all.split(/\r?\n/)[0]!.trim()));
      return;
    }
    process.stderr.write(prompt);
    input.setRawMode(true);
    input.setEncoding('utf8');
    let typed = '';
    const on = (c: string) => {
      for (const ch of c) {
        if (ch === '\r' || ch === '\n' || ch === '\u0004') {
          input.setRawMode(false);
          input.pause();
          input.off('data', on);
          process.stderr.write('\n');
          return done(typed.trim());
        }
        if (ch === '\u0003') process.exit(130);
        typed = ch === '\u007f' || ch === '\b' ? typed.slice(0, -1) : typed + ch;
      }
    };
    input.on('data', on);
    input.resume();
  });
}

const flag = (argv: string[], name: string) => {
  const i = argv.indexOf(name);
  return i >= 0 ? argv[i + 1] : undefined;
};

/** report, sign, revoke, lift, revoke-key, token: what is sent to the store, with the administration token. */
async function admin(argv: string[]): Promise<number> {
  const env = process.env;
  const [command, what] = argv;
  if (command === 'token') {
    const token = await hidden('Jeton d’administration (montré une fois par le store) : ');
    if (!token.startsWith('esc_admin_')) {
      process.stderr.write('Ce n’est pas un jeton d’administration du store (esc_admin_…).\n');
      return 2;
    }
    if (!saveToken(token)) {
      process.stderr.write(
        'Le trousseau du système ne l’a pas pris : garde-le dans ton gestionnaire de mots de passe et passe-le par ESCOUADE_STORE_TOKEN.\n',
      );
      return 1;
    }
    process.stdout.write('Jeton gardé dans le trousseau du système.\n');
    return 0;
  }
  const token = readToken(env);
  if (!token) {
    process.stderr.write('Pas de jeton d’administration : crée-le dans ton compte du store, puis npm run plugin-review -- token.\n');
    return 2;
  }
  const here = dirname(fileURLToPath(import.meta.url));
  const source: Source = {
    store: (env.ESCOUADE_STORE_URL ?? 'https://store.escouade.dev').replace(/\/$/, ''),
    githubApi: (env.ESCOUADE_REVIEW_GITHUB_API ?? 'https://api.github.com').replace(/\/$/, ''),
  };
  const a: Admin = {
    source,
    token,
    reports: resolve(env.ESCOUADE_REVIEW_HOME ?? join(homedir(), '.escouade-review')),
    keys: keysIn(env.STORE_KEYS_DIR ?? resolve(here, '../../keys')),
    minisign: { cmd: env.ESCOUADE_MINISIGN ?? 'minisign', key: signingKey(env) },
    work: resolve(workDir(env)),
  };
  if (command === 'revoke-key') {
    const fr = flag(argv, '--fr');
    const en = flag(argv, '--en');
    if (!what || !fr || !en) {
      process.stderr.write(USAGE);
      return 2;
    }
    const done = await revoke(a, {
      id: '',
      versions: '*',
      action: 'unverify',
      reason: { fr, en },
      revokeKeys: [what.toUpperCase()],
      notify: false,
    });
    process.stdout.write(`Liste de révocation n° ${done.serial} déposée : la clé ${what.toUpperCase()} n’est plus crue.\n`);
    return 0;
  }
  const target = /^([a-z0-9][a-z0-9.-]*[a-z0-9])(?:@v?(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?))?$/.exec(what ?? '');
  if (!target || ((command === 'report' || command === 'sign') && !target[2])) {
    process.stderr.write(USAGE);
    return 2;
  }
  const id = target[1]!;
  const version = target[2];
  if (command === 'report') {
    const done = await sendReport(a, id, version!);
    process.stdout.write(`Rapport de ${id}@${version} déposé dans la file (verdict : ${done.verdict ?? 'à décider'}).\n`);
    return 0;
  }
  if (command === 'sign') {
    const done = await signVersion(a, id, version!);
    process.stdout.write(`${id}@${version} signée (clé ${done.keyId}) et déposée : elle porte le sceau « Vérifié par Escouade ».\n`);
    return 0;
  }
  const action = argv.includes('--disable') ? 'disable' : argv.includes('--unverify') ? 'unverify' : null;
  if (!action) {
    process.stderr.write(USAGE);
    return 2;
  }
  const done = await revoke(a, {
    id,
    versions: version ? [version] : '*',
    action,
    reason: { fr: flag(argv, '--fr') ?? '', en: flag(argv, '--en') ?? '' },
    lift: command === 'lift',
    notify: !argv.includes('--quiet'),
  });
  process.stdout.write(`Liste de révocation n° ${done.serial} signée et déposée.\n`);
  return 0;
}

main(process.argv.slice(2)).then(
  // exitCode, not exit(): on Windows, exit() while fetch closes its sockets trips an assertion of libuv.
  (code) => (process.exitCode = code),
  (e: unknown) => {
    process.stderr.write(`plugin-review : ${e instanceof Error ? e.message : String(e)}\n`);
    process.exitCode = 2;
  },
);

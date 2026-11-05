import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { sha256 } from '../../server/utils/versions';
import { gh, type Runner } from './attest';

// escouade-js on the machine of the review (ESC-343). `escouade-plugin` embeds it when it is built: built without it, or
// with another one, it does not compile as the store did, and nothing can be signed. The CI of main builds it once, as
// the artifact `escouade-js`, kept 7 days: the review fetches the one `version.json` names into
// `src-tauri/escouade-js/target/` (git ignores that folder), and checks its fingerprint itself. `cargo build` then
// embeds it: that is the reader's step, minutes long, and the review says so.

const ARTIFACT = 'escouade-js';
const FILE = 'escouade-js.wasm';
/** How far back in main's artifacts the fingerprint is looked for. */
const RUNS = 10;
/** From `src-tauri/`, whose rust-toolchain.toml pins the compiler: `--manifest-path` from the root would take another. */
const BUILD = 'cd src-tauri && cargo build --bin escouade-plugin';
/** What is left once escouade-js is in its folder. */
const AFTER = `\`${BUILD}\`, puis relance la revue (ESCOUADE_PLUGIN nomme le binaire de src-tauri/target s’il y en a un autre sur le PATH)`;

export const engineFolder = (repo: string) => join(repo, 'src-tauri', 'escouade-js', 'target');

/** How to get the escouade-js of fingerprint `engine` by hand. */
export const engineSteps = (engine: string) =>
  `il faut l’escouade-js ${engine} : prends l’artefact \`escouade-js\` de la CI de main (\`gh run download <run> --name escouade-js --dir src-tauri/escouade-js/target\`) ou construis-le (\`sh scripts/escouade-js.sh\`), puis ${AFTER}`;

export type Fetched = { state: 'here' } | { state: 'fetched'; run: number } | { state: 'not-found'; why: string };

interface Artifacts {
  artifacts?: { expired?: boolean; workflow_run?: { id?: number; head_branch?: string } }[];
}

/** The runs of main that still hold the artifact, the latest first; why not, in words. */
function runsOf(repository: string, run: Runner): number[] | string {
  const r = run(['api', `repos/${repository}/actions/artifacts?name=${ARTIFACT}&per_page=100`]);
  if (r.error) return '`gh` introuvable';
  if (r.status !== 0) return `\`gh api\` a échoué : ${(r.stderr || r.stdout).trim().split('\n').slice(-1)[0]}`;
  let answer: Artifacts;
  try {
    answer = JSON.parse(r.stdout) as Artifacts;
  } catch {
    return '`gh api` n’a pas répondu en JSON';
  }
  const runs = (answer.artifacts ?? [])
    .filter((a) => !a.expired && a.workflow_run?.head_branch === 'main')
    .map((a) => a.workflow_run!.id)
    .filter((id): id is number => typeof id === 'number');
  return [...new Set(runs)].slice(0, RUNS);
}

const isEngine = (file: string, engine: string) => existsSync(file) && sha256(readFileSync(file)) === engine;

/**
 * Puts the escouade-js of fingerprint `engine` in `folder`, from the CI of main of `repository`: nothing is kept that
 * does not have this very fingerprint, whatever the artifact says of itself.
 */
export function fetchEngine(engine: string, folder: string, repository: string, run: Runner = gh): Fetched {
  if (isEngine(join(folder, FILE), engine)) return { state: 'here' };
  const runs = runsOf(repository, run);
  if (typeof runs === 'string') return { state: 'not-found', why: runs };
  for (const id of runs) {
    const dir = mkdtempSync(join(tmpdir(), 'escouade-review-engine-'));
    try {
      const r = run(['run', 'download', String(id), '--repo', repository, '--name', ARTIFACT, '--dir', dir]);
      if (r.status !== 0 || !isEngine(join(dir, FILE), engine)) continue;
      mkdirSync(folder, { recursive: true });
      copyFileSync(join(dir, FILE), join(folder, FILE));
      if (existsSync(join(dir, `${FILE}.sha256`))) copyFileSync(join(dir, `${FILE}.sha256`), join(folder, `${FILE}.sha256`));
      return { state: 'fetched', run: id };
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  }
  return { state: 'not-found', why: `aucun artefact de main ne l’a parmi les ${runs.length} encore gardés (ils durent 7 jours)` };
}

/** What the report says of an escouade-js that is not the one the store compiled with, after trying to fetch it. */
export function engineHelp(engine: string, folder: string, repository: string, run: Runner = gh): string {
  const got = fetchEngine(engine, folder, repository, run);
  if (got.state === 'here') return `l’escouade-js ${engine} est déjà dans ${folder} : ${AFTER}`;
  if (got.state === 'fetched')
    return `l’escouade-js ${engine} est récupéré de la CI (exécution ${got.run}), empreinte vérifiée, dans ${folder} : ${AFTER}`;
  return `${got.why} ; ${engineSteps(engine)}`;
}

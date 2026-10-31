import { iconPath, message, RESERVED_PREFIX, validateText } from '../../../schemas/plugin-manifest.mjs';
import { ICON_DECODE_MAX, imageInfo, SCREENSHOT_BYTES, SCREENSHOT_WIDTH, SCREENSHOTS_MAX } from './images';
import { judgeLicense } from './licenses';
import type { ArchiveProblem, ArchiveRead } from './tar';
import { folderName, tagFor, tagsOf } from '../../shared/folder';
import { compareVersions, type Platform } from './versions';

// The ten checks of a submitted version (docs/STORE.md, §3.3), on the archive the store froze, read in memory:
// nothing of the plugin runs. Each says, in French and in English, what failed, where (the field's path), why and how
// to fix it, with the page of the documentation (§3.5). They read what the worker fetched before (the archive, the
// license GitHub detects, the release's binaries and their attestations, the plugins already on the store); they
// fetch nothing themselves.

export type Lang = 'fr' | 'en';
export type Text = Record<Lang, string>;
export const t = (fr: string, en: string): Text => ({ fr, en });

export const CHECK_KEYS = [
  'manifest',
  'version',
  'id',
  'repository',
  'license',
  'readme',
  'screenshots',
  'archive',
  'permissions',
  'binaries',
] as const;
export type CheckKey = (typeof CHECK_KEYS)[number];

/** `run`: not done yet; `hold`: passes, but a person must look before it is published (a name close to another's). */
export type CheckStatus = 'pass' | 'fail' | 'hold' | 'skip' | 'run';

export interface Problem {
  /** The field or the file at fault: `permissions.network[0]`, `README.md`. */
  path?: string;
  why: Text;
  fix: Text;
}

export interface CheckResult {
  key: CheckKey;
  status: CheckStatus;
  detail: Text;
  problems: Problem[];
}

export const CHECK_NAMES: Record<CheckKey, Text> = {
  manifest: t('Manifeste', 'Manifest'),
  version: t('Version', 'Version'),
  id: t('Identifiant', 'Identifier'),
  repository: t('Dépôt', 'Repository'),
  license: t('Licence', 'License'),
  readme: t('README', 'README'),
  screenshots: t('Captures', 'Screenshots'),
  archive: t('Archive', 'Archive'),
  permissions: t('Permissions', 'Permissions'),
  binaries: t('Binaires', 'Binaries'),
};

/** The page that explains publishing, which every rejection points to. */
export const DOCS: Text = t('https://escouade.dev/docs/plugins/publishing', 'https://escouade.dev/en/docs/plugins/publishing');

export const MANIFEST_FILE = 'escouade-plugin.json';
export const SYSTEMS = ['windows', 'macos', 'linux'] as const;
export type System = (typeof SYSTEMS)[number];

/** A binary of the release, fetched and kept by the worker, with what GitHub's attestation says of it. */
export interface FetchedBinary {
  platform: Platform;
  name: string;
  sha256: string;
  size: number;
  /** `attested`: a provenance attestation of this file, built from this commit of this repository. */
  attestation: { state: 'attested'; workflow: string } | { state: 'none' } | { state: 'mismatch'; commit?: string; repository?: string };
}

export interface Snapshot {
  /** `owner/name` of the repository submitted. */
  repository: string;
  /** The plugin's folder in the repository, `''` at its root (ESC-313). `archive.read` is that folder alone. */
  path?: string;
  /** The whole repository's archive, when the plugin is in a folder: its root's README and LICENSE stand in. */
  root?: ArchiveRead | null;
  /** The link the folder goes through, if any: the store does not follow it. */
  pathLink?: string | null;
  tag: string;
  commit: string;
  archive: { size: number; read: ArchiveRead };
  /** The license GitHub detects in the commit: its SPDX id, `NOASSERTION`, or null without a license file. */
  license: string | null;
  /** The binaries of the release attached to the tag, as the worker kept them (empty without a release). */
  binaries: FetchedBinary[];
  /** The release's file names, to say which one was missing. Null when the tag has no release. */
  releaseAssets: string[] | null;
  /** The release's files for a system that the store did not take, past their size. */
  oversized?: string[];
  /** The screenshots whose header read but which do not decode. */
  undecodable?: string[];
  /** The icon passed its header's checks but did not decode (PLG-13). */
  iconUndecodable?: boolean;
  /** Whether the repository submitted is the app's (STORE_FIRST_PARTY_REPO, by its GitHub id; ESC-314): its plugins
   * alone take the ids `dev.escouade.*`. */
  firstParty?: boolean;
  /** The plugin already on the store under the manifest's id, if any: whose it is, from which repository and folder. */
  existing: { mine: boolean; repository: string; path?: string } | null;
  /** The versions of this id already taken (published, waiting, or held): a number is never used twice. */
  taken: string[];
  /** The highest version published of this id. */
  latest: string | null;
  /** The plugins of other accounts, for names too close (typosquatting). */
  others: { id: string; name: string }[];
}

type Manifest = Record<string, unknown>;

const fail = (key: CheckKey, detail: Text, problems: Problem[]): CheckResult => ({ key, status: 'fail', detail, problems });
const pass = (key: CheckKey, detail: Text): CheckResult => ({ key, status: 'pass', detail, problems: [] });
const skip = (key: CheckKey, detail: Text): CheckResult => ({ key, status: 'skip', detail, problems: [] });
const NO_MANIFEST = t('Sans manifeste lisible, ce contrôle attend.', 'Without a manifest that reads, this check waits.');
const code = (s: string) => `\`${s}\``;
const plural = (n: number, fr: string, en: string): Text => t(`${n} ${fr}${n > 1 ? 's' : ''}`, `${n} ${en}${n > 1 ? 's' : ''}`);
/** Where the plugin is in its repository, as a sentence says it: at its root, or in its folder. */
const where = (path = ''): Text =>
  path ? t(`dans le dossier ${code(path)}`, `in the folder ${code(path)}`) : t('à la racine du dépôt', 'at the root of the repository');
const folderOrRoot = (path = ''): Text => (path ? t(`le dossier ${code(path)}`, `the folder ${code(path)}`) : t('la racine', 'the root'));

/** The manifest of the archive, parsed, or null if it is missing or no JSON object. */
export function manifestOf(read: ArchiveRead): Manifest | null {
  const raw = read.files.get(MANIFEST_FILE);
  if (!raw) return null;
  try {
    const v = JSON.parse(raw.toString('utf8'));
    return v && typeof v === 'object' && !Array.isArray(v) ? v : null;
  } catch {
    return null;
  }
}

/** A text of the manifest (`name`), in English first as the app does. */
export function textOf(v: unknown): string {
  if (typeof v === 'string') return v;
  if (v && typeof v === 'object') {
    const o = v as Record<string, unknown>;
    for (const l of ['en', 'fr']) if (typeof o[l] === 'string' && o[l]) return o[l] as string;
  }
  return '';
}

/** The program `run` gives each system, and whether the archive holds it; the others come from the release. */
export function programs(m: Manifest | null, read: ArchiveRead): { system: System; program: string; inArchive: boolean }[] {
  const run = m?.run;
  if (!run || typeof run !== 'object') return [];
  const r = run as Record<string, unknown>;
  return SYSTEMS.flatMap((system) => {
    const program = typeof r[system] === 'string' ? (r[system] as string) : typeof r.command === 'string' ? (r.command as string) : null;
    return program ? [{ system, program, inArchive: read.files.has(program.replace(/^\.\//, '')) }] : [];
  });
}

/** The system and architecture a release's file is for, from its name (`escouade-slack-1.2.0-windows-x64.zip`). */
export function platformOf(name: string): Platform | null {
  const m = /(?:^|[-_.])(windows|macos|linux)[-_](x64|arm64)(?:[-_.]|$)/i.exec(name);
  return m ? (`${m[1]!.toLowerCase()}-${m[2]!.toLowerCase()}` as Platform) : null;
}

// ---- 1. The manifest ----

/** The icon the manifest declares (`icon`), as a path of the archive; null without one. */
export function iconOf(m: Manifest | null): string | null {
  return typeof m?.icon === 'string' ? iconPath(m.icon) : null;
}

function checkManifest(read: ArchiveRead, iconUndecodable = false, path = '', firstParty = false): CheckResult {
  const raw = read.files.get(MANIFEST_FILE);
  if (!raw)
    return fail('manifest', t(`${MANIFEST_FILE} manque`, `${MANIFEST_FILE} is missing`), [
      {
        path: path ? `${path}/${MANIFEST_FILE}` : MANIFEST_FILE,
        why: t(
          `Le store lit ${code(MANIFEST_FILE)} ${where(path).fr}, au commit de l’étiquette ; il n’y est pas.`,
          `The store reads ${code(MANIFEST_FILE)} ${where(path).en}, at the tag’s commit; it is not there.`,
        ),
        fix: path
          ? t(
              `Ajoute le manifeste dans ${code(path)} (ou choisis le dossier qui le porte), commite-le, puis pousse une nouvelle étiquette.`,
              `Add the manifest in ${code(path)} (or pick the folder that holds it), commit it, then push a new tag.`,
            )
          : t(
              'Ajoute le manifeste à la racine, commite-le, puis pousse une nouvelle étiquette.',
              'Add the manifest at the root, commit it, then push a new tag.',
            ),
      },
    ]);
  // The icon's header is the library's (a PNG, square, 256 px at least, 1 MB at most); its decoding is the store's.
  // The ids dev.escouade.* hold for the app's repository alone (checkId says why to the others).
  const v = validateText(raw.toString('utf8'), { file: (p: string) => read.files.get(p) ?? null, firstParty });
  const icon = v.ok ? iconOf(JSON.parse(raw.toString('utf8'))) : null;
  const info = icon ? imageInfo(read.files.get(icon)!) : null;
  if (info && info.width > ICON_DECODE_MAX)
    return fail('manifest', t('icône trop grande', 'icon too large'), [
      {
        path: 'icon',
        why: t(
          `${code(icon!)} fait ${info.width} px de côté ; le store en décode ${ICON_DECODE_MAX} au plus.`,
          `${code(icon!)} is ${info.width} px a side; the store decodes ${ICON_DECODE_MAX} at most.`,
        ),
        fix: t('Exporte-la en 512 px, puis pousse une nouvelle étiquette.', 'Export it at 512 px, then push a new tag.'),
      },
    ]);
  if (icon && iconUndecodable)
    return fail('manifest', t('icône illisible', 'icon unreadable'), [
      {
        path: 'icon',
        why: t(`${code(icon)} a l’en-tête d’un PNG, mais ne se décode pas.`, `${code(icon)} has a PNG’s header, but does not decode.`),
        fix: t('Exporte-la de nouveau en PNG, puis pousse une nouvelle étiquette.', 'Export it again as a PNG, then push a new tag.'),
      },
    ]);
  if (v.ok)
    return pass(
      'manifest',
      t(
        `${MANIFEST_FILE} au schéma plugin-1, le validateur d’Escouade passe`,
        `${MANIFEST_FILE} on the plugin-1 schema, Escouade’s validator passes`,
      ),
    );
  return fail(
    'manifest',
    t(`${plural(v.errors.length, 'champ', 'field').fr} à corriger`, `${plural(v.errors.length, 'champ', 'field').en} to fix`),
    v.errors.map((e) => ({
      path: e.path || MANIFEST_FILE,
      why: t(message(e, 'fr'), message(e, 'en')),
      fix: t(
        'Corrige ce champ (le schéma https://escouade.dev/schemas/plugin-1.json le complète dans ton éditeur), puis pousse une nouvelle étiquette.',
        'Fix this field (the schema https://escouade.dev/schemas/plugin-1.json completes it in your editor), then push a new tag.',
      ),
    })),
  );
}

// ---- 2. The version ----

function checkVersion(m: Manifest | null, s: Snapshot): CheckResult {
  if (!m || typeof m.version !== 'string') return skip('version', NO_MANIFEST);
  const v = m.version;
  const path = s.path ?? '';
  const fromTag = tagFor(s.tag, path);
  const advised = tagsOf(v, path)[0]!;
  if (fromTag === null)
    return fail(
      'version',
      t(`${s.tag} n’est pas une étiquette de ${folderOrRoot(path).fr}`, `${s.tag} is no tag of ${folderOrRoot(path).en}`),
      [
        {
          path: 'version',
          why: path
            ? t(
                `Dans un dépôt à plusieurs plugins, l’étiquette nomme son dossier : ${code(`${path}/v1.2.3`)}, ${code(`${folderName(path)}@1.2.3`)} ou ${code(`${folderName(path)}-v1.2.3`)} ; ${code(s.tag)} n’en est pas une.`,
                `In a repository with several plugins, the tag names its folder: ${code(`${path}/v1.2.3`)}, ${code(`${folderName(path)}@1.2.3`)} or ${code(`${folderName(path)}-v1.2.3`)}; ${code(s.tag)} is none of them.`,
              )
            : t(
                `La version d’un plugin à la racine est une étiquette ${code('v1.2.3')} ou ${code('1.2.3')} ; ${code(s.tag)} n’en est pas une.`,
                `A version of a plugin at the root is a tag ${code('v1.2.3')} or ${code('1.2.3')}; ${code(s.tag)} is none.`,
              ),
          fix: t(`Pousse l’étiquette ${code(advised)}.`, `Push the tag ${code(advised)}.`),
        },
      ],
    );
  if (v !== fromTag)
    return fail('version', t(`${v} dans le manifeste, ${s.tag} pour l’étiquette`, `${v} in the manifest, ${s.tag} for the tag`), [
      {
        path: 'version',
        why: t(
          `Le manifeste dit ${code(v)}, l’étiquette ${code(s.tag)} : une version du store est une étiquette, au même numéro (le « v » est facultatif).`,
          `The manifest says ${code(v)}, the tag ${code(s.tag)}: a version of the store is a tag, of the same number (the “v” is optional).`,
        ),
        fix: t(
          `Mets ${code('version')} à ${code(fromTag)}, ou pousse l’étiquette ${code(advised)}.`,
          `Set ${code('version')} to ${code(fromTag)}, or push the tag ${code(advised)}.`,
        ),
      },
    ]);
  if (s.taken.includes(v))
    return fail('version', t(`${v} est déjà prise`, `${v} is already taken`), [
      {
        path: 'version',
        why: t(
          `La version ${code(v)} de ce plugin est déjà sur le store : une version publiée ne change pas, son numéro ne se réutilise pas.`,
          `Version ${code(v)} of this plugin is already on the store: a published version never changes, its number is not used again.`,
        ),
        fix: t('Monte le numéro, puis pousse une nouvelle étiquette.', 'Raise the number, then push a new tag.'),
      },
    ]);
  if (s.latest && compareVersions(v, s.latest) <= 0)
    return fail('version', t(`${v} n’est pas plus grande que ${s.latest}`, `${v} is not greater than ${s.latest}`), [
      {
        path: 'version',
        why: t(
          `La dernière version publiée est ${code(s.latest)} ; une nouvelle version doit venir après elle (semver).`,
          `The last version published is ${code(s.latest)}; a new version must come after it (semver).`,
        ),
        fix: t(`Prends un numéro après ${code(s.latest)}.`, `Take a number after ${code(s.latest)}.`),
      },
    ]);
  return pass(
    'version',
    s.latest
      ? t(
          `${v} dans le manifeste, comme l’étiquette ; plus grande que ${s.latest}`,
          `${v} in the manifest, as the tag; greater than ${s.latest}`,
        )
      : t(`${v} dans le manifeste, comme l’étiquette ; la première`, `${v} in the manifest, as the tag; the first one`),
  );
}

// ---- 3. The identifier ----

const squash = (s: string) => s.toLowerCase().replace(/[^a-z0-9]/g, '');

function distance(a: string, b: string): number {
  const row = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i++) {
    let prev = row[0]!;
    row[0] = i;
    for (let j = 1; j <= b.length; j++) {
      const keep = row[j]!;
      row[j] = Math.min(row[j]! + 1, row[j - 1]! + 1, prev + (a[i - 1] === b[j - 1] ? 0 : 1));
      prev = keep;
    }
  }
  return row[b.length]!;
}

/** Whether two names are close enough to be mistaken: the same once the dashes go, or a letter or two apart. */
export function tooClose(a: string, b: string): boolean {
  const [x, y] = [squash(a), squash(b)];
  if (!x || !y) return false;
  if (x === y) return true;
  const n = Math.min(x.length, y.length);
  const room = n >= 8 ? 2 : n >= 4 ? 1 : 0;
  return room > 0 && distance(x, y) <= room;
}

/** The plugins of others whose id (its last word) or name is too close to this one's. */
export function closeNames(id: string, name: string, others: Snapshot['others']): Snapshot['others'] {
  const last = (s: string) => s.split('.').pop() ?? s;
  return others.filter((o) => o.id !== id && (tooClose(last(id), last(o.id)) || (!!name && !!o.name && tooClose(name, o.name))));
}

function checkId(m: Manifest | null, s: Snapshot): CheckResult {
  if (!m || typeof m.id !== 'string') return skip('id', NO_MANIFEST);
  const id = m.id;
  // Escouade's own hand comes from the app's repository (its plugins/<id>/, decision 21), compared by its GitHub id.
  if (id.startsWith(RESERVED_PREFIX) && !s.firstParty)
    return fail('id', t(`${id} est réservé`, `${id} is reserved`), [
      {
        path: 'id',
        why: t(
          `Les identifiants ${code('dev.escouade.*')} sont ceux des plugins d’Escouade.`,
          `The ids ${code('dev.escouade.*')} are those of Escouade’s own plugins.`,
        ),
        fix: t(
          `Prends un identifiant à toi, en nom de domaine inversé (${code('io.github.<ton-login>.<plugin>')}).`,
          `Take an id of your own, as a reversed domain name (${code('io.github.<your-login>.<plugin>')}).`,
        ),
      },
    ]);
  if (s.existing && !s.existing.mine)
    return fail('id', t(`${id} est à un autre auteur`, `${id} belongs to another author`), [
      {
        path: 'id',
        why: t(
          `Un autre compte publie déjà ${code(id)} : un identifiant reste à son auteur, même après le retrait du plugin.`,
          `Another account already publishes ${code(id)}: an id stays its author’s, even once the plugin is removed.`,
        ),
        fix: t('Choisis un autre identifiant.', 'Choose another id.'),
      },
    ]);
  // A plugin is (repository, folder). Its author may change it (ESC-313: a repository of theirs where the App is
  // installed, a folder): the version passes, says so, and goes to the review; the store notes the change at its
  // publication. The versions before keep their own version.json, and their repository.
  const sourceOf = (repo: string, path = '') => (path ? `${repo}/${path}` : repo);
  const moved =
    s.existing && (s.existing.repository.toLowerCase() !== s.repository.toLowerCase() || (s.existing.path ?? '') !== (s.path ?? ''))
      ? { from: sourceOf(s.existing.repository, s.existing.path), to: sourceOf(s.repository, s.path) }
      : null;
  const close = closeNames(id, textOf(m.name), s.others);
  if (close.length)
    return {
      key: 'id',
      status: 'hold',
      detail: t(
        `${id} est proche de ${close.map((c) => c.id).join(', ')} : une personne le regarde avant la publication`,
        `${id} is close to ${close.map((c) => c.id).join(', ')}: a person looks at it before it is published`,
      ),
      problems: [
        {
          path: 'id',
          why: t(
            'Un nom trop proche de celui d’un autre plugin peut tromper (typosquatting) : le store le fait relire avant de le publier.',
            'A name too close to another plugin’s may mislead (typosquatting): the store has it read before publishing it.',
          ),
          fix: t(
            'Rien à faire si le nom est le bon : la file de revue le libère. Sinon, change de nom et pousse une nouvelle étiquette.',
            'Nothing to do if the name is right: the review queue releases it. Otherwise, change the name and push a new tag.',
          ),
        },
      ],
    };
  if (moved)
    return pass(
      'id',
      t(
        `${id} est à toi ; sa source change, de ${moved.from} à ${moved.to} : noté à la publication, cette version repasse en revue`,
        `${id} is yours; its source changes, from ${moved.from} to ${moved.to}: noted at publication, this version goes to the review again`,
      ),
    );
  return pass('id', s.existing ? t(`${id} est à toi`, `${id} is yours`) : t(`${id} est libre`, `${id} is free`));
}

// ---- 4. The repository ----

/** `https://github.com/Owner/Name(.git)(/)` → `owner/name`. */
export function repoOfUrl(url: string): string | null {
  const m = /^https?:\/\/(?:www\.)?github\.com\/([\w.-]+)\/([\w.-]+?)(?:\.git)?(?:\/tree\/.+?)?\/?$/i.exec(url.trim());
  return m ? `${m[1]}/${m[2]}`.toLowerCase() : null;
}

/**
 * The folder a manifest's `repository` names (ESC-313): `https://github.com/o/r` names none (`null`, any folder of
 * the repository); `https://github.com/o/r/tree/<ref>/<folder>` names `<folder>`, whatever the ref (a branch, the tag);
 * `…/tree/<ref>` names the root (`''`). A ref with a slash is read as the longest one that leaves the plugin's folder.
 */
export function folderOfUrl(url: string, path: string): string | null | false {
  const m = /^https?:\/\/(?:www\.)?github\.com\/[\w.-]+\/[\w.-]+?(?:\.git)?(?:\/tree\/(.+?))?\/?$/i.exec(url.trim());
  if (!m) return false;
  if (!m[1]) return null;
  let rest: string;
  try {
    rest = decodeURIComponent(m[1]);
  } catch {
    return false;
  }
  if (path && rest.endsWith(`/${path}`) && rest.length > path.length + 1) return path;
  return rest.includes('/') ? rest.slice(rest.indexOf('/') + 1) : '';
}

function checkRepository(m: Manifest | null, s: Snapshot): CheckResult {
  if (!m) return skip('repository', NO_MANIFEST);
  const path = s.path ?? '';
  const want = `https://github.com/${s.repository}`;
  const fix = path
    ? t(
        `Mets ${code('repository')} à ${code(want)} ou à ${code(`${want}/tree/main/${path}`)}.`,
        `Set ${code('repository')} to ${code(want)} or to ${code(`${want}/tree/main/${path}`)}.`,
      )
    : t(`Mets ${code('repository')} à ${code(want)}.`, `Set ${code('repository')} to ${code(want)}.`);
  if (typeof m.repository !== 'string')
    return fail('repository', t('repository manque', 'repository is missing'), [
      {
        path: 'repository',
        why: t(
          'Le store lie le plugin à son dépôt : le manifeste doit le nommer.',
          'The store ties the plugin to its repository: the manifest must name it.',
        ),
        fix,
      },
    ]);
  if (repoOfUrl(m.repository) !== s.repository.toLowerCase())
    return fail('repository', t(`${m.repository} n’est pas ${s.repository}`, `${m.repository} is not ${s.repository}`), [
      {
        path: 'repository',
        why: t(
          `Le manifeste nomme ${code(m.repository)}, la soumission vient de ${s.repository}.`,
          `The manifest names ${code(m.repository)}, the submission comes from ${s.repository}.`,
        ),
        fix,
      },
    ]);
  const named = folderOfUrl(m.repository, path);
  if (named !== null && named !== path)
    return fail(
      'repository',
      t(`${m.repository} ne nomme pas ${folderOrRoot(path).fr}`, `${m.repository} does not name ${folderOrRoot(path).en}`),
      [
        {
          path: 'repository',
          why: t(
            `Le manifeste nomme ${folderOrRoot(named || '').fr} du dépôt, la soumission vient ${path ? `du dossier ${code(path)}` : `de la racine`}.`,
            `The manifest names ${folderOrRoot(named || '').en} of the repository, the submission comes from ${folderOrRoot(path).en}.`,
          ),
          fix,
        },
      ],
    );
  return pass(
    'repository',
    path
      ? t(
          `repository du manifeste : github.com/${s.repository}, dossier ${path}`,
          `the manifest’s repository: github.com/${s.repository}, folder ${path}`,
        )
      : t(`repository du manifeste : github.com/${s.repository}`, `the manifest’s repository: github.com/${s.repository}`),
  );
}

// ---- 5. The license ----

const LICENSE_FILE = /^(LICEN[CS]E|COPYING)(\.[A-Za-z0-9]+)?$/i;

const licenseFiles = (read: ArchiveRead) => [...read.files.entries()].filter(([p]) => !p.includes('/') && LICENSE_FILE.test(p));

/**
 * The license file the plugin goes by and what is known of it (ESC-313): at the root, the one GitHub detects. In a
 * folder, its own file if it has one, else the root's. GitHub detects the root's alone: the folder's own file is
 * that license if it is the root's file again, or by its `SPDX-License-Identifier:` line; otherwise unrecognized.
 */
export function licenseOf(s: Snapshot): { hasFile: boolean; detected: string | null; inFolder: boolean } {
  const own = licenseFiles(s.archive.read);
  if (!s.path || !s.root) return { hasFile: own.length > 0, detected: s.license, inFolder: false };
  const root = licenseFiles(s.root);
  if (!own.length) return { hasFile: root.length > 0, detected: s.license, inFolder: false };
  const text = own[0]![1].toString('utf8');
  if (root.some(([, b]) => b.toString('utf8').trim() === text.trim())) return { hasFile: true, detected: s.license, inFolder: true };
  const spdx = /SPDX-License-Identifier:\s*([A-Za-z0-9.+-]+)/.exec(text);
  return { hasFile: true, detected: spdx ? spdx[1]! : 'NOASSERTION', inFolder: true };
}

function checkLicense(m: Manifest | null, s: Snapshot): CheckResult {
  if (!m) return skip('license', NO_MANIFEST);
  const { hasFile, detected, inFolder } = licenseOf(s);
  const verdict = judgeLicense(m.license, detected, hasFile);
  if (verdict.ok)
    return pass(
      'license',
      s.path
        ? inFolder
          ? t(`${m.license}, comme le fichier LICENSE du dossier`, `${m.license}, as the folder’s LICENSE file`)
          : t(`${m.license}, comme le fichier LICENSE de la racine`, `${m.license}, as the root’s LICENSE file`)
        : t(`${m.license}, comme le fichier LICENSE`, `${m.license}, as the LICENSE file`),
    );
  const choose = t(
    'Une licence reconnue par l’OSI ou la FSF, en identifiant SPDX : MIT, Apache-2.0, GPL-3.0-or-later, MPL-2.0…',
    'A license approved by the OSI or the FSF, as an SPDX id: MIT, Apache-2.0, GPL-3.0-or-later, MPL-2.0…',
  );
  const then = t(', puis pousse une nouvelle étiquette.', ', then push a new tag.');
  const one = (why: Text, fix: Text, path = 'license') => fail('license', why, [{ path, why, fix }]);
  switch (verdict.code) {
    case 'missing':
      return one(
        t('Le manifeste ne dit pas sa licence.', 'The manifest does not say its license.'),
        t(`Ajoute ${code('license')} : ${choose.fr}${then.fr}`, `Add ${code('license')}: ${choose.en}${then.en}`),
      );
    case 'expression':
      return one(
        t(
          `${code(String(m.license))} n’est pas un identifiant SPDX simple (ni ${code('AND')} ni ${code('WITH')}).`,
          `${code(String(m.license))} is no plain SPDX id (neither ${code('AND')} nor ${code('WITH')}).`,
        ),
        t(`Nomme une licence, ou plusieurs par ${code('OR')}${then.fr}`, `Name one license, or several with ${code('OR')}${then.en}`),
      );
    case 'not-free':
      return one(
        t(
          `${verdict.manifest} n’est pas une licence libre que le store accepte.`,
          `${verdict.manifest} is no free license the store takes.`,
        ),
        t(`${choose.fr}${then.fr}`, `${choose.en}${then.en}`),
      );
    case 'no-file':
      return one(
        s.path
          ? t(
              `Le manifeste dit ${m.license}, mais ni le dossier ${code(s.path)} ni la racine du dépôt n’ont de fichier LICENSE ou COPYING.`,
              `The manifest says ${m.license}, but neither the folder ${code(s.path)} nor the repository’s root has a LICENSE or COPYING file.`,
            )
          : t(
              `Le manifeste dit ${m.license}, mais le dépôt n’a pas de fichier LICENSE ni COPYING à sa racine.`,
              `The manifest says ${m.license}, but the repository has no LICENSE or COPYING file at its root.`,
            ),
        t(`Ajoute le texte de la licence dans LICENSE${then.fr}`, `Add the license’s text in LICENSE${then.en}`),
        'LICENSE',
      );
    case 'undetected':
      return one(
        t(
          `Le manifeste dit ${m.license}, mais GitHub ne reconnaît pas la licence du fichier LICENSE.`,
          `The manifest says ${m.license}, but GitHub does not recognize the license of the LICENSE file.`,
        ),
        inFolder
          ? t(
              `Le LICENSE du dossier doit être celui de la racine, ou porter une ligne ${code(`SPDX-License-Identifier: ${String(m.license)}`)} ; sinon, retire-le : celui de la racine compte${then.fr}`,
              `The folder’s LICENSE must be the root’s, or hold a line ${code(`SPDX-License-Identifier: ${String(m.license)}`)}; otherwise, remove it: the root’s counts${then.en}`,
            )
          : t(
              `Mets dans LICENSE le texte exact de la licence (celui de spdx.org/licenses), sans le modifier${then.fr}`,
              `Put in LICENSE the exact text of the license (spdx.org/licenses’s), unchanged${then.en}`,
            ),
        'LICENSE',
      );
    case 'different':
      return one(
        t(
          `Le manifeste dit ${verdict.manifest}, le fichier LICENSE est sous ${verdict.detected}.`,
          `The manifest says ${verdict.manifest}, the LICENSE file is under ${verdict.detected}.`,
        ),
        t(
          `${code('license')} doit nommer la licence du fichier, en identifiant SPDX : mets les deux d’accord${then.fr}`,
          `${code('license')} must name the file’s license, as an SPDX id: make both agree${then.en}`,
        ),
      );
  }
}

// ---- 6. The README ----

const README = /^README(\.(fr|en))?\.md$/i;

const readmes = (read: ArchiveRead) =>
  [...read.files.entries()].filter(([p, b]) => !p.includes('/') && README.test(p) && b.toString('utf8').trim() !== '');

/** The README files the plugin's page is made of: the folder's, else the repository root's (ESC-313). */
export function readmeSource(read: ArchiveRead, root?: ArchiveRead | null): { read: ArchiveRead; fromRoot: boolean } {
  if (readmes(read).length || !root) return { read, fromRoot: false };
  return readmes(root).length ? { read: root, fromRoot: true } : { read, fromRoot: false };
}

function checkReadme(read: ArchiveRead, s?: Snapshot): CheckResult {
  const found = [...read.files.entries()].filter(([p]) => !p.includes('/') && README.test(p));
  const filled = found.filter(([, b]) => b.toString('utf8').trim() !== '');
  if (filled.length) return pass('readme', t(filled.map(([p]) => p).join(' et '), filled.map(([p]) => p).join(' and ')));
  const fromRoot = s?.path && s.root ? readmes(s.root) : [];
  if (fromRoot.length)
    return pass(
      'readme',
      t(
        `${fromRoot.map(([p]) => p).join(' et ')} de la racine du dépôt (le dossier n’en a pas)`,
        `${fromRoot.map(([p]) => p).join(' and ')} of the repository’s root (the folder has none)`,
      ),
    );
  return fail(
    'readme',
    found.length ? t(`${found[0]![0]} est vide`, `${found[0]![0]} is empty`) : t('README.md manque', 'README.md is missing'),
    [
      {
        path: found[0]?.[0] ?? 'README.md',
        why: t(
          'Le README est la page du plugin, sur le store et dans l’app : il est obligatoire, et non vide.',
          'The README is the plugin’s page, on the store and in the app: it is required, and not empty.',
        ),
        fix: s?.path
          ? t(
              `Ajoute un README.md dans ${code(s.path)} ou à la racine du dépôt (ou README.fr.md et README.en.md), puis pousse une nouvelle étiquette.`,
              `Add a README.md in ${code(s.path)} or at the repository’s root (or README.fr.md and README.en.md), then push a new tag.`,
            )
          : t(
              'Ajoute un README.md à la racine (ou README.fr.md et README.en.md), puis pousse une nouvelle étiquette.',
              'Add a README.md at the root (or README.fr.md and README.en.md), then push a new tag.',
            ),
      },
    ],
  );
}

// ---- 7. The screenshots ----

/** The screenshots the manifest declares (`store.screenshots`), as paths of the archive. */
export function screenshotPaths(m: Manifest | null): string[] {
  const store = m?.store as Record<string, unknown> | undefined;
  const list = store && Array.isArray(store.screenshots) ? store.screenshots : [];
  return list.filter((p): p is string => typeof p === 'string').map((p) => p.replace(/^\.\//, ''));
}

function checkScreenshots(m: Manifest | null, read: ArchiveRead, undecodable: string[] = []): CheckResult {
  if (!m) return skip('screenshots', NO_MANIFEST);
  const paths = screenshotPaths(m);
  if (!paths.length) return pass('screenshots', t('aucune déclarée', 'none declared'));
  const problems: Problem[] = [];
  const at = (i: number) => `store.screenshots[${i}]`;
  const then = t(' Puis pousse une nouvelle étiquette.', ' Then push a new tag.');
  if (paths.length > SCREENSHOTS_MAX)
    problems.push({
      path: 'store.screenshots',
      why: t(
        `${paths.length} captures déclarées, ${SCREENSHOTS_MAX} au plus.`,
        `${paths.length} screenshots declared, ${SCREENSHOTS_MAX} at most.`,
      ),
      fix: t(`Garde les ${SCREENSHOTS_MAX} meilleures.${then.fr}`, `Keep the best ${SCREENSHOTS_MAX}.${then.en}`),
    });
  paths.slice(0, SCREENSHOTS_MAX).forEach((p, i) => {
    const b = read.files.get(p);
    if (!b)
      return problems.push({
        path: at(i),
        why: t(
          `${code(p)} n’est pas dans le dépôt, au commit de l’étiquette.`,
          `${code(p)} is not in the repository, at the tag’s commit.`,
        ),
        fix: t(`Commite l’image, ou corrige son chemin.${then.fr}`, `Commit the image, or fix its path.${then.en}`),
      });
    const info = imageInfo(b);
    if (!info)
      return problems.push({
        path: at(i),
        why: t(`${code(p)} n’est ni un PNG, ni un JPEG, ni un WebP.`, `${code(p)} is no PNG, JPEG or WebP.`),
        fix: t(`Exporte-la en PNG, JPEG ou WebP.${then.fr}`, `Export it as PNG, JPEG or WebP.${then.en}`),
      });
    if (b.length > SCREENSHOT_BYTES)
      problems.push({
        path: at(i),
        why: t(
          `${code(p)} pèse ${(b.length / 1048576).toFixed(1)} Mo, 1 Mo au plus.`,
          `${code(p)} weighs ${(b.length / 1048576).toFixed(1)} MB, 1 MB at most.`,
        ),
        fix: t(`Réduis-la ou compresse-la.${then.fr}`, `Shrink or compress it.${then.en}`),
      });
    if (info.width > SCREENSHOT_WIDTH)
      problems.push({
        path: at(i),
        why: t(
          `${code(p)} fait ${info.width} px de large, ${SCREENSHOT_WIDTH} au plus.`,
          `${code(p)} is ${info.width} px wide, ${SCREENSHOT_WIDTH} at most.`,
        ),
        fix: t(`Redimensionne-la.${then.fr}`, `Resize it.${then.en}`),
      });
  });
  paths.forEach((p, i) => {
    if (undecodable.includes(p))
      problems.push({
        path: at(i),
        why: t(`${code(p)} a l’en-tête d’une image, mais ne se décode pas.`, `${code(p)} has an image’s header, but does not decode.`),
        fix: t(`Exporte-la de nouveau.${then.fr}`, `Export it again.${then.en}`),
      });
  });
  if (problems.length) return fail('screenshots', plural(problems.length, 'problème', 'problem'), problems);
  return pass(
    'screenshots',
    t(`${paths.length} sur ${paths.length}, réencodées par le store`, `${paths.length} of ${paths.length}, re-encoded by the store`),
  );
}

// ---- 8. The archive ----

const LFS_POINTER = 'version https://git-lfs.github.com/spec/v1';

function archiveProblem(p: ArchiveProblem): Problem {
  const fix = t(
    'Retire-le du dépôt (ou fais-en un fichier ordinaire), puis pousse une nouvelle étiquette.',
    'Take it out of the repository (or make it a plain file), then push a new tag.',
  );
  switch (p.code) {
    case 'too-big':
      return {
        why: t(
          `L’archive du commit pèse ${(p.size / 1048576).toFixed(1)} Mo, 50 Mo au plus.`,
          `The commit’s archive weighs ${(p.size / 1048576).toFixed(1)} MB, 50 MB at most.`,
        ),
        fix: t(
          'Sors du dépôt ce qui ne sert pas au plugin (les binaires vont dans la release), puis pousse une nouvelle étiquette.',
          'Move out of the repository what the plugin does not need (binaries go in the release), then push a new tag.',
        ),
      };
    case 'bomb':
      return {
        why: t('L’archive se décompresse en plus de 200 Mo.', 'The archive unpacks to more than 200 MB.'),
        fix: t('Sors du dépôt les gros fichiers, puis pousse une nouvelle étiquette.', 'Move the big files out, then push a new tag.'),
      };
    case 'too-many':
      return {
        why: t('L’archive a plus de 20 000 fichiers.', 'The archive has more than 20,000 files.'),
        fix: t('Sors du dépôt ce qui ne sert pas au plugin.', 'Move out of the repository what the plugin does not need.'),
      };
    case 'corrupt':
      return {
        why: t('L’archive envoyée par GitHub ne se lit pas.', 'The archive GitHub sent does not read.'),
        fix: t('Relance les contrôles ; si cela revient, écris-nous.', 'Run the checks again; if it comes back, write to us.'),
      };
    case 'symlink':
      return { path: p.path, why: t(`${code(p.path)} est un lien symbolique.`, `${code(p.path)} is a symbolic link.`), fix };
    case 'hardlink':
      return { path: p.path, why: t(`${code(p.path)} est un lien physique.`, `${code(p.path)} is a hard link.`), fix };
    case 'special':
      return { path: p.path, why: t(`${code(p.path)} n’est pas un fichier ordinaire.`, `${code(p.path)} is no plain file.`), fix };
    case 'absolute':
      return { path: p.path, why: t(`${code(p.path)} est un chemin absolu.`, `${code(p.path)} is an absolute path.`), fix };
    case 'parent':
      return {
        path: p.path,
        why: t(`${code(p.path)} sort du dossier par ${code('..')}.`, `${code(p.path)} climbs out with ${code('..')}.`),
        fix,
      };
  }
}

function checkArchive(s: Snapshot): CheckResult {
  const { read, size } = s.archive;
  const problems = read.problems.map(archiveProblem);
  if (s.pathLink)
    problems.push({
      path: s.pathLink,
      why: t(
        `Le dossier ${code(s.path ?? '')} passe par ${code(s.pathLink)}, un lien : le store ne suit pas de lien.`,
        `The folder ${code(s.path ?? '')} goes through ${code(s.pathLink)}, a link: the store follows no link.`,
      ),
      fix: t(
        'Soumets le dossier réel du plugin, ou fais du lien un vrai dossier, puis pousse une nouvelle étiquette.',
        'Submit the plugin’s real folder, or make the link a real folder, then push a new tag.',
      ),
    });
  if (read.files.has('.gitmodules'))
    problems.push({
      path: '.gitmodules',
      why: t(
        'Le dépôt a des sous-modules : leur code n’est pas dans l’archive du commit, le plugin serait incomplet.',
        'The repository has submodules: their code is not in the commit’s archive, the plugin would be incomplete.',
      ),
      fix: t(
        'Copie leur contenu dans le dépôt, puis pousse une nouvelle étiquette.',
        'Copy their content into the repository, then push a new tag.',
      ),
    });
  for (const [p, b] of read.files)
    if (b.length < 1024 && b.subarray(0, LFS_POINTER.length).toString('utf8') === LFS_POINTER)
      problems.push({
        path: p,
        why: t(
          `${code(p)} est un pointeur Git LFS : son contenu n’est pas dans l’archive du commit.`,
          `${code(p)} is a Git LFS pointer: its content is not in the commit’s archive.`,
        ),
        fix: t(
          'Commite le fichier lui-même, ou mets-le dans la release, puis pousse une nouvelle étiquette.',
          'Commit the file itself, or put it in the release, then push a new tag.',
        ),
      });
  if (problems.length) return fail('archive', plural(problems.length, 'problème', 'problem'), problems);
  const mb = (size / 1048576).toLocaleString('fr-FR', { maximumFractionDigits: 1 });
  const n = read.files.size;
  return pass(
    'archive',
    t(
      `${mb} Mo, ${n} fichiers, ni lien ni chemin qui sorte`,
      `${(size / 1048576).toFixed(1)} MB, ${n} files, no link nor path that leaves`,
    ),
  );
}

// ---- 9. The permissions ----

const list = (v: unknown): string[] => (Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : []);

function checkPermissions(m: Manifest | null): CheckResult {
  if (!m) return skip('permissions', NO_MANIFEST);
  const p = (m.permissions ?? {}) as Record<string, unknown>;
  const c = (m.contributes ?? {}) as Record<string, unknown>;
  const runs = !!m.run;
  const webhooks = Array.isArray(c.webhooks) && c.webhooks.length > 0;
  const declared = Array.isArray(c.secrets) ? c.secrets.map((x) => (x as Record<string, unknown>)?.key).filter(Boolean) : [];
  const problems: Problem[] = [];
  const unused = (key: string, what: Text) =>
    list(p[key]).forEach((_, i) =>
      problems.push({
        path: `permissions.${key}[${i}]`,
        why: what,
        fix: t(
          `Retire cette permission, ou ajoute le programme (${code('run')}) qui s’en sert ; puis pousse une nouvelle étiquette.`,
          `Remove this permission, or add the program (${code('run')}) that uses it; then push a new tag.`,
        ),
      }),
    );
  if (!runs && !webhooks) {
    unused(
      'network',
      t('Un plugin sans programme ni webhook ne se connecte à rien.', 'A plugin with no program and no webhook connects to nothing.'),
    );
    unused(
      'events',
      t('Un plugin sans programme ni webhook ne reçoit pas d’événement.', 'A plugin with no program and no webhook gets no event.'),
    );
    unused(
      'secrets',
      t('Un plugin sans programme ni webhook ne lit pas de secret.', 'A plugin with no program and no webhook reads no secret.'),
    );
  }
  if (!runs) unused('commands', t('Un plugin sans programme ne lance pas de commande.', 'A plugin with no program runs no command.'));
  list(p.network).forEach((host, i) => {
    if (!/^(\*\.)?[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+(:\d+)?$/.test(host))
      problems.push({
        path: `permissions.network[${i}]`,
        why: t(
          `${code(host)} n’est pas un nom d’hôte (${code('hooks.slack.com')}).`,
          `${code(host)} is no host name (${code('hooks.slack.com')}).`,
        ),
        fix: t('Nomme l’hôte seul, sans https:// ni chemin.', 'Name the host alone, without https:// nor a path.'),
      });
  });
  list(p.secrets).forEach((key, i) => {
    if (!declared.includes(key))
      problems.push({
        path: `permissions.secrets[${i}]`,
        why: t(
          `${code(key)} n’est pas un secret de ${code('contributes.secrets')}.`,
          `${code(key)} is no secret of ${code('contributes.secrets')}.`,
        ),
        fix: t(
          'Déclare le champ du secret dans contributes.secrets, ou retire la permission.',
          'Declare the secret’s field in contributes.secrets, or remove the permission.',
        ),
      });
  });
  if (problems.length) return fail('permissions', plural(problems.length, 'permission incohérente', 'inconsistent permission'), problems);
  const asks: Text[] = [];
  if (runs) asks.push(t('Lance un programme', 'Runs a program'));
  if (p.decide === true) asks.push(t('décide des autorisations', 'decides permissions'));
  if (list(p.network).length) asks.push(t(`se connecte à ${list(p.network).join(', ')}`, `connects to ${list(p.network).join(', ')}`));
  if (list(p.events).length) asks.push(t('reçoit des événements', 'gets events'));
  if (list(p.secrets).length) asks.push(t('garde un secret', 'keeps a secret'));
  if (!asks.length) return pass('permissions', t('ne demande rien', 'asks for nothing'));
  const join = (l: Lang) => asks.map((a, i) => (i ? a[l] : a[l].charAt(0).toUpperCase() + a[l].slice(1))).join(' ; ');
  return pass('permissions', t(join('fr'), join('en').replace(/ ; /g, '; ')));
}

// ---- 10. The binaries ----

function checkBinaries(m: Manifest | null, s: Snapshot): CheckResult {
  if (!m) return skip('binaries', NO_MANIFEST);
  const progs = programs(m, s.archive.read);
  if (!progs.length) return pass('binaries', t('aucun : le plugin ne lance pas de programme', 'none: the plugin runs no program'));
  const problems: Problem[] = [];
  for (const { system, program, inArchive } of progs) {
    if (inArchive) continue;
    if (!s.binaries.some((b) => b.platform.startsWith(`${system}-`)))
      problems.push({
        path: m.run && typeof (m.run as Record<string, unknown>)[system] === 'string' ? `run.${system}` : 'run.command',
        why:
          s.releaseAssets === null
            ? t(
                `${code(program)} n’est pas dans le dépôt, et l’étiquette ${s.tag} n’a pas de release où le prendre pour ${system}.`,
                `${code(program)} is not in the repository, and the tag ${s.tag} has no release to take it from for ${system}.`,
              )
            : t(
                `${code(program)} n’est pas dans le dépôt, et la release ${s.tag} n’a pas de fichier pour ${system} (${code(`…-${system}-x64.zip`)}, ${code(`…-${system}-arm64.tar.gz`)}).`,
                `${code(program)} is not in the repository, and the release ${s.tag} has no file for ${system} (${code(`…-${system}-x64.zip`)}, ${code(`…-${system}-arm64.tar.gz`)}).`,
              ),
        fix: t(
          `Attache à la release l’archive de chaque système (${code('escouade-plugin pack')}), construite par un workflow public avec ${code('actions/attest-build-provenance')}.`,
          `Attach to the release each system’s archive (${code('escouade-plugin pack')}), built by a public workflow with ${code('actions/attest-build-provenance')}.`,
        ),
      });
  }
  for (const name of s.oversized ?? [])
    problems.push({
      path: name,
      why: t(`${code(name)} pèse plus de 50 Mo.`, `${code(name)} weighs more than 50 MB.`),
      fix: t(
        'Allège le binaire (sans symboles de débogage, compressé), puis pousse une nouvelle étiquette.',
        'Make the binary lighter (no debug symbols, compressed), then push a new tag.',
      ),
    });
  for (const b of s.binaries)
    if (b.attestation.state === 'mismatch')
      problems.push({
        path: b.name,
        why: t(
          `L’attestation de ${code(b.name)} le dit construit depuis ${b.attestation.repository ?? '?'}@${(b.attestation.commit ?? '?').slice(0, 7)}, pas depuis ${s.repository}@${s.commit.slice(0, 7)}.`,
          `The attestation of ${code(b.name)} says it was built from ${b.attestation.repository ?? '?'}@${(b.attestation.commit ?? '?').slice(0, 7)}, not from ${s.repository}@${s.commit.slice(0, 7)}.`,
        ),
        fix: t(
          'Reconstruis les binaires depuis le commit de l’étiquette, puis pousse une nouvelle étiquette.',
          'Build the binaries again from the tag’s commit, then push a new tag.',
        ),
      });
  if (problems.length) return fail('binaries', plural(problems.length, 'binaire en défaut', 'binary at fault'), problems);
  if (!s.binaries.length) return pass('binaries', t('le programme est dans le dépôt', 'the program is in the repository'));
  const platforms = s.binaries.map((b) => b.platform).join(', ');
  const attested = s.binaries.filter((b) => b.attestation.state === 'attested');
  if (attested.length === s.binaries.length) {
    const workflows = [...new Set(attested.map((b) => (b.attestation as { workflow: string }).workflow))].join(', ');
    return pass(
      'binaries',
      t(`${platforms} · attestés par le workflow ${workflows}`, `${platforms} · attested by the workflow ${workflows}`),
    );
  }
  return pass(
    'binaries',
    t(
      `${platforms} · sans attestation de provenance pour ${s.binaries.length - attested.length} : publiable non vérifié, ne pourra pas être vérifié`,
      `${platforms} · no provenance attestation for ${s.binaries.length - attested.length}: publishable unverified, cannot be verified`,
    ),
  );
}

/** Whether the archive was read through: a link or a path refused leaves the rest readable, a bomb does not. */
export const whole = (read: ArchiveRead) => !read.problems.some((p) => ['too-big', 'bomb', 'too-many', 'corrupt'].includes(p.code));

/** Runs the ten checks on what the worker fetched. Nothing of the plugin runs: its files are read as bytes. */
export function runChecks(s: Snapshot): CheckResult[] {
  const read = s.archive.read;
  const clean = whole(read);
  const m = clean ? manifestOf(read) : null;
  const unread = t('L’archive ne se lit pas : ce contrôle attend.', 'The archive does not read: this check waits.');
  const results: Record<CheckKey, CheckResult> = {
    manifest: clean ? checkManifest(read, s.iconUndecodable, s.path, !!s.firstParty) : skip('manifest', unread),
    version: checkVersion(m, s),
    id: checkId(m, s),
    repository: checkRepository(m, s),
    license: checkLicense(m, s),
    readme: clean ? checkReadme(read, s) : skip('readme', unread),
    screenshots: checkScreenshots(m, read, s.undecodable),
    archive: checkArchive(s),
    permissions: checkPermissions(m),
    binaries: checkBinaries(m, s),
  };
  return CHECK_KEYS.map((k) => results[k]);
}

/** Where a version stands after its checks: one failed, one holds it for a person, or it may be published. */
export function verdict(results: CheckResult[]): 'rejected' | 'held' | 'ready' {
  if (results.some((r) => r.status === 'fail' || r.status === 'skip')) return 'rejected';
  return results.some((r) => r.status === 'hold') ? 'held' : 'ready';
}

/** The rejection as an author reads it (the page, the e-mail): check by check, what, where, why, how to fix. */
export function rejectionText(results: CheckResult[], lang: Lang): string {
  const lines: string[] = [];
  for (const r of results) {
    if (r.status !== 'fail') continue;
    lines.push(`✗ ${CHECK_NAMES[r.key][lang]} — ${r.detail[lang]}`);
    for (const p of r.problems) {
      lines.push(`  ${p.path ? `${p.path}${lang === 'fr' ? ' :' : ':'} ` : ''}${p.why[lang]}`);
      lines.push(`  → ${p.fix[lang]}`);
    }
  }
  lines.push('', `${lang === 'fr' ? 'La documentation :' : 'The documentation:'} ${DOCS[lang]}`);
  return lines.join('\n');
}

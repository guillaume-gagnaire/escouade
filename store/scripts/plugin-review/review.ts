import { copyFileSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { verifyAttestation, type Runner } from './attest';
import { benchFolder, playBench, type BenchSetup } from './bench';
import { diffFolders, grownPermissions } from './diff';
import { baseVersion, catalog, checkTag, download, Integrity, type Source } from './download';
import { extract, problemText } from './extract';
import { findLeads } from './leads';
import { renderPublic, renderReport, proposed, type ReviewData } from './report';

// The local review of a version (docs/STORE.md §4.2), the deterministic part, without AI: download and check, unpack
// without danger in a new folder outside the repository, prepare the leads, play the bench in a container without
// network, diff with the last verified version, write the report outside the repository. The plugin's folder is passed
// as an argument, never made the current folder of a program; nothing of the plugin runs outside the container.

export interface ReviewOptions {
  id: string;
  version: string;
  since?: string;
  source: Source;
  /** Where the review's folder is made: `<work>/<id>-<version>-<random>/`. */
  work: string;
  /** Where the reports go: `<reports>/<id>/<version>.md`. */
  reports: string;
  bench: BenchSetup;
  /** Why there is no engine, when there is none. */
  benchWhy?: string;
  /** Runs `gh` (the tests give a fake). */
  gh?: Runner;
  /** The repository the script lives in: neither the work nor the reports may be in it. */
  repo: string;
  now?: Date;
}

export interface ReviewDone {
  data: ReviewData;
  report: string;
  summary: string;
  json: string;
  /** 0: written; 1: refused for its integrity. */
  code: 0 | 1;
}

/** Whether `path` is `root` or inside it. */
const within = (root: string, path: string) => {
  const rel = relative(resolve(root), resolve(path));
  return rel === '' || (!rel.startsWith('..') && !/^[A-Za-z]:/.test(rel));
};

export async function review(o: ReviewOptions): Promise<ReviewDone> {
  for (const [what, path] of [
    ['le dossier de travail', o.work],
    ['le dossier des rapports', o.reports],
  ] as const)
    if (within(o.repo, path)) throw new Error(`${what} (${path}) est dans le dépôt : la revue écrit hors de lui`);

  const data: ReviewData = { id: o.id, version: o.version, date: (o.now ?? new Date()).toISOString(), store: o.source.store };
  mkdirSync(o.work, { recursive: true });
  const dir = mkdtempSync(join(o.work, `${o.id}-${o.version}-`));
  data.work = dir;
  try {
    const cat = await catalog(o.source);
    const got = await download(o.source, cat, o.id, o.version);
    const f = got.file;
    Object.assign(data, {
      repository: f.repository,
      tag: f.tag,
      commit: f.commit,
      versionSha256: got.entry.sha256,
      archive: f.archive,
    });
    writeFileSync(join(dir, 'version.json'), got.json);
    data.tagCheck = await checkTag(o.source, f);

    const folder = join(dir, 'plugin');
    const unpacked = extract(got.archive, folder);
    data.folder = folder;
    data.files = unpacked.files.size;
    data.refusedEntries = unpacked.problems.map(problemText);
    if (unpacked.comment && unpacked.comment !== f.commit)
      data.refusedEntries.push(`l’en-tête de l’archive nomme le commit ${unpacked.comment}, version.json ${f.commit}`);
    const manifestBytes = unpacked.files.get('escouade-plugin.json');
    try {
      data.manifest = manifestBytes ? (JSON.parse(manifestBytes.toString('utf8')) as Record<string, unknown>) : null;
    } catch {
      data.manifest = null;
    }
    const network = ((data.manifest?.permissions ?? {}) as { network?: unknown }).network;
    data.leads = findLeads(unpacked.files, Array.isArray(network) ? network.map(String) : []);

    // The binaries, kept to be checked, never run here.
    const repo = f.repository.replace(/^https:\/\/github\.com\//i, '');
    data.binaries = [];
    if (Object.keys(got.binaries).length) mkdirSync(join(dir, 'binaries'));
    for (const [platform, bytes] of Object.entries(got.binaries)) {
      const at = join(dir, 'binaries', platform);
      writeFileSync(at, bytes, { mode: 0o644 });
      data.binaries.push({
        platform,
        sha256: f.binaries[platform as keyof typeof f.binaries]!.sha256,
        attested: verifyAttestation(at, repo, f.commit, o.gh),
      });
    }

    const base = baseVersion(got.plugin, o.version, o.since);
    if (base) {
      const old = await download(o.source, cat, o.id, base.version);
      const was = extract(old.archive, join(dir, 'base'));
      const oldManifest = was.files.get('escouade-plugin.json');
      let before: Record<string, unknown> = {};
      try {
        before = oldManifest ? (JSON.parse(oldManifest.toString('utf8')) as Record<string, unknown>) : {};
      } catch {
        before = {};
      }
      const d = diffFolders(dir, 'base', 'plugin');
      const diffPath = join(dir, 'changes.diff');
      writeFileSync(diffPath, d.text);
      data.base = {
        version: base.version,
        grown: grownPermissions(before, data.manifest ?? {}),
        files: d.files,
        added: d.added,
        removed: d.removed,
        diff: diffPath,
      };
    } else data.base = null;

    const linux = got.binaries['linux-x64'] ?? null;
    const benchDir = data.manifest?.run ? benchFolder(folder, join(dir, 'bench'), data.manifest, linux) : folder;
    data.bench = await playBench(o.bench, benchDir, o.benchWhy);
  } catch (e) {
    if (!(e instanceof Integrity)) throw e;
    data.integrity = e.message;
  }
  return write(o, data);
}

function write(o: ReviewOptions, data: ReviewData): ReviewDone {
  const folder = join(o.reports, data.id);
  mkdirSync(folder, { recursive: true });
  const report = join(folder, `${data.version}.md`);
  // A report already there may hold what a reader wrote: kept beside, never lost.
  if (existsSync(report)) copyFileSync(report, join(folder, `${data.version}.${data.date.replace(/[:.]/g, '-')}.md`));
  writeFileSync(report, renderReport(data));
  const summary = join(folder, `${data.version}.public.md`);
  if (!data.integrity) writeFileSync(summary, renderPublic(data));
  const json = join(folder, `${data.version}.json`);
  writeFileSync(json, JSON.stringify({ ...data, proposed: proposed(data) }, null, 2));
  return { data, report, summary, json, code: data.integrity ? 1 : 0 };
}

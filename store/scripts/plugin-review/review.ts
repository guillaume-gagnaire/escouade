import { copyFileSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { verifyAttestation, type Runner } from './attest';
import { benchFolder, firstPartyBench, playBench, playWasm, type BenchSetup } from './bench';
import { recompile, recompileRust } from './recompile';
import type { CompileOne } from '../../server/utils/compile';
import { diffFolders, grownPermissions } from './diff';
import { baseVersion, catalog, checkTag, download, Integrity, type Source } from './download';
import { extract, problemText } from './extract';
import { findLeads } from './leads';
import { lockChanges, readNativeList, rustLeads } from './rust-leads';
import { normalizePath } from '../../shared/folder';
import { moduleSource, renderPublic, renderPublicEn, renderReport, proposed, type ReviewData } from './report';

// The local review of a version (docs/STORE.md §4.2), the deterministic part, without AI: download and check, unpack
// without danger in a new folder outside the repository, compile a `run.js` bundle or the sources of a `run.wasm` again
// and compare the module with the published one (ESC-324, ESC-337), prepare the leads, play the bench (in a container
// without network; the published module in the app's WASM host, offline), diff with the last verified version, write the
// report outside the repository. The plugin's folder is passed as an argument, never made the current folder of a
// program; nothing of a third party's plugin runs outside a container or the WASM host.

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
  /**
   * Compiles again on this machine, by its `escouade-plugin`: a `run.js` bundle, which runs nothing of the plugin in
   * native, and a `run.wasm` of the first hand. Null without one. Never given a third party's `run.wasm`.
   */
  compile?: CompileOne | null;
  /** Compiles a third party's `run.wasm` again, in a container without network; null without an engine or a Linux tool. */
  container?: CompileOne | null;
  /** Why there is no `container`, when there is none. */
  containerWhy?: string;
  /** Fetches the escouade-js of this fingerprint, when `escouade-plugin` has another one, and says what to do next. */
  engine?: (sha256: string) => string;
  /** The version of Escouade under development: a first-party plugin is played as it, a third party's never (ESC-343). */
  development?: string;
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
    // The plugin's folder in its repository (ESC-313); an old version.json without one is the root's. A folder that
    // is not a plain relative path stops the review, as a fingerprint that does not hold.
    const folder = normalizePath(f.path ?? '');
    if ('problem' in folder || folder.path !== (f.path ?? '')) throw new Integrity(`version.json nomme un dossier mal formé : ${f.path}`);
    Object.assign(data, {
      path: folder.path,
      repository: f.repository,
      tag: f.tag,
      commit: f.commit,
      versionSha256: got.entry.sha256,
      archive: f.archive,
    });
    writeFileSync(join(dir, 'version.json'), got.json);
    data.tagCheck = await checkTag(o.source, f);

    const unpackedAt = join(dir, 'plugin');
    const unpacked = extract(got.archive, unpackedAt);
    data.folder = unpackedAt;
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
    // A plugin with a module: its `run.js` bundle or its `run.wasm` sources compiled again must give the module the store
    // published, byte for byte; an other one stops the review, as an archive changed (ESC-324, ESC-337).
    const firstParty = firstPartyBench(o.id, f.repository);
    const source = moduleSource(data.manifest);
    const rust = source && 'rust' in source ? source.rust : null;
    let module: string | null = null;
    if (source) {
      const js = 'js' in source ? source.js : null;
      const at = js?.replace(/^\.\//, '');
      const bundle = at ? unpacked.files.get(at) : undefined;
      if (js && !bundle) throw new Integrity(`le bundle ${js} que nomme le manifeste n’est pas dans l’archive`);
      if (!f.wasm || !got.wasm)
        data.recompile = {
          state: 'not-done',
          why: `version.json ne nomme aucun module : le store n’a pas compilé ${js ? 'ce bundle' : 'ces sources'}`,
        };
      else {
        data.wasm = f.wasm;
        module = join(dir, 'plugin.wasm');
        writeFileSync(module, got.wasm);
        const manifest = manifestBytes!;
        if (at) data.recompile = await recompile(o.compile ?? null, { manifest, js: at, bundle: bundle!, firstParty }, f.wasm, o.engine);
        else if (firstParty)
          data.recompile = await recompileRust(
            o.compile ?? null,
            { manifest, wasm: rust!, files: unpacked.files, firstParty },
            f.wasm,
            'pas d’`escouade-plugin` sur ce poste pour recompiler (ESCOUADE_PLUGIN, le PATH ou src-tauri/target)',
            'local',
          );
        // A third party's scripts and macros run in native: a container or nothing, never this machine.
        else
          data.recompile = await recompileRust(
            o.container ?? null,
            { manifest, wasm: rust!, files: unpacked.files },
            f.wasm,
            `pas de recompilation d’un tiers sans conteneur : ${o.containerWhy ?? 'aucun moteur de conteneur'}`,
            'container',
          );
        if (data.recompile.state === 'different')
          throw new Integrity(
            `le module publié (${data.recompile.published}) n’est pas la recompilation ${js ? `du bundle ${js}` : 'des sources de l’archive'} (${data.recompile.local}), avec les mêmes versions du compilateur`,
          );
      }
    }

    const network = ((data.manifest?.permissions ?? {}) as { network?: unknown }).network;
    data.leads = findLeads(unpacked.files, Array.isArray(network) ? network.map(String) : []);
    if (rust)
      data.leads.push(
        ...rustLeads(unpacked.files, got.wasm, (data.manifest?.permissions ?? {}) as Record<string, unknown>, readNativeList()),
      );

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
      if (rust) {
        const lock = (files: Map<string, Buffer>) => files.get('Cargo.lock')?.toString('utf8') ?? '';
        data.leads.push(...lockChanges(lock(was.files), lock(unpacked.files)));
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

    if (source)
      // The module published, in the WASM host, never in a container; not played when the review has none to play.
      data.bench = module
        ? await playWasm(o.bench, unpackedAt, module, firstParty, o.development)
        : { played: false, why: 'aucun module publié à jouer' };
    else {
      const linux = got.binaries['linux-x64'] ?? null;
      const benchDir = data.manifest?.run ? benchFolder(unpackedAt, join(dir, 'bench'), data.manifest, linux) : unpackedAt;
      data.bench = await playBench(o.bench, benchDir, o.benchWhy, firstParty, o.development);
    }
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
  if (!data.integrity) {
    writeFileSync(summary, renderPublic(data));
    writeFileSync(join(folder, `${data.version}.public.en.md`), renderPublicEn(data));
  }
  const json = join(folder, `${data.version}.json`);
  writeFileSync(json, JSON.stringify({ ...data, proposed: proposed(data) }, null, 2));
  return { data, report, summary, json, code: data.integrity ? 1 : 0 };
}

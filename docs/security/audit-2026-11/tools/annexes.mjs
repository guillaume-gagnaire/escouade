// Makes the annexes of the audit's report from the raw outputs of its tools and tests (ESC-418):
//
//   node docs/security/audit-2026-11/tools/annexes.mjs <folder of the raw outputs>
//
// The folder holds what the commands of the report's annex F wrote: cargo-audit-app.txt, cargo-audit-plugins.txt,
// cargo-deny-app.txt, npm-audit-app.txt, npm-audit-store-prod.txt, npm-audit-website-prod.txt, cargo-geiger.txt,
// semgrep.json, whitebox-audit.txt (the audit's own tests), whitebox-existing.txt (the project's tests run again),
// vitest-plugins.txt (the project's tests of its view of the plugins), plugin-dev-host.txt (the malicious plugin run
// alone in the plugins' host), update-signatures.txt, macos-package.txt, published.txt and provenance.txt (the published
// release checked by its signatures), and blackbox/ (the JSON notes of the black-box tests). Each is copied as it is
// but for one thing: the home folder of who ran the tools is written `~`, their name `<utilisateur>` and the folder of
// the repository `<dépôt>` (the report is public). The summaries (cargo deny, cargo geiger, semgrep, the tests' lines)
// and the list of the direct dependencies are made here.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const RAW = process.argv[2];
if (!RAW) throw new Error('give the folder of the raw outputs');
const ROOT = path.resolve(HERE, '..', '..', '..', '..');
const OUT = path.resolve(HERE, '..', 'annexes');

/** `text` without the home folder nor the name of who ran the tools, nor where the repository is on their machine. */
function anonymous(text) {
  const home = os.homedir();
  const doubled = home.split('\\').join('\\\\');
  let out = text;
  for (const spelling of [ROOT.split('\\').join('\\\\'), ROOT, ROOT.split('\\').join('/')]) out = out.split(spelling).join('<dépôt>');
  for (const spelling of [
    doubled.split('\\').join('\\\\'),
    doubled,
    home,
    home.split('\\').join('/'),
    `/c${home.slice(2).split('\\').join('/')}`,
    `/mnt/c${home.slice(2).split('\\').join('/')}`,
  ]) {
    out = out.split(spelling).join('~');
  }
  return out.split(os.userInfo().username).join('<utilisateur>');
}

const read = (name) => (fs.existsSync(path.join(RAW, name)) ? fs.readFileSync(path.join(RAW, name), 'utf8') : null);
/** What the report cannot do without. */
function needed(name) {
  const text = read(name);
  if (text === null) throw new Error(`${name} is missing from ${RAW}`);
  return text;
}
function write(name, text) {
  const file = path.join(OUT, name);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${anonymous(text).replace(/\r\n/g, '\n').trimEnd()}\n`);
  console.log(`annexes/${name}`);
}

// ---- The raw outputs, as they are ----
for (const name of [
  'cargo-audit-app.txt',
  'cargo-audit-plugins.txt',
  'npm-audit-app.txt',
  'npm-audit-store-prod.txt',
  'npm-audit-website-prod.txt',
]) {
  write(name, needed(name));
}

// ---- cargo deny: what it found, by kind, and its verdict ----
{
  const text = needed('cargo-deny-app.txt');
  const kinds = new Map();
  const titles = [];
  for (const line of text.split('\n')) {
    const m = /^(error|warning)\[([\w-]+)\]: (.*)$/.exec(line);
    if (!m) continue;
    kinds.set(`${m[1]}[${m[2]}]`, (kinds.get(`${m[1]}[${m[2]}]`) ?? 0) + 1);
    if (m[1] === 'error') titles.push(`${m[2]}: ${m[3]}`);
  }
  const verdict =
    text
      .split('\n')
      .reverse()
      .find((l) => /advisories .*bans .*licenses .*sources/.test(l)) ?? '';
  write(
    'cargo-deny-resume.txt',
    [
      'cargo deny 0.20.2 --manifest-path src-tauri/Cargo.toml --config deny.toml check   (deny.toml : celui du dépôt)',
      '',
      verdict.trim(),
      '',
      ...[...kinds].sort((a, b) => b[1] - a[1]).map(([k, n]) => `${String(n).padStart(4)}  ${k}`),
      '',
      'Erreurs :',
      ...[...new Set(titles)].sort().map((t) => `  ${t}`),
    ].join('\n'),
  );
}

// ---- cargo geiger: how many crates hold `unsafe`, and the lines of the crates of the repository ----
{
  const text = needed('cargo-geiger.txt');
  const seen = new Map();
  const own = new Map();
  for (const line of text.split('\n')) {
    const m = /^(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(!|\?|:\))\s+[|`\- ]*([A-Za-z0-9_-]+) (\d[^\s]*)/.exec(line);
    if (!m) continue;
    seen.set(`${m[7]} ${m[8]}`, m[6]);
    if (m[7].startsWith('escouade'))
      own.set(
        `${m[7]} ${m[8]}`,
        `${m[1].padEnd(10)} ${m[2].padEnd(12)} ${m[3].padEnd(6)} ${m[4].padEnd(7)} ${m[5].padEnd(8)} ${m[6].padEnd(2)} ${m[7]} ${m[8]}`,
      );
  }
  const count = (s) => [...seen.values()].filter((v) => v === s).length;
  write(
    'cargo-geiger-resume.txt',
    [
      `${text.split('\n')[0].trim()}, cargo geiger --output-format Ascii, dans src-tauri`,
      '',
      `${seen.size} crates dans le graphe : ${count('!')} avec du code unsafe (!), ${count('?')} sans (?), ${count(':)')} qui l'interdisent (:))`,
      '',
      'Les crates du dépôt (x/y : unsafe utilisé par la construction / trouvé dans la crate) :',
      'Functions  Expressions  Impls  Traits  Methods',
      ...[...own].sort((a, b) => a[0].localeCompare(b[0])).map(([, line]) => line),
    ].join('\n'),
  );
}

// ---- The tests: their lines that tell a result ----
{
  // A test's own lines start with AUDIT; what it lists under one of them is indented by two spaces.
  const kept = needed('whitebox-audit.txt')
    .split('\n')
    .filter((l) => /^\s*(PASS|FAIL)\b|^AUDIT|^ {2}\S|Summary \[/.test(l));
  write('essais/boite-blanche.txt', kept.map((l) => l.trimEnd()).join('\n'));
  const existing = read('whitebox-existing.txt');
  if (existing !== null) {
    const lines = existing.split('\n').filter((l) => /^\s*(PASS|FAIL|FLAKY)\b|Summary \[/.test(l));
    write('essais/tests-existants.txt', lines.map((l) => l.trimEnd().replace(/\s+\[\s*[\d.]+s\]/, '')).join('\n'));
  }
  // The project's own tests of its view of the plugins (vitest): one line per test, its colours and its time taken
  // off, and how many passed.
  const view = read('vitest-plugins.txt');
  if (view !== null) {
    const lines = view
      .replace(/\u001b\[[0-9;]*m/g, '')
      .split('\n')
      .filter((l) => /^\s*[✓×↓]\s|^\s*(Test Files|Tests)\s/.test(l))
      .map((l) => l.trimEnd().replace(/\s+\d+\s?ms$/, ''));
    write('essais/tests-vue-plugins.txt', lines.join('\n'));
  }
  // The malicious plugin run alone in the plugins' host (`escouade-plugin dev`): one line per try.
  const alone = read('plugin-dev-host.txt');
  if (alone !== null) {
    const tries = alone
      .split('\n')
      .filter((l) => l.includes('AUDIT '))
      .map((l) => l.slice(l.indexOf('AUDIT ')).trimEnd());
    write('essais/plugin-seul.txt', tries.join('\n'));
  }
  // The files of the published release checked by their signatures (the report's annex F): as they were written.
  for (const [from, to] of [
    ['update-signatures.txt', 'essais/signatures-mise-a-jour.txt'],
    ['macos-package.txt', 'essais/paquet-macos.txt'],
    ['published.txt', 'essais/produit-publie.txt'],
    ['provenance.txt', 'essais/attestation-de-provenance.txt'],
  ]) {
    const text = read(from);
    if (text !== null) write(to, text);
  }
  const dir = path.join(RAW, 'blackbox');
  if (fs.existsSync(dir)) {
    for (const f of fs
      .readdirSync(dir)
      .filter((f) => f.endsWith('.json'))
      .sort())
      write(`essais/${f}`, fs.readFileSync(path.join(dir, f), 'utf8'));
  }
  const run = read('blackbox.txt');
  if (run !== null)
    write(
      'essais/boite-noire.txt',
      run
        .split('\n')
        .filter((l) => /^\s*(ok|x|-)\s+\d+|passed|failed/.test(l))
        .join('\n'),
    );
}

// ---- semgrep: how many findings, by rule ----
{
  const report = JSON.parse(needed('semgrep.json'));
  const rules = new Map();
  for (const r of report.results ?? []) {
    const key = `${(r.extra?.severity ?? '?').padEnd(8)} ${r.check_id}`;
    const at = rules.get(key) ?? [];
    at.push(`${r.path}:${r.start?.line}`);
    rules.set(key, at);
  }
  const lines = [
    `semgrep ${report.version ?? ''}, règles publiques p/rust, p/typescript, p/javascript, p/security-audit, p/github-actions, p/secrets`,
    `Fichiers lus : ${report.paths?.scanned?.length ?? '?'} ; résultats : ${report.results?.length ?? 0} ; erreurs de lecture : ${report.errors?.length ?? 0}`,
    '',
    ...[...rules]
      .sort((a, b) => b[1].length - a[1].length)
      .flatMap(([k, at]) => [
        `${String(at.length).padStart(4)}  ${k}`,
        ...at.slice(0, 6).map((p) => `        ${p}`),
        ...(at.length > 6 ? [`        … et ${at.length - 6} autres`] : []),
      ]),
  ];
  write('semgrep-resume.txt', lines.join('\n'));
}

// ---- The direct dependencies, with the version the lock files hold ----
{
  const lock = fs.readFileSync(path.join(ROOT, 'src-tauri', 'Cargo.lock'), 'utf8');
  const locked = new Map();
  for (const m of lock.matchAll(/name = "([^"]+)"\nversion = "([^"]+)"/g)) locked.set(m[1], [...(locked.get(m[1]) ?? []), m[2]]);
  const manifest = fs.readFileSync(path.join(ROOT, 'src-tauri', 'Cargo.toml'), 'utf8');
  const rust = new Set();
  let inDeps = false;
  for (const line of manifest.split('\n')) {
    if (/^\[/.test(line)) inDeps = /^\[(target\..*\.)?dependencies\]/.test(line.trim());
    const m = /^([A-Za-z0-9_-]+)\s*=/.exec(line);
    if (inDeps && m) rust.add(m[1]);
  }
  const pkg = JSON.parse(fs.readFileSync(path.join(ROOT, 'package.json'), 'utf8'));
  const npmLock = JSON.parse(fs.readFileSync(path.join(ROOT, 'package-lock.json'), 'utf8'));
  const npm = Object.keys(pkg.dependencies ?? {}).sort();
  const lines = [
    `Rust, application (src-tauri/Cargo.toml, ${rust.size} dépendances directes ; versions de Cargo.lock) :`,
    ...[...rust].sort().map((n) => `  ${n} ${(locked.get(n) ?? ['(chemin local)']).join(', ')}`),
    '',
    `npm, interface (package.json, ${npm.length} dépendances livrées ; versions de package-lock.json) :`,
    ...npm.map((n) => `  ${n} ${npmLock.packages?.[`node_modules/${n}`]?.version ?? '?'}`),
  ];
  write('dependances-directes.txt', lines.join('\n'));
}

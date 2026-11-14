// Builds the report of the security audit of 2026-11 (ESC-418): rapport.md → HTML → escouade-audit-securite-2026-11.pdf,
// by the Chromium of Playwright (already a dependency of the repository, as `marked` and the two fonts are).
//
//   node docs/security/audit-2026-11/build.mjs            the PDF (and build/rapport.html, kept out of git, to look at)
//   node docs/security/audit-2026-11/build.mjs --check    only reads the sources for what must not leave: see `leaks`
//
// The report is public: `--check` runs before every build too, and a build that finds a leak writes nothing.
//
// The layout: a title page, a table of contents with page numbers, then one chapter per `# title` (each on a new page)
// and, in the chapter of the findings, one page or more per `## SEC-nn` (a « fiche »). Page numbers come from CSS
// (`@page` margin boxes). The table's own numbers are computed: each chapter and each fiche starts a page, so its length
// is that of its own PDF, rendered alone first.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { Marked } from 'marked';
import { chromium } from '@playwright/test';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..', '..');
const SOURCE = path.join(HERE, 'rapport.md');
const PDF = path.join(HERE, 'escouade-audit-securite-2026-11.pdf');
const OUT = path.join(HERE, 'build');

// ---- What must not leave (acceptance criterion 6) -----------------------------------------------------------------

/** The hosts the report may name though they hold the publisher's name: the network inventory needs them as they are. */
const NAMED = ['aptabase.gagnaire.dev', 'dev.gagnaire.escouade', 'github.com/guillaume-gagnaire/escouade'];

/** What the machine knows of its user, never written here: looked for in what the report holds. */
function personal() {
  const words = new Set();
  const add = (w) => w && w.trim().length >= 4 && words.add(w.trim().toLowerCase());
  add(os.userInfo().username);
  add(path.basename(os.homedir()));
  for (const key of ['user.email', 'user.name']) {
    try {
      const value = execFileSync('git', ['config', key], { cwd: ROOT, encoding: 'utf8' }).trim();
      add(value);
      for (const part of value.split(/[\s@.]+/)) add(part);
    } catch {
      // Not set: nothing to look for.
    }
  }
  return [...words];
}

/**
 * A word that would tell for whom, or for what occasion, an audit was made: this report tells the state of a version on
 * a date, to any reader. Refused as a word of the text (« mission », « client », « patte blanche », in French or in
 * English), not inside another word (« permission », « transmission ») nor an identifier (`fence_by_client`).
 */
const OCCASION = /(?<![\p{L}\p{N}_-])(?:missions?|clients?|clientes?|patte blanche)(?![\p{L}\p{N}_-])/giu;
/** The same word, said of a program that calls a server: what a tool's raw output may say (`HTTP client`). */
const OF_A_PROGRAM =
  /\b(?:http|https|quic|mcp|acp|tls|git|oauth|api|rpc|ssh|dns|smtp)[ -]clients?\b|\bclients? (?:http|https|mcp|acp|tls|git|oauth)\b|\bclient[- ](?:side|id|secret|hello|certificate|library|libraries)\b/giu;

/** The lines of `text` that hold something the report must not: `[line number, why, excerpt]`. */
export function leaks(text, name) {
  const found = [];
  const mine = personal();
  const rules = [
    [
      'an e-mail address',
      /[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+\.[A-Za-z]{2,}/g,
      (m) => !/@(v?\d|[a-z-]+\/)/.test(m) && !/(example|exemple)\.(com|org|dev|invalid)$/.test(m) && !m.startsWith('git@'),
    ],
    [
      'a home folder',
      /(?:[A-Za-z]:\\+Users\\+|\/Users\/|\/home\/)(?!<)[A-Za-z0-9._-]+/g,
      (m) => !/(runner|runneradmin|you|toi|user|utilisateur)$/i.test(m),
    ],
    [
      'a webhook',
      /https:\/\/(?:hooks\.slack\.com\/services|(?:ptb\.|canary\.)?discord(?:app)?\.com\/api\/webhooks)\/[^\s"')]+/g,
      () => true,
    ],
    [
      'a token',
      /\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|xox[abprs]-[A-Za-z0-9-]{10,}|glpat-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16})\b/g,
      () => true,
    ],
    ['a bearer token', /Bearer\s+[A-Za-z0-9_\-.=]{24,}/g, () => true],
    // Where the audit was run: a checkout at the root of a drive, a worktree of the repository.
    ['a folder of the machine that ran the audit', /[A-Za-z]:[\\/]+escouade-[\w.-]+|worktrees[\\/]+esc-\d+/gi, () => true],
    ['a private key', /-----BEGIN [A-Z ]*PRIVATE KEY-----/g, () => true],
  ];
  text.split('\n').forEach((line, i) => {
    let scrubbed = line;
    for (const ok of NAMED) scrubbed = scrubbed.split(ok).join('');
    for (const [why, re, counts] of rules) {
      for (const m of scrubbed.match(re) ?? []) if (counts(m)) found.push([`${name}:${i + 1}`, why, m.slice(0, 60)]);
    }
    const lower = scrubbed.toLowerCase();
    for (const word of mine) if (lower.includes(word)) found.push([`${name}:${i + 1}`, 'the name of the machine’s user', word]);
    for (const m of scrubbed.replace(OF_A_PROGRAM, '').match(OCCASION) ?? [])
      found.push([`${name}:${i + 1}`, 'a word of the occasion of an audit (a mission, a client)', m]);
  });
  return found;
}

/** The report and every file it quotes, read for leaks: the list of what was found. */
function check() {
  // The report, what it quotes, and everything else kept beside it: the notes of the tests go with the report.
  const under = (dir) =>
    fs.existsSync(dir)
      ? fs
          .readdirSync(dir, { withFileTypes: true, recursive: true })
          .filter((e) => e.isFile())
          .map((e) => path.join(e.parentPath, e.name))
      : [];
  const quoted = includesOf(fs.readFileSync(SOURCE, 'utf8')).map((f) => path.join(HERE, f));
  const files = [...new Set([SOURCE, path.join(HERE, 'README.md'), ...quoted, ...under(path.join(HERE, 'annexes'))])];
  const found = files.flatMap((f) => leaks(fs.readFileSync(f, 'utf8'), path.relative(HERE, f)));
  for (const [where, why, what] of found) console.error(`${where}: ${why} (${what})`);
  console.log(
    found.length
      ? `${found.length} leak(s) in ${files.length} files`
      : `no leak in ${files.length} files (the report, its README and its annexes)`,
  );
  return found.length === 0;
}

// ---- Markdown to HTML -------------------------------------------------------------------------------------------

const escapeHtml = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** `<!-- annexe: file -->` on a line of its own: the file, as it is. */
const INCLUDE = /^<!-- annexe: ([\w./-]+) -->$/gm;
const includesOf = (md) => [...md.matchAll(INCLUDE)].map((m) => m[1]);

const slug = (text) =>
  text
    .toLowerCase()
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/<[^>]+>/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');

function toHtml(md) {
  const withFiles = md.replace(
    INCLUDE,
    (_, file) => `<pre class="raw">${escapeHtml(fs.readFileSync(path.join(HERE, file), 'utf8').trimEnd())}</pre>`,
  );
  const marked = new Marked({
    gfm: true,
    renderer: {
      heading({ tokens, depth }) {
        const html = this.parser.parseInline(tokens);
        return `<h${depth} id="${slug(html)}">${html}</h${depth}>\n`;
      },
      table(token) {
        const cells = (row, tag) => `<tr>${row.map((c) => `<${tag}>${this.parser.parseInline(c.tokens)}</${tag}>`).join('')}</tr>`;
        // A table of six columns or more is set smaller.
        const wide = token.header.length >= 6 ? ' class="wide"' : '';
        return `<table${wide}><thead>${cells(token.header, 'th')}</thead><tbody>${token.rows.map((r) => cells(r, 'td')).join('')}</tbody></table>\n`;
      },
    },
  });
  // A path or a name in the text may break after a `/` and before a `:`, nowhere else: read whole, never cut mid-word.
  return marked
    .parse(withFiles, { async: false })
    .replace(
      /(?<!<pre>)<code>([^<]*)<\/code>/g,
      (_, text) => `<code>${text.replace(/\//g, '/<wbr>').replace(/(?<=[\w)]):(?=[\w:])/g, '<wbr>:')}</code>`,
    );
}

/** The severities, as the fiches and the tables colour them. */
const SEVERITIES = ['Critique', 'Élevée', 'Moyenne', 'Faible', 'Info'];
const SEVERITY_ROW = new RegExp(`<td>Gravité</td><td>(?:<strong>)?(${SEVERITIES.join('|')})`);
const STATES = ['Démontré', 'Probable', 'Théorique', 'Observé'];
/** What the publisher says of a finding it knows and leaves as it is (the row « Position de l'éditeur »), and the class of its label. */
const STATUSES = { Accepté: 'kept' };
const STATUS_ROW = new RegExp(`<td>Position de l(?:'|&#39;)éditeur</td><td>(?:<strong>)?(${Object.keys(STATUSES).join('|')})`);

/**
 * The HTML cut where a page starts: a chapter at each `<h1>`, and in the chapter of the findings a fiche at each `<h2>`.
 * Each block: its title, its level (1 or 2), its id and its HTML.
 */
function blocksOf(html) {
  const blocks = [];
  const chapters = html.split(/(?=<h1 )/).filter((c) => c.trim());
  for (const chapter of chapters) {
    const fiches = /<h2 id="sec-\d+/.test(chapter) ? chapter.split(/(?=<h2 id="sec-\d+)/) : [chapter];
    fiches.forEach((part, i) => {
      const head = /<h([12]) id="([^"]+)">(.*?)<\/h\1>/s.exec(part);
      if (!head) return;
      const level = i === 0 ? 1 : 2;
      // A fiche's severity is the first word of its « Gravité » row.
      const severity = level === 2 ? (SEVERITY_ROW.exec(part)?.[1] ?? null) : null;
      // And what the publisher says of it, when it has a row « Position de l'éditeur ».
      const status = level === 2 ? (STATUS_ROW.exec(part)?.[1] ?? null) : null;
      // The title is HTML already (its entities escaped): kept so, its tags taken off.
      blocks.push({ level, id: head[2], title: head[3].replace(/<[^>]+>/g, ''), severity, status, html: part });
    });
  }
  return blocks;
}

// ---- The page ----------------------------------------------------------------------------------------------------

// The fonts go in the page itself: it is rendered from its text (no origin), which may not read a file.
const font = (pkg, file) =>
  `data:font/woff2;base64,${fs.readFileSync(path.join(ROOT, 'node_modules', pkg, 'files', file)).toString('base64')}`;

const CSS = `
@font-face { font-family: Inter; font-weight: 100 900; src: url("${font('@fontsource-variable/inter', 'inter-latin-wght-normal.woff2')}") format("woff2"); unicode-range: U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD; }
@font-face { font-family: Inter; font-weight: 100 900; src: url("${font('@fontsource-variable/inter', 'inter-latin-ext-wght-normal.woff2')}") format("woff2"); unicode-range: U+0100-02AF, U+1E00-1EFF, U+2020, U+20A0-20AB, U+20AD-20CF, U+2113, U+2C60-2C7F, U+A720-A7FF; }
@font-face { font-family: "JetBrains Mono"; font-weight: 400; src: url("${font('@fontsource/jetbrains-mono', 'jetbrains-mono-latin-400-normal.woff2')}") format("woff2"); }
@font-face { font-family: "JetBrains Mono"; font-weight: 700; src: url("${font('@fontsource/jetbrains-mono', 'jetbrains-mono-latin-700-normal.woff2')}") format("woff2"); }

@page {
  size: A4;
  margin: 20mm 17mm 20mm 17mm;
  @bottom-left { content: "Escouade — audit de sécurité, novembre 2026"; font: 7.5pt Inter, sans-serif; color: #6b7280; }
  @bottom-right { content: counter(page) " / " counter(pages); font: 7.5pt Inter, sans-serif; color: #6b7280; }
}
@page :first { @bottom-left { content: none; } @bottom-right { content: none; } }

* { box-sizing: border-box; }
html { font: 9.4pt/1.5 Inter, "Segoe UI", Arial, sans-serif; color: #111827; -webkit-print-color-adjust: exact; print-color-adjust: exact; }
body { margin: 0; }
h1 { font-size: 19pt; line-height: 1.2; margin: 0 0 5mm; padding-bottom: 2.5mm; border-bottom: 1.5pt solid #111827; letter-spacing: -0.01em; }
h2 { font-size: 12.5pt; line-height: 1.25; margin: 7mm 0 2.5mm; break-after: avoid; }
h3 { font-size: 10pt; margin: 4.5mm 0 1.5mm; break-after: avoid; text-transform: uppercase; letter-spacing: 0.05em; color: #374151; }
h4 { font-size: 9.4pt; margin: 3.5mm 0 1mm; break-after: avoid; }
p { margin: 0 0 2.4mm; orphans: 3; widows: 3; }
p:has(+ pre), p:has(+ table), p:has(+ ul), p:has(+ ol) { break-after: avoid; }
ul, ol { margin: 0 0 2.4mm; padding-left: 5mm; }
li { margin-bottom: 0.9mm; }
a { color: inherit; text-decoration: none; }
strong { font-weight: 650; }
code { font: 8pt "JetBrains Mono", Consolas, monospace; background: #f3f4f6; padding: 0.2mm 0.8mm; border-radius: 0.6mm; overflow-wrap: break-word; }
pre { font: 7.4pt/1.45 "JetBrains Mono", Consolas, monospace; background: #f6f7f9; border: 0.4pt solid #d1d5db; border-radius: 1mm; padding: 2mm 2.5mm; margin: 0 0 2.6mm; white-space: pre-wrap; overflow-wrap: anywhere; break-inside: avoid; }
pre code { background: none; padding: 0; font: inherit; }
pre.raw { font-size: 6.3pt; line-height: 1.35; break-inside: auto; }
blockquote { margin: 0 0 2.6mm; padding: 1.6mm 3mm; border-left: 2pt solid #9ca3af; background: #f9fafb; color: #374151; }
blockquote p:last-child { margin-bottom: 0; }
table { width: 99.7%; border-collapse: collapse; margin: 0 0 3mm; font-size: 8.3pt; line-height: 1.38; }
th { text-align: left; font-weight: 650; background: #eef0f3; border: 0.4pt solid #c5cad3; padding: 1.2mm 1.6mm; vertical-align: bottom; }
td { border: 0.4pt solid #d1d5db; padding: 1.2mm 1.6mm; vertical-align: top; overflow-wrap: break-word; }
table.wide { font-size: 7.4pt; line-height: 1.34; }
table.wide th, table.wide td { padding: 1mm 1.2mm; }
table.wide code { font-size: 6.6pt; padding: 0 0.4mm; }
tr { break-inside: avoid; }
thead { display: table-header-group; }
hr { border: 0; border-top: 0.4pt solid #d1d5db; margin: 4mm 0; }
figure { margin: 0 0 3mm; break-inside: avoid; }
figcaption { font-size: 8pt; color: #4b5563; margin-top: 1mm; }
svg text { font-family: Inter, "Segoe UI", Arial, sans-serif; }

.block { break-before: page; }
.sev { display: inline-block; font-size: 7.6pt; font-weight: 700; padding: 0.3mm 1.8mm; border-radius: 1mm; color: #fff; letter-spacing: 0.02em; white-space: nowrap; }
.sev-Critique { background: #7f1d1d; } .sev-Élevée { background: #b91c1c; } .sev-Moyenne { background: #b45309; }
.sev-Faible { background: #1d4ed8; } .sev-Info { background: #4b5563; }
.st { display: inline-block; font-size: 7.6pt; font-weight: 700; padding: 0.3mm 1.8mm; border-radius: 1mm; white-space: nowrap; border: 0.5pt solid; }
.st-kept { color: #1e3a8a; border-color: #1d4ed8; background: #eff6ff; }
.etat { display: inline-block; font-size: 7.6pt; font-weight: 650; padding: 0.3mm 1.8mm; border-radius: 1mm; border: 0.5pt solid #6b7280; color: #374151; white-space: nowrap; }
.fiche h2 { font-size: 13.5pt; margin-top: 0; padding-bottom: 2mm; border-bottom: 0.8pt solid #9ca3af; }
.fiche table:first-of-type td:first-child, .fiche table:first-of-type th:first-child { width: 31mm; font-weight: 650; background: #f6f7f9; }
.fiche table:first-of-type thead { display: none; }

.cover { height: 257mm; display: flex; flex-direction: column; justify-content: space-between; padding: 14mm 4mm 0; }
.cover .brand { font-size: 12pt; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase; color: #374151; }
.cover h1 { font-size: 33pt; border: 0; margin: 0 0 5mm; padding: 0; line-height: 1.08; }
.cover .sub { font-size: 14pt; color: #374151; margin-bottom: 12mm; }
.cover table { width: auto; font-size: 9.6pt; }
.cover td { border: 0; padding: 0.9mm 6mm 0.9mm 0; }
.cover td:first-child { color: #6b7280; }
.cover .note { font-size: 8.4pt; color: #4b5563; border-top: 0.4pt solid #9ca3af; padding-top: 3mm; }

.toc h1 { margin-bottom: 2.5mm; }
.toc ol { list-style: none; padding: 0; margin: 0; }
.toc li { display: flex; align-items: baseline; gap: 2mm; margin: 0; padding: 0.2mm 0; }
.toc li.l1 { font-weight: 650; margin-top: 0.8mm; }
.toc li.l2 { padding-left: 5mm; font-size: 8.5pt; }
.toc .dots { flex: 1; border-bottom: 0.4pt dotted #9ca3af; transform: translateY(-1mm); }
.toc .page { font-variant-numeric: tabular-nums; min-width: 7mm; text-align: right; }
.toc .sev, .toc .st { transform: translateY(-0.2mm); }
.toc .st { font-size: 6.8pt; padding: 0.1mm 1.2mm; }
`;

const page = (body) =>
  `<!doctype html><html lang="fr"><head><meta charset="utf-8"><title>Escouade — audit de sécurité, novembre 2026</title><style>${CSS}</style></head><body>${body}</body></html>`;

/**
 * The severities, the states and the publisher's position shown as labels: in a cell that holds one alone (the tables
 * that list the findings), and at the start of the « Gravité », « État » and « Position de l'éditeur » rows of a fiche.
 * Never elsewhere: « Faible » is also an effort.
 */
function labelled(html) {
  const sev = (s) => `<span class="sev sev-${s}">${s}</span>`;
  const state = (s) => `<span class="etat">${s}</span>`;
  let out = html;
  for (const s of SEVERITIES) {
    out = out.split(`<td>${s}</td>`).join(`<td>${sev(s)}</td>`);
    out = out.replace(new RegExp(`(<td>Gravité</td><td>)(?:<strong>)?${s}(?:</strong>)?`, 'g'), `$1${sev(s)}`);
  }
  for (const s of STATES) {
    out = out.split(`<td>${s}</td>`).join(`<td>${state(s)}</td>`);
    out = out.replace(new RegExp(`(<td>État</td><td>)${s}`, 'g'), `$1${state(s)}`);
  }
  for (const [s, k] of Object.entries(STATUSES)) {
    const label = `<span class="st st-${k}">${s}</span>`;
    out = out.split(`<td>${s}</td>`).join(`<td>${label}</td>`);
    out = out.replace(new RegExp(`(<td>Position de l(?:'|&#39;)éditeur</td><td>)(?:<strong>)?${s}(?:</strong>)?`, 'g'), `$1${label}`);
  }
  return out;
}

const section = (b) => `<section class="block${b.level === 2 ? ' fiche' : ''}">${labelled(b.html)}</section>`;

function tocHtml(blocks, pages) {
  const rows = blocks
    .map((b, i) => {
      const sev = b.severity ? `<span class="sev sev-${b.severity}">${b.severity}</span>` : '';
      const st = b.status ? `<span class="st st-${STATUSES[b.status]}">${b.status}</span>` : '';
      return `<li class="l${b.level}"><a href="#${b.id}">${b.title}</a>${sev}${st}<span class="dots"></span><span class="page">${pages[i] ?? ''}</span></li>`;
    })
    .join('');
  return `<section class="block toc"><h1>Sommaire</h1><ol>${rows}</ol></section>`;
}

/** How many pages a PDF made by Chromium holds. */
const pagesOf = (pdf) => (pdf.toString('latin1').match(/\/Type\s*\/Page\b(?!s)/g) ?? []).length;

async function build() {
  const md = fs.readFileSync(SOURCE, 'utf8');
  const html = toHtml(md);
  // The title page is the first block of the source, before the first chapter.
  const coverEnd = html.indexOf('<h1 ');
  const cover = `<section class="cover">${html.slice(0, coverEnd)}</section>`;
  const blocks = blocksOf(html.slice(coverEnd));
  fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch();
  try {
    const tab = await browser.newPage();
    const render = async (body) => {
      await tab.setContent(page(body), { waitUntil: 'load' });
      await tab.evaluate(() => document.fonts.ready);
      return tab.pdf({ preferCSSPageSize: true, printBackground: true });
    };
    // Each block alone, for its length; then the table of contents, whose own length moves every number after it.
    const lengths = [];
    for (const b of blocks) lengths.push(pagesOf(await render(section(b))));
    let tocPages = 1;
    let starts = [];
    for (let pass = 0; pass < 4; pass++) {
      let at = 1 + tocPages + 1;
      starts = lengths.map((n) => {
        const start = at;
        at += n;
        return start;
      });
      const now = pagesOf(await render(tocHtml(blocks, starts)));
      if (now === tocPages) break;
      tocPages = now;
    }
    const whole = page(cover + tocHtml(blocks, starts) + blocks.map(section).join(''));
    fs.writeFileSync(path.join(OUT, 'rapport.html'), whole);
    const pdf = await render(whole);
    const total = pagesOf(pdf);
    const expected = 1 + tocPages + lengths.reduce((a, b) => a + b, 0);
    if (total !== expected)
      throw new Error(`the PDF has ${total} pages, the table of contents counted ${expected}: its numbers would be wrong`);
    fs.writeFileSync(PDF, pdf);
    console.log(
      `${path.relative(ROOT, PDF)}: ${total} pages, ${blocks.filter((b) => b.level === 1).length} chapters, ${blocks.filter((b) => b.level === 2).length} fiches, ${(pdf.length / 1024).toFixed(0)} KiB`,
    );
  } finally {
    await browser.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const clean = check();
  if (!clean) process.exit(1);
  if (!process.argv.includes('--check')) await build();
}

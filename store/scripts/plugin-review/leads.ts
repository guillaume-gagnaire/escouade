// The leads the review script prepares for the one who reads the code (docs/STORE.md §4.2, step 3): what the plugin's
// files do that its permissions may not cover. A lead is not a finding: it says where to look. Nothing here runs the
// plugin; its files are read as text (or noted as binary), line by line.

import { hostCovered } from '../../../schemas/plugin-manifest.mjs';

export type LeadKind = 'network' | 'host' | 'files' | 'secrets' | 'exec' | 'dynamic' | 'obscure' | 'dependencies' | 'instructions';

export interface Lead {
  kind: LeadKind;
  file: string;
  /** 1-based; absent for what holds for the whole file. */
  line?: number;
  /** The line, trimmed and cut to 160 characters. */
  text?: string;
  /** Why it is a lead, in French: the report's language. */
  why: string;
}

/** What the leads are titled in the report, in the order they are shown. */
export const KINDS: Record<LeadKind, string> = {
  host: 'Hôtes hors de `permissions.network`',
  network: 'Appels réseau',
  secrets: 'Secrets et dossiers personnels',
  files: 'Fichiers hors du dossier du plugin',
  exec: 'Programmes lancés',
  dynamic: 'Code dynamique',
  obscure: 'Code illisible',
  dependencies: 'Dépendances',
  instructions: 'Consignes aux agents',
};

interface Pattern {
  kind: LeadKind;
  re: RegExp;
  why: string;
}

const PATTERNS: Pattern[] = [
  { kind: 'network', re: /\bfetch\s*\(/, why: 'fetch' },
  {
    kind: 'network',
    re: /\brequire\(\s*['"](?:node:)?(?:https?|http2|net|tls|dgram|dns)['"]\s*\)|from\s+['"](?:node:)?(?:https?|http2|net|tls|dgram|dns)['"]/,
    why: 'module réseau de Node',
  },
  { kind: 'network', re: /\bnew\s+WebSocket\b|\bXMLHttpRequest\b|\bEventSource\b/, why: 'WebSocket, XMLHttpRequest ou EventSource' },
  {
    kind: 'network',
    re: /\b(?:axios|got|undici|node-fetch|reqwest|ureq|hyper|requests|urllib|aiohttp|socket)\b\s*[.(:]/,
    why: 'client réseau',
  },
  { kind: 'secrets', re: /\bprocess\.env\b|\bstd::env::var|\bos\.environ|\bgetenv\s*\(|\bDeno\.env\b/, why: 'variables d’environnement' },
  {
    kind: 'secrets',
    re: /\.ssh\b|\.aws\b|\.gnupg\b|\.npmrc\b|\.netrc\b|\.docker[\\/]config|\.kube[\\/]config|\.git-credentials/,
    why: 'chemin de clés ou d’identifiants',
  },
  {
    kind: 'secrets',
    re: /\.claude\b|\.codex\b|\.escouade\b|\.cursor\b|\.copilot\b|\.config[\\/](?:gh|opencode)\b/,
    why: 'dossier d’un harness ou d’Escouade',
  },
  {
    kind: 'secrets',
    re: /\bkeytar\b|\bkeyring\b|\bcmdkey\b|find-(?:generic|internet)-password|secret-tool|CredRead|\bkeychain\b/i,
    why: 'trousseau du système',
  },
  { kind: 'files', re: /\bos\.homedir\s*\(|\bhomedir\b|\bUSERPROFILE\b|\bAPPDATA\b|\bHOME\b|['"`]~[\\/]/, why: 'dossier de l’utilisateur' },
  { kind: 'files', re: /['"`](?:\/etc\/|\/usr\/|\/var\/|\/root\/|\/Users\/|\/home\/|[A-Za-z]:\\\\)/, why: 'chemin absolu' },
  {
    kind: 'files',
    re: /\b(?:writeFile|appendFile|createWriteStream|rmSync|rm|unlink|rename|chmod|symlink)(?:Sync)?\s*\(|\bfs::(?:write|remove_file|remove_dir_all)/,
    why: 'écriture ou suppression de fichier',
  },
  { kind: 'files', re: /['"`]\.\.[\\/]/, why: 'chemin qui remonte (`..`)' },
  { kind: 'exec', re: /\bchild_process\b|\b(?:spawn|spawnSync|exec|execSync|execFile|execFileSync|fork)\s*\(/, why: 'lance un programme' },
  {
    kind: 'exec',
    re: /\bCommand::new\b|\bstd::process\b|\bsubprocess\b|\bos\.system\b|\bos\.popen\b|\bRuntime\.getRuntime\(\)\.exec|\bDeno\.(?:run|Command)\b|\bBun\.spawn\b/,
    why: 'lance un programme',
  },
  {
    kind: 'dynamic',
    re: /\beval\s*\(|\bnew\s+Function\s*\(|\bvm\.(?:runIn\w+|Script|compileFunction)\b|\bsetTimeout\s*\(\s*['"`]/,
    why: 'code construit à l’exécution',
  },
  { kind: 'dynamic', re: /\bimport\s*\(\s*(?!['"`]\.{0,2}\/)|\brequire\s*\(\s*[^'"`\s)]/, why: 'import ou require dont le nom se calcule' },
  { kind: 'dynamic', re: /\bimport\s*\(\s*['"`]https?:/, why: 'import d’une URL' },
  {
    kind: 'obscure',
    re: /\batob\s*\(|Buffer\.from\([^)]*['"]base64['"]|\bbase64\s*-d\b|\bfromCharCode\b|\\x[0-9a-f]{2}(?:\\x[0-9a-f]{2}){7}/i,
    why: 'décodage ou texte échappé',
  },
  {
    kind: 'instructions',
    re: /ignore\s+(?:all\s+|any\s+)?(?:the\s+)?(?:previous|prior|above)\s+instructions|disregard\s+(?:all\s+|any\s+)?(?:previous|prior)|ignore[sz]?\s+(?:toutes\s+)?les\s+consignes|you\s+are\s+now\b|system\s+prompt|<\/?system>|do\s+not\s+tell\s+the\s+user|sans\s+le\s+dire\s+à\s+l.utilisateur/i,
    why: 'texte qui s’adresse à un agent',
  },
];

/** Files whose name says they are read by agents: what the plugin adds there is instructions. */
const INSTRUCTION_FILES =
  /(?:^|\/)(?:CLAUDE|AGENTS|GEMINI|COPILOT)\.md$|(?:^|\/)\.cursor\/rules\/|(?:^|\/)\.github\/copilot-instructions\.md$|\.prompt(?:\.md)?$|(?:^|\/)(?:instructions|consignes|rules)\.(?:md|txt)$/i;
const IMAGE = /\.(?:png|jpe?g|gif|webp|ico)$/i;
const URL = /\b(?:https?|wss?):\/\/([a-z0-9.-]+\.[a-z]{2,}|\[[0-9a-f:]+\]|\d{1,3}(?:\.\d{1,3}){3})(?::\d+)?/gi;
/** Hosts a URL names without the plugin talking to them: documentation, schemas, licences. */
const QUIET_HOSTS =
  /^(?:escouade\.dev|www\.w3\.org|json-schema\.org|opensource\.org|www\.gnu\.org|spdx\.org|choosealicense\.com|semver\.org|example\.(?:com|org|net)|localhost)$/i;
/** Files where a URL is documentation, not a call. */
const PROSE = /\.(?:md|markdown|txt)$|^LICEN[CS]E|^COPYING/i;

const cut = (s: string) => {
  const t = s.trim();
  return t.length > 160 ? `${t.slice(0, 157)}…` : t;
};

/**
 * Whether `host` is one `permissions.network` lets the plugin reach: the app's own rule (`hostCovered`, the one its
 * webhooks are sent by), so that the review flags what the app would refuse; `*.domain` covers its subdomains, not itself.
 */
export function hostAllowed(host: string, network: string[]): boolean {
  return hostCovered(host, network);
}

/** Whether `bytes` look binary: a NUL in their first 8 KB. */
export const isBinary = (bytes: Buffer) => bytes.subarray(0, 8192).includes(0);

/** The leads of one `package.json`: install scripts, dependencies that are not pinned or come from elsewhere. */
function packageLeads(file: string, text: string): Lead[] {
  const out: Lead[] = [];
  let pkg: Record<string, unknown>;
  try {
    pkg = JSON.parse(text) as Record<string, unknown>;
  } catch {
    return [{ kind: 'dependencies', file, why: '`package.json` illisible' }];
  }
  const lineOf = (needle: string) => {
    const i = text.split('\n').findIndex((l) => l.includes(needle));
    return i < 0 ? undefined : i + 1;
  };
  const scripts = (pkg.scripts ?? {}) as Record<string, unknown>;
  for (const name of ['preinstall', 'install', 'postinstall', 'prepare', 'prepublish', 'preprepare', 'postprepare'])
    if (typeof scripts[name] === 'string')
      out.push({
        kind: 'dependencies',
        file,
        line: lineOf(`"${name}"`),
        text: cut(`"${name}": ${JSON.stringify(scripts[name])}`),
        why: `script \`${name}\` : il tourne à l’installation`,
      });
  for (const field of ['dependencies', 'optionalDependencies', 'peerDependencies']) {
    const deps = (pkg[field] ?? {}) as Record<string, unknown>;
    for (const [name, spec] of Object.entries(deps)) {
      const s = String(spec);
      const line = lineOf(`"${name}"`);
      if (/^(?:git|github:|https?:|file:|link:)|\//.test(s) && !/^npm:/.test(s))
        out.push({ kind: 'dependencies', file, line, text: cut(`"${name}": "${s}"`), why: 'dépendance hors du registre npm' });
      else if (!/^\d+\.\d+\.\d+(?:-[\w.]+)?$/.test(s.replace(/^npm:[^@]+@/, '')))
        out.push({ kind: 'dependencies', file, line, text: cut(`"${name}": "${s}"`), why: 'version non épinglée' });
    }
  }
  return out;
}

/**
 * The leads of the plugin's files, given what its manifest grants. `files` is path → bytes (as the archive held them),
 * `network` the hosts of `permissions.network`.
 */
export function findLeads(files: Map<string, Buffer>, network: string[]): Lead[] {
  const out: Lead[] = [];
  for (const [file, bytes] of [...files].sort(([a], [b]) => a.localeCompare(b))) {
    if (IMAGE.test(file)) continue;
    if (isBinary(bytes)) {
      out.push({ kind: 'obscure', file, why: `fichier binaire (${bytes.length} octets) : on ne lit pas ce qu’il fait` });
      continue;
    }
    if (INSTRUCTION_FILES.test(file))
      out.push({ kind: 'instructions', file, why: 'fichier lu par les agents : chaque phrase y est une consigne' });
    const text = bytes.toString('utf8');
    if (/(?:^|\/)package\.json$/.test(file)) out.push(...packageLeads(file, text));
    const prose = PROSE.test(file.split('/').pop()!);
    const lines = text.split('\n');
    lines.forEach((raw, i) => {
      const line = i + 1;
      if (raw.length > 500)
        out.push({ kind: 'obscure', file, line, text: cut(raw), why: `ligne de ${raw.length} caractères (minifiée ?)` });
      if (/[A-Za-z0-9+/]{200,}={0,2}/.test(raw)) out.push({ kind: 'obscure', file, line, text: cut(raw), why: 'long bloc en base64' });
      // A string in base64 that decodes to text: what it hides, shown.
      for (const m of raw.matchAll(/['"`]([A-Za-z0-9+/]{24,}={0,2})['"`]/g)) {
        const decoded = Buffer.from(m[1]!, 'base64').toString('utf8');
        if (/^[\x20-\x7e\t\r\n]+$/.test(decoded) && /[^A-Za-z0-9]/.test(decoded))
          out.push({ kind: 'obscure', file, line, text: cut(decoded), why: 'texte en base64, décodé' });
      }
      for (const p of PATTERNS) {
        if (prose && p.kind !== 'instructions') continue;
        if (p.re.test(raw)) out.push({ kind: p.kind, file, line, text: cut(raw), why: p.why });
      }
      if (prose) return;
      for (const m of raw.matchAll(URL)) {
        const host = m[1]!.replace(/^\[|\]$/g, '');
        if (QUIET_HOSTS.test(host) || hostAllowed(host, network)) continue;
        out.push({ kind: 'host', file, line, text: cut(raw), why: `${host} n’est pas dans permissions.network` });
      }
    });
  }
  return out;
}

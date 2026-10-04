// The mark shown before a file's name in the tree, as VS Code's Seti icons do: a short colored glyph by type.

export interface FileIcon {
  glyph: string;
  color: string;
}

const BLUE = 'var(--info)';
const YELLOW = 'var(--wait)';
const ORANGE = 'var(--accent)';
const RED = 'var(--del)';
const GREEN = 'var(--ok)';
const PURPLE = 'oklch(0.72 0.11 305)';
const GREY = 'var(--muted)';
const PLAIN: FileIcon = { glyph: '≡', color: 'var(--dim)' };

const icon = (glyph: string, color: string): FileIcon => ({ glyph, color });

const BY_EXT: Record<string, FileIcon> = {
  ts: icon('TS', BLUE),
  mts: icon('TS', BLUE),
  cts: icon('TS', BLUE),
  tsx: icon('⚛', BLUE),
  jsx: icon('⚛', BLUE),
  js: icon('JS', YELLOW),
  mjs: icon('JS', YELLOW),
  cjs: icon('JS', YELLOW),
  json: icon('{}', YELLOW),
  jsonc: icon('{}', YELLOW),
  md: icon('M↓', BLUE),
  mdx: icon('M↓', BLUE),
  rs: icon('RS', ORANGE),
  py: icon('PY', BLUE),
  css: icon('#', BLUE),
  scss: icon('#', RED),
  html: icon('<>', ORANGE),
  svelte: icon('S', ORANGE),
  vue: icon('V', GREEN),
  yaml: icon('Y', PURPLE),
  yml: icon('Y', PURPLE),
  toml: icon('⚙', GREY),
  ini: icon('⚙', GREY),
  sql: icon('DB', YELLOW),
  xml: icon('<>', ORANGE),
  svg: icon('◇', YELLOW),
  png: icon('▣', PURPLE),
  jpg: icon('▣', PURPLE),
  jpeg: icon('▣', PURPLE),
  gif: icon('▣', PURPLE),
  webp: icon('▣', PURPLE),
  ico: icon('▣', PURPLE),
  java: icon('J', RED),
  kt: icon('K', ORANGE),
  c: icon('C', BLUE),
  h: icon('C', PURPLE),
  cpp: icon('C+', BLUE),
  hpp: icon('C+', PURPLE),
  cc: icon('C+', BLUE),
  cs: icon('C#', BLUE),
  go: icon('GO', BLUE),
  php: icon('php', PURPLE),
  rb: icon('RB', RED),
  swift: icon('SW', ORANGE),
  sh: icon('>_', GREEN),
  bash: icon('>_', GREEN),
  ps1: icon('>_', BLUE),
  bat: icon('>_', YELLOW),
  cmd: icon('>_', YELLOW),
  lock: icon('lck', GREY),
  txt: icon('≡', GREY),
  log: icon('≡', GREY),
  csv: icon('≡', GREEN),
  prisma: icon('◭', BLUE),
  tf: icon('TF', PURPLE),
};

export function fileIcon(path: string): FileIcon {
  const name = path.slice(path.lastIndexOf('/') + 1).toLowerCase();
  if (!name) return PLAIN;
  if (name === 'dockerfile' || name.startsWith('dockerfile.')) return icon('D', BLUE);
  if (name.startsWith('.git')) return icon('git', ORANGE);
  if (name === '.env' || name.startsWith('.env.')) return icon('env', YELLOW);
  if (name === 'license' || name.startsWith('license.')) return icon('©', YELLOW);
  if (/-lock\.json$|\.lock$|^pnpm-lock\.yaml$/.test(name)) return BY_EXT.lock;
  const ext = name.slice(name.lastIndexOf('.') + 1);
  return name.includes('.') && Object.hasOwn(BY_EXT, ext) ? BY_EXT[ext] : PLAIN;
}

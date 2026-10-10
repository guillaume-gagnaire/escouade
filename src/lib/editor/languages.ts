// The language of a file, by extension (or name), and its syntax for the editor, loaded on demand.

import type { Extension } from '@codemirror/state';
import { t } from '../i18n';

interface Lang {
  label: string;
  load?: () => Promise<Extension>;
}

/** A CodeMirror 5 mode of @codemirror/legacy-modes. */
async function legacy(mod: Promise<Record<string, any>>, name: string): Promise<Extension> {
  const [{ StreamLanguage }, m] = await Promise.all([import('@codemirror/language'), mod]);
  return StreamLanguage.define(m[name]);
}

const js = (o: { typescript?: boolean; jsx?: boolean }) => () => import('@codemirror/lang-javascript').then((m) => m.javascript(o));

const LANGS: Record<string, Lang> = {
  ts: { label: 'TypeScript', load: js({ typescript: true }) },
  mts: { label: 'TypeScript', load: js({ typescript: true }) },
  cts: { label: 'TypeScript', load: js({ typescript: true }) },
  tsx: { label: 'TypeScript JSX', load: js({ typescript: true, jsx: true }) },
  js: { label: 'JavaScript', load: js({}) },
  mjs: { label: 'JavaScript', load: js({}) },
  cjs: { label: 'JavaScript', load: js({}) },
  jsx: { label: 'JavaScript JSX', load: js({ jsx: true }) },
  json: { label: 'JSON', load: () => import('@codemirror/lang-json').then((m) => m.json()) },
  md: { label: 'Markdown', load: () => import('@codemirror/lang-markdown').then((m) => m.markdown()) },
  rs: { label: 'Rust', load: () => import('@codemirror/lang-rust').then((m) => m.rust()) },
  py: { label: 'Python', load: () => import('@codemirror/lang-python').then((m) => m.python()) },
  css: { label: 'CSS', load: () => import('@codemirror/lang-css').then((m) => m.css()) },
  scss: { label: 'SCSS', load: () => import('@codemirror/lang-css').then((m) => m.css()) },
  html: { label: 'HTML', load: () => import('@codemirror/lang-html').then((m) => m.html()) },
  svelte: { label: 'Svelte', load: () => import('@codemirror/lang-html').then((m) => m.html()) },
  vue: { label: 'Vue', load: () => import('@codemirror/lang-html').then((m) => m.html()) },
  yaml: { label: 'YAML', load: () => import('@codemirror/lang-yaml').then((m) => m.yaml()) },
  yml: { label: 'YAML', load: () => import('@codemirror/lang-yaml').then((m) => m.yaml()) },
  sql: { label: 'SQL', load: () => import('@codemirror/lang-sql').then((m) => m.sql()) },
  xml: { label: 'XML', load: () => import('@codemirror/lang-xml').then((m) => m.xml()) },
  svg: { label: 'SVG', load: () => import('@codemirror/lang-xml').then((m) => m.xml()) },
  java: { label: 'Java', load: () => import('@codemirror/lang-java').then((m) => m.java()) },
  c: { label: 'C', load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()) },
  h: { label: 'C', load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()) },
  cpp: { label: 'C++', load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()) },
  hpp: { label: 'C++', load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()) },
  cc: { label: 'C++', load: () => import('@codemirror/lang-cpp').then((m) => m.cpp()) },
  php: { label: 'PHP', load: () => import('@codemirror/lang-php').then((m) => m.php()) },
  go: { label: 'Go', load: () => import('@codemirror/lang-go').then((m) => m.go()) },
  toml: { label: 'TOML', load: () => legacy(import('@codemirror/legacy-modes/mode/toml'), 'toml') },
  sh: { label: 'Shell', load: () => legacy(import('@codemirror/legacy-modes/mode/shell'), 'shell') },
  bash: { label: 'Shell', load: () => legacy(import('@codemirror/legacy-modes/mode/shell'), 'shell') },
  ps1: { label: 'PowerShell', load: () => legacy(import('@codemirror/legacy-modes/mode/powershell'), 'powerShell') },
  dockerfile: { label: 'Dockerfile', load: () => legacy(import('@codemirror/legacy-modes/mode/dockerfile'), 'dockerFile') },
  rb: { label: 'Ruby', load: () => legacy(import('@codemirror/legacy-modes/mode/ruby'), 'ruby') },
  swift: { label: 'Swift', load: () => legacy(import('@codemirror/legacy-modes/mode/swift'), 'swift') },
  kt: { label: 'Kotlin', load: () => legacy(import('@codemirror/legacy-modes/mode/clike'), 'kotlin') },
  cs: { label: 'C#', load: () => legacy(import('@codemirror/legacy-modes/mode/clike'), 'csharp') },
  prisma: { label: 'Prisma' },
  tf: { label: 'Terraform' },
};

function langOf(path: string): Lang | null {
  const name = path.slice(path.lastIndexOf('/') + 1).toLowerCase();
  if (name === 'dockerfile') return LANGS.dockerfile;
  const dot = name.lastIndexOf('.');
  return dot > 0 ? (LANGS[name.slice(dot + 1)] ?? null) : null;
}

export function languageLabel(path: string): string {
  return langOf(path)?.label ?? t('editor.language.text');
}

export async function loadLanguage(path: string): Promise<Extension | null> {
  const l = langOf(path);
  return l?.load ? l.load() : null;
}

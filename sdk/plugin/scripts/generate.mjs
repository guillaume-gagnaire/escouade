#!/usr/bin/env node
// The generated files of @escouade-dev/plugin (ESC-181, PLG-9), from the one source of each:
// - src/protocol.ts, the protocol's types, from schemas/plugin-protocol-1.json;
// - src/manifest.ts, the manifest's types, from schemas/plugin-1.json (itself written by the core, manifest::schema());
// - vendor/plugin-manifest.mjs and its .d.ts, the checks of the manifest the store and the core share, copied.
//
// `node sdk/plugin/scripts/generate.mjs` writes them; `--check` says which differ from what the schemas give and
// exits 1 (a test plays it: one cannot change without the other).
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const here = dirname(fileURLToPath(import.meta.url));
export const PACKAGE = resolve(here, '..');
export const ROOT = resolve(PACKAGE, '..', '..');

const HEADER = (source) => `// Generated from ${source} by sdk/plugin/scripts/generate.mjs: do not edit, run it again.\n`;

const IDENT = /^[A-Za-z_$][A-Za-z0-9_$]*$/;
const key = (k) => (IDENT.test(k) ? k : JSON.stringify(k));
const quote = (s) => `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;

function doc(text, indent) {
  if (!text) return '';
  const lines = String(text).split('\n');
  if (lines.length === 1) return `${indent}/** ${text.replace(/\*\//g, '*\\/')} */\n`;
  return `${indent}/**\n${lines.map((l) => `${indent} * ${l}`).join('\n')}\n${indent} */\n`;
}

/** The TypeScript type of a JSON schema (the subset the two schemas use). */
export function typeOf(schema, indent = '') {
  if (schema === true || schema === undefined) return 'unknown';
  if (schema === false) return 'never';
  if (schema.$ref) return schema.$ref.replace(/^#\/definitions\//, '');
  if (schema.const !== undefined) return JSON.stringify(schema.const);
  if (schema.enum) return schema.enum.map((v) => (typeof v === 'string' ? quote(v) : JSON.stringify(v))).join(' | ');
  if (schema.oneOf) return schema.oneOf.map((s) => typeOf(s, indent)).join(' | ');
  // An anyOf of required fields alone only constrains the object it is in.
  if (schema.anyOf && !schema.anyOf.every((s) => Object.keys(s).every((k) => k === 'required'))) {
    return schema.anyOf.map((s) => typeOf(s, indent)).join(' | ');
  }
  const types = Array.isArray(schema.type) ? schema.type : [schema.type];
  if (types.length > 1) return types.map((t) => typeOf({ ...schema, type: t }, indent)).join(' | ');
  switch (types[0]) {
    case 'string':
      return 'string';
    case 'integer':
    case 'number':
      return 'number';
    case 'boolean':
      return 'boolean';
    case 'null':
      return 'null';
    case 'array': {
      const item = typeOf(schema.items, indent);
      return /^[A-Za-z0-9_']+$/.test(item) ? `${item}[]` : `Array<${item}>`;
    }
    case 'object':
      return objectOf(schema, indent);
    default:
      return 'unknown';
  }
}

function objectOf(schema, indent) {
  const props = schema.properties ?? {};
  const required = new Set(schema.required ?? []);
  const extra = schema.additionalProperties;
  const names = Object.keys(props);
  if (names.length === 0) {
    if (extra && extra !== true) return `Record<string, ${typeOf(extra, indent)}>`;
    return 'Record<string, unknown>';
  }
  const inner = indent + '  ';
  let out = '{\n';
  for (const name of names) {
    const p = props[name];
    out += doc(p.description, inner);
    out += `${inner}${key(name)}${required.has(name) ? '' : '?'}: ${typeOf(p, inner)};\n`;
  }
  if (extra === true) out += `${inner}[field: string]: unknown;\n`;
  return out + `${indent}}`;
}

function declaration(name, schema) {
  const t = typeOf(schema, '');
  const head = doc(schema.description, '');
  if (t.startsWith('{\n')) return `${head}export interface ${name} ${t}\n`;
  return `${head}export type ${name} = ${t};\n`;
}

export function protocolTs(schema, source = 'schemas/plugin-protocol-1.json') {
  let out = HEADER(source);
  out += doc(schema.description, '');
  out += `export const PROTOCOL_VERSION = ${schema['x-protocolVersion']};\n\n`;
  for (const [name, def] of Object.entries(schema.definitions)) out += declaration(name, def) + '\n';
  const facts = schema.definitions.FactName.enum;
  out += `/** Every fact, in the order of the schema. */\nexport const FACT_NAMES: readonly FactName[] = [${facts.map(quote).join(', ')}];\n\n`;
  const methods = Object.entries(schema['x-methods']);
  for (const [direction, title, what] of [
    ['toPlugin', 'ToPlugin', 'What Escouade sends to the plugin'],
    ['toEscouade', 'ToEscouade', 'What the plugin sends to Escouade'],
  ]) {
    out += `/** ${what}: each method, its parameters and its answer (\`void\` for a notification). */\nexport interface ${title} {\n`;
    for (const [m, d] of methods.filter(([, d]) => d.direction === direction)) {
      out += doc(d.description, '  ');
      out += `  ${key(m)}: { params: ${typeOf(d.params, '  ')}; result: ${d.notification ? 'void' : typeOf(d.result, '  ')} };\n`;
    }
    out += '}\n\n';
  }
  out += `/** The delay of each of Escouade's calls, in milliseconds; past it, its default answer applies. */\nexport const TIMEOUTS_MS = {\n`;
  for (const [m, d] of methods) if (d.timeoutMs) out += `  ${key(m)}: ${d.timeoutMs},\n`;
  out += `} as const;\n\n/** The delay of the other calls, in milliseconds. */\nexport const DEFAULT_TIMEOUT_MS = ${schema['x-defaultTimeoutMs']};\n\n`;
  out += `/** The error codes of the protocol. */\nexport const ERRORS = {\n`;
  for (const [n, e] of Object.entries(schema['x-errors'])) out += doc(e.description, '  ') + `  ${n}: ${e.code},\n`;
  return out + '} as const;\n';
}

export function manifestTs(schema, source = 'schemas/plugin-1.json') {
  let out = HEADER(source);
  out += `/** The manifest of a plugin, escouade-plugin.json (${schema.$id}). */\n`;
  out += `export interface Manifest ${objectOf(schema, '')}\n`;
  return out;
}

const read = (p) => readFileSync(join(ROOT, p), 'utf8');

/** The TypeScript, as Prettier writes it in this repository. */
async function pretty(text) {
  // By require: under Vitest, an import of prettier is its browser build, without resolveConfig.
  const prettier = createRequire(import.meta.url)('prettier');
  const options = (await prettier.resolveConfig(join(PACKAGE, 'src', 'protocol.ts'))) ?? {};
  return prettier.format(text, { ...options, plugins: [], parser: 'typescript' });
}

/** Each generated file, by its path in the package, and what it holds. */
export async function generate() {
  return {
    'src/protocol.ts': await pretty(protocolTs(JSON.parse(read('schemas/plugin-protocol-1.json')))),
    'src/manifest.ts': await pretty(manifestTs(JSON.parse(read('schemas/plugin-1.json')))),
    'vendor/plugin-manifest.mjs': read('schemas/plugin-manifest.mjs'),
    'vendor/plugin-manifest.d.ts': read('schemas/plugin-manifest.d.ts'),
  };
}

/** The generated files that differ from what is written. */
export async function stale() {
  return Object.entries(await generate())
    .filter(([path, text]) => {
      try {
        return readFileSync(join(PACKAGE, path), 'utf8').replace(/\r\n/g, '\n') !== text.replace(/\r\n/g, '\n');
      } catch {
        return true;
      }
    })
    .map(([path]) => path);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes('--check')) {
    const differ = await stale();
    for (const p of differ) console.error(`sdk/plugin/${p} differs from its schema: run node sdk/plugin/scripts/generate.mjs`);
    process.exit(differ.length ? 1 : 0);
  }
  for (const [path, text] of Object.entries(await generate())) {
    mkdirSync(dirname(join(PACKAGE, path)), { recursive: true });
    writeFileSync(join(PACKAGE, path), text);
  }
}

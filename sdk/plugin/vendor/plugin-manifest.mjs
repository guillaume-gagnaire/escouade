// The checks of a plugin's manifest, escouade-plugin.json (docs/PLUGINS.md §3.1, ESC-173), as a library without
// dependencies: what the store runs on a submitted plugin (STORE-3, docs/STORE.md §3.3), reading it, never running it.
// The same rules as the app's reader (src-tauri/src/plugins/manifest.rs): both go through the cases of
// tests/fixtures/plugins/manifests.json. The schema for the editors' completion is plugin-1.json, beside this file.
//
//   import { validate, message } from '../schemas/plugin-manifest.mjs';
//   const { ok, errors, ignored } = validate(JSON.parse(text));
//   for (const e of errors) console.log(message(e, 'fr'));

/** The major version of the plugins' protocol Escouade serves (`escouade.api`). */
export const API = 1;
/** The ids of Escouade's own plugins, which only the app gives. */
export const RESERVED_PREFIX = 'dev.escouade.';
/** The folder of the app's programs, in the `run` of a plugin of Escouade's own. */
export const ESCOUADE_DIR = '{escouade}';

const SETTING_TYPES = ['string', 'number', 'boolean', 'select'];
const POINTS_WITH_ID = ['accounts', 'ticketSources', 'forges', 'harnesses'];
const OPEN_POINTS = ['themes', 'webhooks', 'commands', 'menus', 'statusBar', 'badges', 'projectTemplates'];

const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
const join = (path, key) => (path ? `${path}.${key}` : key);
const lowerWord = (w) => /^[a-z0-9]([a-z0-9-]*[a-z0-9])?$/.test(w);
const isId = (s) => s.length <= 100 && s.split('.').length >= 2 && s.split('.').every(lowerWord);
const isSemver = (s) => /^[0-9]+\.[0-9]+\.[0-9]+([-+][0-9A-Za-z.+-]+)?$/.test(s);
const isAppVersion = (s) => /^[0-9]+(\.[0-9]+){0,2}$/.test(s);
const isKey = (s) => /^[A-Za-z][A-Za-z0-9_.-]*$/.test(s);
const isUrl = (s) => /^https?:\/\/./.test(s);
const isInnerPath = (s) => s !== '' && !/^[/\\]/.test(s) && !/^[A-Za-z]:/.test(s) && !s.split(/[/\\]/).includes('..');

/** A version of Escouade as numbers, `2` as much as `2.1.0`. */
const numbers = (s) => s.split('.').map(Number);
const compare = (a, b) => {
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    const d = (a[i] ?? 0) - (b[i] ?? 0);
    if (d) return Math.sign(d);
  }
  return 0;
};
/** Whether `v` is past `max`, compared on `max`'s own numbers (`2.4` takes every 2.4.x). */
const beyond = (v, max) => compare(numbers(v).slice(0, max.split('.').length), numbers(max)) > 0;

/**
 * Checks a manifest's value, field by field. `options.builtin`: a plugin the app itself gives, which may take the ids
 * and the programs of Escouade's own (never true for a submitted plugin).
 * @returns {{ ok: boolean, errors: { path: string, code: string, expected?: string }[], ignored: string[] }}
 */
export function validate(manifest, options = {}) {
  const builtin = !!options.builtin;
  const errors = [];
  const ignored = [];
  const problem = (path, code, expected) => errors.push(expected ? { path, code, expected } : { path, code });
  const ignoreOthers = (o, path, known) => {
    for (const k of Object.keys(o)) if (!known.includes(k)) ignored.push(join(path, k));
  };
  const object = (v, path) => (isObject(v) ? v : (problem(path, 'type', 'object'), null));
  const string = (v, path) => (typeof v === 'string' ? v : (problem(path, 'type', 'string'), null));
  const filled = (v, path) => {
    const s = string(v, path);
    if (s === '') problem(path, 'empty');
    return s || null;
  };
  const words = (v, path) => {
    if (!Array.isArray(v)) return problem(path, 'type', 'array');
    v.forEach((item, i) => filled(item, `${path}[${i}]`));
  };
  const text = (v, path) => {
    if (typeof v === 'string') return v === '' ? problem(path, 'empty') : undefined;
    if (!isObject(v)) return problem(path, 'type', 'text');
    const before = errors.length;
    for (const [lang, t] of Object.entries(v)) if (typeof t !== 'string') problem(join(path, lang), 'type', 'string');
    const some = ['en', 'fr'].some((l) => typeof v[l] === 'string' && v[l] !== '');
    if (!some && errors.length === before) problem(path, 'empty');
  };
  const url = (v, path) => {
    const s = string(v, path);
    if (s !== null && !isUrl(s)) problem(path, 'url');
  };
  const innerPath = (v, path) => {
    const s = string(v, path);
    if (s !== null && !isInnerPath(s)) problem(path, 'path');
  };
  const program = (v, path) => {
    const s = string(v, path);
    if (s === null) return;
    if (s.startsWith(ESCOUADE_DIR)) return builtin ? undefined : problem(path, 'reserved');
    if (!isInnerPath(s)) problem(path, 'path');
  };
  const required = (o, key, path, then) => (key in o ? then(o[key], path) : problem(path, 'required'));

  const o = object(manifest, '');
  if (!o) return { ok: false, errors, ignored };
  ignoreOthers(o, '', [
    '$schema',
    'id',
    'name',
    'version',
    'description',
    'author',
    'license',
    'repository',
    'escouade',
    'run',
    'activation',
    'permissions',
    'contributes',
    'store',
  ]);
  if ('$schema' in o) string(o.$schema, '$schema');
  required(o, 'id', 'id', (v, p) => {
    const s = string(v, p);
    if (s === null) return;
    if (!isId(s)) problem(p, 'id');
    else if (s.startsWith(RESERVED_PREFIX) && !builtin) problem(p, 'reserved');
  });
  required(o, 'name', 'name', text);
  required(o, 'version', 'version', (v, p) => {
    const s = string(v, p);
    if (s !== null && !isSemver(s)) problem(p, 'version');
  });
  if ('description' in o) text(o.description, 'description');
  if ('author' in o) {
    const a = object(o.author, 'author');
    if (a) {
      ignoreOthers(a, 'author', ['name', 'url', 'email']);
      if ('url' in a) url(a.url, 'author.url');
      if ('email' in a) filled(a.email, 'author.email');
      required(a, 'name', 'author.name', filled);
    }
  }
  if ('license' in o) filled(o.license, 'license');
  if ('repository' in o) url(o.repository, 'repository');
  required(o, 'escouade', 'escouade', (v) => {
    const e = object(v, 'escouade');
    if (!e) return;
    ignoreOthers(e, 'escouade', ['api', 'min', 'max']);
    required(e, 'api', 'escouade.api', (a, p) => {
      const s = string(a, p);
      if (s !== null && !/^[1-9][0-9]*$/.test(s)) problem(p, 'api');
    });
    const version = (key) => {
      const p = `escouade.${key}`;
      const s = string(e[key], p);
      if (s === null) return null;
      if (!isAppVersion(s)) return (problem(p, 'version'), null);
      return s;
    };
    if (!('min' in e)) problem('escouade.min', 'required');
    const min = 'min' in e ? version('min') : null;
    const max = 'max' in e ? version('max') : null;
    if (min && max && beyond(min, max)) problem('escouade.max', 'range');
  });
  if ('run' in o) {
    const r = object(o.run, 'run');
    if (r) {
      ignoreOthers(r, 'run', ['command', 'args', 'windows', 'macos', 'linux']);
      required(r, 'command', 'run.command', program);
      if ('args' in r) words(r.args, 'run.args');
      for (const sys of ['windows', 'macos', 'linux']) if (sys in r) program(r[sys], `run.${sys}`);
    }
  }
  if ('activation' in o) words(o.activation, 'activation');
  if ('permissions' in o) {
    const p = object(o.permissions, 'permissions');
    if (p) {
      ignoreOthers(p, 'permissions', ['events', 'network', 'secrets', 'commands', 'decide']);
      for (const key of ['events', 'network', 'secrets', 'commands']) if (key in p) words(p[key], `permissions.${key}`);
      if ('decide' in p && typeof p.decide !== 'boolean') problem('permissions.decide', 'type', 'boolean');
    }
  }
  if ('contributes' in o) {
    const c = object(o.contributes, 'contributes');
    if (c) {
      const known = ['settings', 'secrets', 'escouadeYml', ...POINTS_WITH_ID, ...OPEN_POINTS];
      ignoreOthers(c, 'contributes', known);
      const items = (v, path) => {
        if (!Array.isArray(v)) return (problem(path, 'type', 'array'), []);
        return v.map((item, i) => [`${path}[${i}]`, object(item, `${path}[${i}]`)]).filter(([, it]) => it);
      };
      const keys = (list, field) => {
        const seen = [];
        const good = [];
        for (const [at, it] of list) {
          const p = join(at, field);
          if (!(field in it)) {
            problem(p, 'required');
            continue;
          }
          const k = string(it[field], p);
          if (k === null) continue;
          if (seen.includes(k)) problem(p, 'duplicate');
          seen.push(k);
          if (isKey(k)) good.push(k);
          else problem(p, 'key');
        }
        return good;
      };
      const titled = (list, field, needed) => {
        for (const [at, it] of list) {
          const p = join(at, field);
          if (field in it) text(it[field], p);
          else if (needed) problem(p, 'required');
        }
      };
      if ('settings' in c) {
        const list = items(c.settings, 'contributes.settings');
        keys(list, 'key');
        for (const [at, it] of list) {
          required(it, 'type', join(at, 'type'), (t, p) => {
            const s = string(t, p);
            if (s !== null && !SETTING_TYPES.includes(s)) problem(p, 'enum', SETTING_TYPES.join(', '));
          });
        }
        titled(list, 'title', true);
      }
      if ('secrets' in c) {
        const list = items(c.secrets, 'contributes.secrets');
        keys(list, 'key');
        titled(list, 'title', true);
      }
      let accounts = [];
      for (const point of POINTS_WITH_ID) {
        if (!(point in c)) continue;
        const list = items(c[point], `contributes.${point}`);
        const ids = keys(list, 'id');
        titled(list, 'name', false);
        if (point === 'accounts') {
          accounts = ids;
          continue;
        }
        for (const [at, it] of list) {
          if (!('account' in it)) continue;
          const p = join(at, 'account');
          const a = string(it.account, p);
          if (a !== null && !accounts.includes(a)) problem(p, 'account');
        }
      }
      for (const point of OPEN_POINTS) if (point in c) items(c[point], `contributes.${point}`);
      if ('escouadeYml' in c) object(c.escouadeYml, 'contributes.escouadeYml');
    }
  }
  if ('store' in o) {
    const s = object(o.store, 'store');
    if (s) {
      ignoreOthers(s, 'store', ['categories', 'keywords', 'screenshots', 'docs']);
      for (const key of ['categories', 'keywords']) if (key in s) words(s[key], `store.${key}`);
      if ('screenshots' in s) {
        if (Array.isArray(s.screenshots)) s.screenshots.forEach((p, i) => innerPath(p, `store.screenshots[${i}]`));
        else problem('store.screenshots', 'type', 'array');
      }
      if ('docs' in s) innerPath(s.docs, 'store.docs');
    }
  }
  return { ok: errors.length === 0, errors, ignored };
}

/** Reads the text of a manifest: `validate`, or the whole file wrong when it is no JSON. */
export function validateText(text, options = {}) {
  let value;
  try {
    value = JSON.parse(text);
  } catch {
    return { ok: false, errors: [{ path: '', code: 'type', expected: 'object' }], ignored: [] };
  }
  return validate(value, options);
}

const TYPES = {
  fr: {
    string: 'un texte',
    object: 'un objet',
    array: 'une liste',
    boolean: 'true ou false',
    text: 'un texte, ou un objet de textes par langue',
  },
  en: {
    string: 'a text',
    object: 'an object',
    array: 'a list',
    boolean: 'true or false',
    text: 'a text, or an object of texts by language',
  },
};

const WHAT = {
  fr: {
    required: () => 'manque.',
    type: (e) => `devrait être ${TYPES.fr[e.expected] ?? e.expected}.`,
    empty: () => "est vide (un texte, ou `en` ou `fr` d'un texte dans les deux langues).",
    id: () => "n'est pas un nom de domaine inversé en minuscules (`dev.exemple.slack`).",
    reserved: () => "est réservé aux plugins d'Escouade (`dev.escouade.*`, `{escouade}`).",
    version: () => "n'est pas un numéro de version (`1.2.0`).",
    api: () => 'n\'est pas une version majeure de l\'API des plugins (`"1"`).',
    range: () => 'est plus ancienne que `escouade.min`.',
    url: () => "n'est pas une adresse en http:// ou https://.",
    path: () => 'sort du dossier du plugin (un chemin relatif, sans `..`).',
    key: () => "n'est pas un nom (une lettre, puis lettres, chiffres, `_`, `-` ou `.`).",
    enum: (e) => `vaut l'un de : ${e.expected}.`,
    duplicate: () => 'est déjà pris plus haut.',
    account: () => "n'est pas un compte de `contributes.accounts`.",
  },
  en: {
    required: () => 'is missing.',
    type: (e) => `should be ${TYPES.en[e.expected] ?? e.expected}.`,
    empty: () => 'is empty (a text, or the `en` or `fr` of a text in both languages).',
    id: () => 'is no reversed domain name in lower case (`dev.exemple.slack`).',
    reserved: () => "is kept for Escouade's own plugins (`dev.escouade.*`, `{escouade}`).",
    version: () => 'is no version number (`1.2.0`).',
    api: () => 'is no major version of the plugins\' API (`"1"`).',
    range: () => 'is older than `escouade.min`.',
    url: () => 'is no http:// or https:// address.',
    path: () => "leaves the plugin's folder (a relative path, without `..`).",
    key: () => 'is no name (a letter, then letters, digits, `_`, `-` or `.`).',
    enum: (e) => `is one of: ${e.expected}.`,
    duplicate: () => 'is already taken above.',
    account: () => 'is no account of `contributes.accounts`.',
  },
};

/** What is wrong with a field, in `lang` (`fr` or `en`), its field first: the app's own words. */
export function message(error, lang = 'en') {
  const l = lang === 'fr' ? 'fr' : 'en';
  const field = error.path ? `\`${error.path}\`` : l === 'fr' ? 'Le manifeste' : 'The manifest';
  return `${field} ${WHAT[l][error.code](error)}`;
}

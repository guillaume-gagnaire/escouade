// The types of plugin-manifest.mjs: the checks of a plugin's manifest, escouade-plugin.json (ESC-173).

/** A field of a manifest that is wrong, by its path (`contributes.settings[0].key`; empty for the whole file). */
export interface ManifestError {
  path: string;
  code:
    | 'required'
    | 'type'
    | 'empty'
    | 'id'
    | 'reserved'
    | 'version'
    | 'api'
    | 'range'
    | 'url'
    | 'path'
    | 'key'
    | 'enum'
    | 'duplicate'
    | 'account'
    | 'file'
    | 'png'
    | 'heavy'
    | 'square'
    | 'small'
    | 'network'
    | 'host'
    | 'exclusive'
    | 'cargo'
    | 'command'
    | 'readOnly'
    | 'variable'
    | 'interpreter'
    | 'pointer'
    | 'session'
    | 'misplaced'
    | 'repo'
    | 'bounds'
    | 'extension'
    | 'field'
    | 'profile';
  /** The type, or the values, the field takes, for `type` and `enum`; the whole numbers, `lo..hi`, for `bounds`. */
  expected?: string;
}

/** The host of an address written whole, in lower case; null for none, or one made by a template. */
export function writtenHost(url: string): string | null;
/** Whether `permissions.network` covers `host`: itself, or `*.domain` for its subdomains (not the domain itself). */
/** Whether a word of `permissions.network` is a host too wide to declare: no dot, or a public suffix (audit SEC-09). */
export function networkWordRefused(word: string): boolean;
export function hostCovered(host: string, network: string[]): boolean;
export const FILE_ROOTS: string[];
export const FILE_ACCESS: string[];
/** Whether a line of `permissions.commands` holds: a program's name first, never a path nor a joker; a joker a whole word. */
export function commandHolds(command: string): boolean;
/** What breaks a line of permissions.commands that does not hold (`program:<word>`, `star`, `path:<word>`, `glued:<word>`, `brace:<word>`), for its message; empty when it holds. */
export function commandFault(command: string): string;
/** The words of `permissions.commands` that stand for one argument of a closed form (ESC-525). */
export const JOKERS: readonly string[];
/** Whether a word of a command is one of the jokers, whole. */
export function isJoker(word: string): boolean;
/** Whether a line of `permissions.commands` holds a joker. */
export function hasJoker(command: string): boolean;
/** Where a setting or a secret holds (`scope`): `app`, the default, or `project`. */
export const SCOPES: readonly string[];
/** The lines a field on several lines shows, at least and at most. */
export const LINES: readonly [number, number];
/** The programs at once a plugin may ask for (`limits.processes`), at least and at most. */
export const PROCESSES: readonly [number, number];
/** The programs at once a plugin has without asking. */
export const PROCESSES_UNASKED: number;
/** A name of variable: a letter or `_`, then letters, digits and `_`. */
export function isVarName(name: unknown): boolean;
/** Whether Escouade refuses the variable `name` to a plugin even declared (ESC-424); `firstParty`: of Escouade's own hand. */
export function envRefused(name: string, firstParty?: boolean): boolean;
/** Whether a harness of anyone's would run a shell or an interpreter (ESC-424). */
export function interpreter(program: string): boolean;

export interface Validation {
  ok: boolean;
  errors: ManifestError[];
  /** The fields Escouade does not know, ignored. */
  ignored: string[];
  /** The reserved points it declares (`contributes.statusBar`): no effect in this version, a warning, never an error. */
  reserved: string[];
}

export interface ValidateOptions {
  /** A plugin the app itself gives, which may take the ids and the programs of Escouade's own. */
  builtin?: boolean;
  /** A plugin of Escouade's own hand not given by the app (`plugins/<id>/`, ESC-314): the ids `dev.escouade.*`, nothing more. */
  firstParty?: boolean;
  /** The bytes of a file of the plugin's folder by its path, null when it is none: to read its `icon`. */
  file?: (path: string) => Uint8Array | null | undefined;
}

export const API: number;
export const RESERVED_PREFIX: string;
export const ESCOUADE_DIR: string;
export const ICON_MIN: number;
export const ICON_BYTES: number;
/** The points the manifest reserves: `menus`, `statusBar`, `badges`, `projectTemplates`, `escouadeYml` (ESC-524). */
export const RESERVED_POINTS: readonly string[];
/** The fields known, object by object (`''` the manifest itself): the properties of plugin-1.json. */
export const FIELDS: Readonly<Record<string, readonly string[]>>;
/** An icon's path as the folder's files are named: `/` between its parts, without `./` in front. */
export function iconPath(path: string): string;
/** What is wrong with an icon's bytes, from its header alone. */
export function iconProblems(bytes: Uint8Array): ('png' | 'heavy' | 'square' | 'small')[];
export function validate(manifest: unknown, options?: ValidateOptions): Validation;
export function validateText(text: string, options?: ValidateOptions): Validation;
export function message(error: ManifestError, lang?: 'fr' | 'en'): string;
/** What is said of a reserved point at `path`: « reserved, no effect in this version ». */
export function reservedMessage(path: string, lang?: 'fr' | 'en'): string;
/** The version of Escouade that serves the profiles, the settings of each project, the jokers and limits.processes. */
export const CONTRACT_MIN: string;
/** What a manifest takes of the contract served since CONTRACT_MIN when its escouade.min is older: a warning of validate, or null. */
export function belowContract(manifest: unknown, lang?: 'fr' | 'en'): string | null;

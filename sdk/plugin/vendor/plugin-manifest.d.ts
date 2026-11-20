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
    | 'exclusive'
    | 'cargo'
    | 'command'
    | 'readOnly'
    | 'variable'
    | 'interpreter'
    | 'pointer'
    | 'session';
  /** The type, or the values, the field takes, for `type` and `enum`. */
  expected?: string;
}

/** The host of an address written whole, in lower case; null for none, or one made by a template. */
export function writtenHost(url: string): string | null;
/** Whether `permissions.network` covers `host`: itself, or `*.domain` for its subdomains (not the domain itself). */
export function hostCovered(host: string, network: string[]): boolean;
export const FILE_ROOTS: string[];
export const FILE_ACCESS: string[];
export function commandHolds(command: string): boolean;
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

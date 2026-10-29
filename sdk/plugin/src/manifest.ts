// Generated from schemas/plugin-1.json by sdk/plugin/scripts/generate.mjs: do not edit, run it again.
/** The manifest of a plugin, escouade-plugin.json (https://escouade.dev/schemas/plugin-1.json). */
export interface Manifest {
  $schema?: string;
  /** What starts it (`event:ticket.column`, `account:github`); without any, it does not run. */
  activation?: string[];
  author?: {
    email?: string;
    name: string;
    url?: string;
  };
  /** Its extension points. */
  contributes?: {
    /** A kind of account its points share. */
    accounts?: Array<{
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Badges of the cards. */
    badges?: Array<Record<string, unknown>>;
    /** Commands run on an event. */
    commands?: Array<Record<string, unknown>>;
    /** Its keys of .escouade.yml. */
    escouadeYml?: Record<string, unknown>;
    /** Code hosts: pull requests, CI, deployments. */
    forges?: Array<{
      /** The id of one of `contributes.accounts`. */
      account?: string;
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Harnesses. */
    harnesses?: Array<{
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Menus. */
    menus?: Array<Record<string, unknown>>;
    /** Templates of the project wizard. */
    projectTemplates?: Array<Record<string, unknown>>;
    secrets?: Array<{
      key: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
    }>;
    settings?: Array<{
      key: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
      type: 'string' | 'number' | 'boolean' | 'select';
    }>;
    /** Items of the status bar. */
    statusBar?: Array<Record<string, unknown>>;
    /** Themes. */
    themes?: Array<Record<string, unknown>>;
    /** Sources of the Kanban. */
    ticketSources?: Array<{
      /** The id of one of `contributes.accounts`. */
      account?: string;
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Webhooks, without code. */
    webhooks?: Array<Record<string, unknown>>;
  };
  /** One text, or a text by language (`en` first, then `fr`). */
  description?: string | Record<string, string>;
  /** The Escouade it was written for: out of it, the plugin is disabled, and the reason shown. */
  escouade: {
    /** The major version of the plugins' protocol it speaks (`"1"`). */
    api: string;
    max?: string;
    min: string;
  };
  /** Its logo: a square PNG of its folder, 256 px a side at least (512 advised), 1 MB at most; no SVG. */
  icon?: string;
  /** Unique, a reversed domain name (`dev.exemple.slack`); `dev.escouade.*` is kept for Escouade's own plugins. */
  id: string;
  /** An SPDX identifier (`MIT`). */
  license?: string;
  /** One text, or a text by language (`en` first, then `fr`). */
  name: string | Record<string, string>;
  /** What it asks for, shown before it is installed. */
  permissions?: {
    commands?: string[];
    decide?: boolean;
    events?: string[];
    network?: string[];
    secrets?: string[];
  };
  repository?: string;
  /** Its program; without one, the plugin is declarative: nothing runs. */
  run?: {
    args?: string[];
    /** A path in the plugin's folder: relative, without `..`. */
    command: string;
    /** A path in the plugin's folder: relative, without `..`. */
    linux?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    macos?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    windows?: string;
  };
  /** What store.escouade.dev shows of it. */
  store?: {
    categories?: string[];
    /** A path in the plugin's folder: relative, without `..`. */
    docs?: string;
    keywords?: string[];
    screenshots?: string[];
  };
  /** Its version (`1.2.0`), the same as its git tag. */
  version: string;
}

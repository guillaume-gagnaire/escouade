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
      /** The id of one of `contributes.accounts`. */
      account?: string;
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Launch environments: the up, ls, down and destroy of a worktree. */
    launchEnvironments?: Array<{
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
    /** Readers of workflows Escouade does not read by itself (it gives them the project's files). */
    workflowReaders?: Array<{
      /** The files of the project it is given, as globs from the project's folder (`.planning/**`). */
      files: string[];
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
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
  /** What a plugin in WASM asks for beyond the host's defaults; Escouade caps them at 120 s and 32 MiB for `net/fetch`, 4 MiB for `fs/read`, 256 MiB for its memory, 10 s for a turn. */
  limits?: {
    fetchBytes?: number;
    fetchSeconds?: number;
    /** How much `fs/read` gives at most: 256 KiB without it. */
    fileBytes?: number;
    /** How much linear memory its instance may take: 64 MiB without it. */
    memoryBytes?: number;
    /** How long a turn of its instance may take: 2 s without it. */
    turnSeconds?: number;
  };
  /** One text, or a text by language (`en` first, then `fr`). */
  name: string | Record<string, string>;
  /** What it asks for, shown before it is installed. */
  permissions?: {
    /** The programs it may run (`proc/spawn`): a program's name, never a path, then its arguments, word for word. A last word `*` lets any more follow. */
    commands?: string[];
    decide?: boolean;
    events?: string[];
    /** The folders it may read or write (`fs/*`): a root of the closed list, which the host resolves; no path of its own. `plugin` is read only. */
    files?: Array<{
      access: 'read' | 'write';
      root: 'plugin' | 'agent' | 'agentRepo' | 'agentHome' | 'temp';
    }>;
    network?: string[];
    secrets?: string[];
  };
  repository?: string;
  /** Its program; without one, the plugin is declarative: nothing runs. A command (`command`, a process), a JS/TS bundle of its folder that Escouade runs in WASM (`js`), or the `Cargo.toml` of a Rust module that Escouade runs in WASM (`wasm`): one of the three. */
  run?: {
    args?: string[];
    /** A path in the plugin's folder: relative, without `..`. */
    command?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    js?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    linux?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    macos?: string;
    /** A path in the plugin's folder: relative, without `..`. */
    wasm?: string;
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

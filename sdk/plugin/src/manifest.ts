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
    /** Reserved, with no effect in this version: Escouade shows and calls nothing of it. */
    badges?: Array<Record<string, unknown>>;
    /** Commands run on an event. */
    commands?: Array<Record<string, unknown>>;
    /** Reserved, with no effect in this version: read and ignored. */
    escouadeYml?: Record<string, unknown>;
    /** Code hosts: pull requests, CI, deployments. */
    forges?: Array<{
      /** The id of one of `contributes.accounts`. */
      account?: string;
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      name?: string | Record<string, string>;
    }>;
    /** Groups of its settings and secrets, in order, on its page. */
    groups?: Array<{
      /** What it is for, under its title: one text, or a text by language. */
      description?: string | Record<string, string>;
      /** The keys of its settings and secrets, mixed, in the order they are shown; a key in one group at most. */
      fields: string[];
      /** Folded at first, as « Advanced ». */
      folded?: boolean;
      id: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
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
    /** Reserved, with no effect in this version: Escouade shows and calls nothing of it. */
    menus?: Array<Record<string, unknown>>;
    /** Kinds of profile: sets of fields the user makes as many times as wanted, and that each project chooses (a setting of `type: "profile"`). Their secrets are asked as `profile:<id>` in `permissions.secrets`. */
    profiles?: Array<{
      fields: Array<{
        /** What it is for, under its title: one text, or a text by language. */
        description?: string | Record<string, string>;
        /** The extensions of the files « Import a file… » offers (`.ovpn`): their text fills the field once; the path is never kept. Never from .escouade.yml. */
        import?: string[];
        key: string;
        /** How many lines it shows, with `multiline`: 6 without it. */
        lines?: number;
        /** Typed on several lines (a `string` setting or a secret), 64 KiB at most. */
        multiline?: boolean;
        /** Kept in the keychain, never shown again; it has no `type`. */
        secret?: boolean;
        /** One text, or a text by language (`en` first, then `fr`). */
        title: string | Record<string, string>;
        /** Needed unless it is `secret`. */
        type?: 'string' | 'number' | 'boolean' | 'select';
      }>;
      id: string;
      /** The key of the `string` field that names a profile (neither secret nor on several lines). */
      nameField: string;
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
    }>;
    /** Reserved, with no effect in this version: Escouade shows and calls nothing of it. */
    projectTemplates?: Array<Record<string, unknown>>;
    secrets?: Array<{
      /** What it is for, under its title: one text, or a text by language. */
      description?: string | Record<string, string>;
      /** The extensions of the files « Import a file… » offers (`.ovpn`): their text fills the field once; the path is never kept. Never from .escouade.yml. */
      import?: string[];
      key: string;
      /** How many lines it shows, with `multiline`: 6 without it. */
      lines?: number;
      /** Typed on several lines (a `string` setting or a secret), 64 KiB at most. */
      multiline?: boolean;
      /** A secret never comes from .escouade.yml. */
      repo?: false;
      /** Where it holds: for every project (`app`, the default), or for each one, set in the project's settings (`project`). */
      scope?: 'app' | 'project';
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
    }>;
    /** Its settings, shown on its page in the order of this list, under « Settings » (or in their groups). */
    settings?: Array<{
      /** Its value until the user sets one, of its type (for a `select`, one of its options); without it, empty, or false for a `boolean`. */
      default?: unknown;
      /** Shown under its title: one text, or a text by language (`en` first, then `fr`). */
      description?: unknown;
      /** The extensions of the files « Import a file… » offers (`.ovpn`): their text fills the field once; the path is never kept. Never from .escouade.yml. */
      import?: string[];
      key: string;
      /** How many lines it shows, with `multiline`: 6 without it. */
      lines?: number;
      /** Typed on several lines (a `string` setting or a secret), 64 KiB at most. */
      multiline?: boolean;
      /** With `type: "profile"`: the id of a kind of `contributes.profiles`. */
      of?: string;
      /** The choices of a `select`, in this order: each a value, or `{ "value": …, "title": … }` to show a title in its stead (one text, or a text by language). */
      options?: unknown;
      /** `false`: its value never comes from .escouade.yml. Never `true` for a profile or an imported value. */
      repo?: boolean;
      /** Where it holds: for every project (`app`, the default), or for each one, set in the project's settings (`project`). */
      scope?: 'app' | 'project';
      /** One text, or a text by language (`en` first, then `fr`). */
      title: string | Record<string, string>;
      /** `profile`: one profile of the kind `of` names, chosen by the user. */
      type: 'string' | 'number' | 'boolean' | 'select' | 'profile';
    }>;
    /** Reserved, with no effect in this version: Escouade shows and calls nothing of it. */
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
    /** How many programs it may run at once (`proc/spawn`): 4 without it. Beyond 4, said at its install and with `reason`. */
    processes?: number;
    /** Why it asks for more programs at once, in a sentence: needed with `processes` beyond 4. */
    reason?: string | Record<string, string>;
    /** How long a turn of its instance may take: 2 s without it. */
    turnSeconds?: number;
  };
  /** One text, or a text by language (`en` first, then `fr`). */
  name: string | Record<string, string>;
  /** What it asks for, shown before it is installed. */
  permissions?: {
    /** The programs it may run (`proc/spawn`): a program's name, never a path nor a joker, then its arguments, word for word. A joker stands for one word of a closed form, written whole: `{int}`, `{port}`, `{name}`, `{pid}` (a program it runs), `{path:temp}`, `{path:plugin}`; no other brace. A last word `*` lets any more follow. */
    commands?: string[];
    decide?: boolean;
    /** The variables it may set on what it runs (`proc/spawn`, the launches of its harnesses), shown before it is installed; any other is refused. Never Escouade's own nor `LD_*`, `DYLD_*`, `GIT_*` (but `GIT_AUTHOR_*`, `GIT_COMMITTER_*`), `PYTHON*`, `RUBY*`, `PERL*`, `NODE_*`, `JAVA_TOOL_OPTIONS`, `npm_config_*`, `*_PROXY`, `SSL_*`, `XDG_*`, `HOME`, `USERPROFILE`, `APPDATA`, `PATH`. */
    env?: string[];
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

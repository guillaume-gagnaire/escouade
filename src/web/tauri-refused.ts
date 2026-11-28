// What the page served by `escouade-server --browser` has in place of Tauri (BC-1, ESC-704, docs/REMOTE.md « Le client
// navigateur »): each module `@tauri-apps/*` the interface imports is replaced, in the web build alone
// (vite.web.config.ts), by a stub whose every export refuses, with an error of its own name. Never a `null` nor a value
// that would let a call look as if it had worked: what the window does by Tauri, a browser does by its own means (BC-2's
// `poste`), or says it cannot.

/** The error of a call to Tauri in the browser's build: its module and the name called. */
export class TauriUnavailableError extends Error {
  override readonly name = 'TauriUnavailableError';
  constructor(
    readonly module: string,
    readonly member: string,
  ) {
    super(`${module} › ${member} : absent du client navigateur (pas de Tauri dans cette page)`);
  }
}

/** A function that throws at once: a call that gives back a value of Tauri (`getCurrentWindow()`). */
export function refusing(module: string, member: string): (...args: unknown[]) => never {
  return () => {
    throw new TauriUnavailableError(module, member);
  };
}

/** A function whose promise is refused: a call that waits on Tauri (`invoke`, `open`, `listen`). */
export function refusingAsync(module: string, member: string): (...args: unknown[]) => Promise<never> {
  return () => Promise.reject(new TauriUnavailableError(module, member));
}

/** A class that cannot be made (`new Channel()`). */
export function refusingClass(module: string, member: string): new (...args: unknown[]) => never {
  return class {
    constructor() {
      throw new TauriUnavailableError(module, member);
    }
  } as unknown as new (...args: unknown[]) => never;
}

/** How a member of a stub refuses. */
export type Refusal = 'sync' | 'async' | 'class';

/**
 * The modules of Tauri the interface imports, and what each gives: the stub of each exports these names alone. A module
 * or a name that is not here fails the web build (an import of a name the stub does not export): a new use of Tauri is
 * seen, and given its refusal, before it reaches a browser.
 */
export const STUBS: Record<string, Record<string, Refusal>> = {
  '@tauri-apps/api/core': { invoke: 'async', Channel: 'class' },
  '@tauri-apps/api/event': { listen: 'async', emit: 'async', once: 'async' },
  '@tauri-apps/api/webview': { getCurrentWebview: 'sync' },
  '@tauri-apps/api/window': { getCurrentWindow: 'sync' },
  '@tauri-apps/plugin-dialog': { open: 'async', save: 'async' },
  '@tauri-apps/plugin-opener': { openUrl: 'async', openPath: 'async', revealItemInDir: 'async' },
};

/** The source of the stub of `module` (its import of this file named by `self`), or null when it has none. */
export function stubSource(module: string, self: string): string | null {
  const members = STUBS[module];
  if (!members) return null;
  const how = { sync: 'refusing', async: 'refusingAsync', class: 'refusingClass' } as const;
  const lines = [`import { refusing, refusingAsync, refusingClass } from ${JSON.stringify(self)};`];
  for (const [name, kind] of Object.entries(members))
    lines.push(`export const ${name} = ${how[kind]}(${JSON.stringify(module)}, ${JSON.stringify(name)});`);
  return lines.join('\n') + '\n';
}

/** What the built page must never hold: the marks of Tauri's IPC (BC-1). */
export const FORBIDDEN_IN_BUNDLE = ['__TAURI_INTERNALS__', 'ipc.localhost'] as const;

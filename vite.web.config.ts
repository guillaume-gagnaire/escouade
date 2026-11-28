// The build of the interface for a browser (BC-1, ESC-704, docs/REMOTE.md « Le client navigateur »): the desktop's
// configuration (vite.config.ts), with each module `@tauri-apps/*` replaced by a stub that refuses (src/web/
// tauri-refused.ts), into `dist-web/` (`npm run build:web`). `escouade-server` embeds it (crates/escouade-web) and
// serves it with `--browser`. The built page never holds `__TAURI_INTERNALS__` nor `ipc.localhost`: the build fails
// otherwise. The page learns it is served by the mark the server puts in index.html, never by Tauri's absence.

import { fileURLToPath } from 'node:url';
import { mergeConfig, type Plugin, type UserConfig } from 'vite';
import desktop from './vite.config';
import { FORBIDDEN_IN_BUNDLE, stubSource } from './src/web/tauri-refused';

const REFUSED = fileURLToPath(new URL('./src/web/tauri-refused.ts', import.meta.url));
const STUB = '\0escouade-tauri-stub:';

/** Each `@tauri-apps/*` the code imports, its refusing stub; a module with none fails the build, named. */
export function tauriStubs(): Plugin {
  return {
    name: 'escouade-tauri-stubs',
    enforce: 'pre',
    resolveId(id) {
      if (!id.startsWith('@tauri-apps/')) return null;
      if (stubSource(id, REFUSED) === null)
        this.error(`${id} : aucun bouchon dans src/web/tauri-refused.ts (STUBS) ; ajoute le sien, qui refuse`);
      return STUB + id;
    },
    load(id) {
      if (!id.startsWith(STUB)) return null;
      return stubSource(id.slice(STUB.length), REFUSED);
    },
  };
}

/** The files of a bundle that hold a mark of Tauri's IPC, and which. */
export function tauriMarks(files: Record<string, string | Uint8Array>): string[] {
  const found: string[] = [];
  for (const [name, content] of Object.entries(files)) {
    const text = typeof content === 'string' ? content : Buffer.from(content).toString('latin1');
    for (const mark of FORBIDDEN_IN_BUNDLE) if (text.includes(mark)) found.push(`${name}: ${mark}`);
  }
  return found;
}

/** The bundle written fails when a file of it holds a mark of Tauri's IPC. */
export function noTauriInBundle(): Plugin {
  return {
    name: 'escouade-no-tauri-in-bundle',
    apply: 'build',
    generateBundle(_options, bundle) {
      const files: Record<string, string | Uint8Array> = {};
      for (const [name, out] of Object.entries(bundle)) files[name] = out.type === 'chunk' ? out.code : out.source;
      const found = tauriMarks(files);
      if (found.length) this.error(`le paquet du navigateur garde Tauri : ${found.join(', ')}`);
    },
  };
}

export const web: UserConfig = {
  plugins: [tauriStubs(), noTauriInBundle()],
  // The tests of `inTauri` fall away (`!true && …`): the page never looks for Tauri's IPC.
  define: { 'import.meta.env.ESCOUADE_WEB': 'true' },
  // public/ is the window's (desktop's `publicDir`, kept): What's up ? (index.html's `/whats-up/whats-up.js`), the tour's
  // images, the logo, served by escouade-server at the same paths.
  build: { outDir: 'dist-web', emptyOutDir: true, assetsDir: 'assets' },
};

export default mergeConfig(desktop, web);

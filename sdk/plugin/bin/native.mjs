// Where the escouade-plugin of Escouade is: the native command that plays the app's conformance bench.
import { existsSync, readFileSync, statSync } from 'node:fs';
import { delimiter, join } from 'node:path';

/** The escouade-plugin of Escouade: ESCOUADE_PLUGIN_BIN, the PATH (not an npm one), or where the app installs it. */
export function nativeBin(env = process.env, platform = process.platform) {
  if (env.ESCOUADE_PLUGIN_BIN) return env.ESCOUADE_PLUGIN_BIN;
  const exe = platform === 'win32' ? 'escouade-plugin.exe' : 'escouade-plugin';
  for (const dir of (env.PATH ?? env.Path ?? '').split(delimiter)) {
    if (!dir || dir.split(/[\\/]/).includes('node_modules')) continue;
    const p = join(dir, exe);
    try {
      // npm's own shim is a script; Escouade's is a program (on Windows, an .exe).
      if (statSync(p).isFile() && !readFileSync(p).subarray(0, 2).equals(Buffer.from('#!'))) return p;
    } catch {
      // not there
    }
  }
  const known = {
    win32: env.LOCALAPPDATA && join(env.LOCALAPPDATA, 'Escouade', exe),
    darwin: '/Applications/Escouade.app/Contents/MacOS/escouade-plugin',
    linux: '/usr/bin/escouade-plugin',
  }[platform];
  return known && existsSync(known) ? known : null;
}

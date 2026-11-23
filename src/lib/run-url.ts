// The address a launch command opens once it answers (ESC-560): http or https, checked as it is typed. It may name the
// ports a worktree gives its launches the way a shell does, `${ESCOUADE_PORT_BASE}` and `${ESCOUADE_PORT_END}`, with a
// value for a launch in the project after `:-` (`http://localhost:${ESCOUADE_PORT_BASE:-3000}`). The backend checks
// it the same way (`testlaunch::check_run_url`), and refuses what this would.

import { t } from './i18n';

/** The variables an address may name. */
export const URL_VARS = ['ESCOUADE_PORT_BASE', 'ESCOUADE_PORT_END'] as const;

/** How many ports a worktree gets (the backend's `PORT_BLOCK`). */
const PORT_BLOCK = 10;

/** The variables of a launch: a worktree's ports, none in the project. */
export function portVars(portBase: number | null | undefined): Record<string, string> {
  return portBase == null ? {} : { ESCOUADE_PORT_BASE: String(portBase), ESCOUADE_PORT_END: String(portBase + PORT_BLOCK - 1) };
}

export type Expanded = { url: string; error?: undefined } | { url?: undefined; error: string };

/** `url` with its variables given their values from `vars`, else their value for the project; or why it cannot be. */
export function expandRunUrl(url: string, vars: Record<string, string>): Expanded {
  let out = '';
  let rest = url.trim();
  for (let at = rest.indexOf('${'); at >= 0; at = rest.indexOf('${')) {
    out += rest.slice(0, at);
    const inner = rest.slice(at + 2);
    const end = inner.indexOf('}');
    if (end < 0) return { error: t('runs.url.unclosed') };
    const body = inner.slice(0, end);
    const sep = body.indexOf(':-');
    const name = sep < 0 ? body : body.slice(0, sep);
    const fallback = sep < 0 ? undefined : body.slice(sep + 2);
    if (!(URL_VARS as readonly string[]).includes(name)) return { error: t('runs.url.unknownVar', { name, list: URL_VARS.join(', ') }) };
    const value = vars[name] ?? fallback;
    if (value === undefined) return { error: t('runs.url.noValue', { name, example: `\${${name}:-3000}` }) };
    out += value;
    rest = inner.slice(end + 1);
  }
  return { url: out + rest };
}

/** What is wrong with the address typed, null when it can be saved (none, or an http(s) address with a host). */
export function runUrlProblem(url: string): string | null {
  const typed = url.trim();
  if (!typed) return null;
  const x = expandRunUrl(typed, portVars(4100));
  if (x.error !== undefined) return x.error;
  let parsed: URL;
  try {
    parsed = new URL(x.url);
  } catch {
    return t('runs.url.notAnAddress');
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return t('runs.url.scheme');
  if (!parsed.hostname) return t('runs.url.noHost');
  return null;
}

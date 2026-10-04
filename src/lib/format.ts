// French number / duration formatting, same conventions as the design.

export function fTok(n: number): string {
  if (n >= 1e9) return (n / 1e9).toFixed(2).replace('.', ',') + ' Md';
  if (n >= 1e6) return (n / 1e6).toFixed(2).replace('.', ',') + ' M';
  if (n >= 1e3) return (n / 1e3).toFixed(1).replace('.', ',') + ' k';
  return String(Math.round(n));
}

export function fUsd(x: number): string {
  const digits = x > 0 && x < 0.1 ? 3 : 2;
  return x.toLocaleString('fr-FR', { minimumFractionDigits: digits, maximumFractionDigits: digits }) + ' $';
}

export function fDur(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const ss = s % 60;
  return h ? `${h}h ${String(m).padStart(2, '0')}m` : `${m}m ${String(ss).padStart(2, '0')}s`;
}

/** How long ago `seconds` (Unix time) was, relative to `now` (ms). */
export function fAgo(seconds: number, now: number): string {
  const s = Math.max(0, now / 1000 - seconds);
  if (s < 60) return 'à l’instant';
  if (s < 3600) return `il y a ${Math.floor(s / 60)} min`;
  if (s < 86400) return `il y a ${Math.floor(s / 3600)} h`;
  const d = Math.floor(s / 86400);
  if (d === 1) return 'hier';
  if (d < 30) return `il y a ${d} j`;
  return new Date(seconds * 1000).toLocaleDateString('fr-FR');
}

/** "1h48" style countdown until a timestamp. */
export function fCountdown(target: number | null, now: number): string {
  if (!target) return '—';
  const s = Math.max(0, Math.floor((target - now) / 1000));
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}j ${h}h`;
  return `${h}h${String(m).padStart(2, '0')}`;
}

export function fPct(p: number): string {
  return `${Math.round(p)} %`;
}

export function fInt(n: number): string {
  return Math.round(n).toLocaleString('fr-FR');
}

export function fDate(ts: number): string {
  return new Date(ts).toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' });
}

export function fTime(ts: number): string {
  return new Date(ts).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n > 1 ? many : one}`;
}

export function basename(p: string): string {
  const s = p.replace(/\\/g, '/');
  return s.slice(s.lastIndexOf('/') + 1);
}

export function dirname(p: string): string {
  const s = p.replace(/\\/g, '/');
  const i = s.lastIndexOf('/');
  return i > 0 ? s.slice(0, i) : '.';
}

/** Path relative to `base` when inside it (case-insensitive, Windows friendly). */
export function relPath(base: string, p: string): string {
  const b = base.replace(/\\/g, '/').replace(/\/$/, '');
  const s = p.replace(/\\/g, '/');
  return s.toLowerCase().startsWith(b.toLowerCase() + '/') ? s.slice(b.length + 1) : s;
}

/** `rel` (forward slashes) under the folder `root`, written as its system does: with `\` for a Windows folder. */
export function joinPath(root: string, rel: string): string {
  const sep = /^[A-Za-z]:|\\/.test(root) ? '\\' : '/';
  return (root.replace(/[\\/]+$/, '') + '/' + rel).replace(/[\\/]/g, sep);
}

/** A path from the root of a drive or of the filesystem (`C:\x`, `/x`, `\\server\x`), not relative to a folder. */
export function isAbsPath(p: string): boolean {
  return /^([A-Za-z]:[\\/]|[\\/])/.test(p);
}

/** Shortens the user's home folder to "~" for display. */
export function tildify(p: string): string {
  const m = p.replace(/\\/g, '/').match(/^[A-Za-z]:\/Users\/[^/]+(\/.*)?$/);
  return m ? '~' + (m[1] ?? '') : p;
}

/** A memory size: "312 Mo", "1,3 Go". */
export function fBytes(n: number): string {
  const MB = 1024 * 1024;
  if (n < 1024 * MB) return `${Math.round(n / MB)} Mo`;
  return `${(n / (1024 * MB)).toLocaleString('fr-FR', { maximumFractionDigits: 1 })} Go`;
}

/** When something happens: "à 15:00" today, "le vendredi 2 octobre à 09:30" another day. */
export function fWhen(ts: number, now: number): string {
  const d = new Date(ts);
  if (d.toDateString() === new Date(now).toDateString()) return `à ${fTime(ts)}`;
  return `le ${d.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long' })} à ${fTime(ts)}`;
}

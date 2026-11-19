// The keys Escouade keeps when the VS Code view has the focus (ESC-464), as VS Code names them in its keybindings: the
// view takes them off VS Code's own, so that its menus and its palette show no key that does something else. The rule
// itself, on the key pressed, is `appWinsInVscode` (`lib/shortcuts.ts`); the spec says it (Éditeur › Les touches).

/**
 * Escouade's keys that are VS Code's too, written as VS Code writes a key (`ctrl+shift+n`, `cmd+p` on macOS), with the
 * primary modifier left out: Ctrl+P (« Ouvrir un fichier »), Ctrl+1…9 (the projects), Ctrl+N, Ctrl+Shift+N, Ctrl+J,
 * Ctrl+T, Ctrl+, (Escouade's settings), Ctrl+Shift+A, Ctrl+Shift+B, Ctrl+\ (the split screen).
 */
const APP_KEYS = new Set(['p', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'n', 'shift+n', 'j', 't', ',', 'shift+a', 'shift+b', '\\']);

/**
 * Whether VS Code's keybinding `label` (`getUserSettingsLabel`, `ctrl+k ctrl+c` for a chord) is one of the keys the
 * app keeps in the view, or `command` one of its zooms (the app zooms the whole window).
 */
export function appTakes(label: string | null | undefined, command: string, mac: boolean): boolean {
  if (command.startsWith('workbench.action.zoom')) return true;
  if (!label || label.includes(' ')) return false;
  const parts = label.toLowerCase().split('+');
  const key = parts.pop() ?? '';
  // The spaces' keys (ESC-539, S3): Alt+1…9 (⌃1…⌃9 on macOS) and Ctrl+² (VS Code's `ctrl+` and the backquote).
  const mods = parts.join('+');
  if (/^[1-9]$/.test(key) && mods === (mac ? 'ctrl' : 'alt')) return true;
  if (key === '`' && mods === 'ctrl') return true;
  const primary = mac ? 'cmd' : 'ctrl';
  if (!parts.includes(primary)) return false;
  const others = parts.filter((p) => p !== primary);
  if (others.some((p) => p !== 'shift')) return false;
  return APP_KEYS.has([...others, key].join('+'));
}

/**
 * The column `col` of a line (1-based, in characters: an emoji is one, as the links of the conversations and
 * « Ouvrir un fichier » count) as VS Code counts it: 0-based, in UTF-16 code units. Past the end, the end.
 */
export function utf16Column(text: string, col: number): number {
  let at = 0;
  let n = 1;
  for (const c of text) {
    if (n >= col) break;
    at += c.length;
    n++;
  }
  return at;
}

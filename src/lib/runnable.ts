// The files « Ouvrir » does not hand to the system: what it would run, install, mount or follow, as the core judges them
// (`sentfiles::RUNNABLE`, which `runnable.test.ts` keeps this list equal to), and the pages a browser would run. The core
// refuses them whatever the window asks; the window offers « Afficher dans le dossier » in their place.

/** `sentfiles::RUNNABLE` of the core, its extensions only. */
export const RUNNABLE = [
  ...['exe', 'com', 'scr', 'pif', 'cpl', 'msi', 'msp', 'msu', 'appinstaller', 'msix', 'msixbundle', 'appx', 'appxbundle'],
  ...['application', 'appref-ms', 'xbap', 'gadget', 'jar', 'jnlp'],
  ...['bat', 'cmd', 'ps1', 'psm1', 'psd1', 'ps1xml', 'vbs', 'vbe', 'js', 'jse', 'ws', 'wsf', 'wsh', 'hta'],
  ...['py', 'pyw', 'pyz', 'pyzw', 'reg'],
  ...['lnk', 'url', 'website', 'scf', 'library-ms', 'search-ms', 'searchconnector-ms', 'settingcontent-ms', 'theme', 'themepack'],
  ...['wsb', 'msc', 'chm', 'hlp', 'diagcab', 'xll', 'mht', 'mhtml'],
  ...['iso', 'img', 'vhd', 'vhdx', 'dmg'],
  ...['app', 'command', 'tool', 'terminal', 'workflow', 'scpt', 'scptd', 'applescript', 'webloc', 'inetloc', 'fileloc'],
  ...['pkg', 'mpkg', 'kext', 'prefpane', 'saver'],
  ...['sh', 'desktop', 'appimage', 'run', 'bin', 'deb', 'rpm', 'snap', 'flatpakref'],
];

/** What a browser would run (`asks::refused_opening`), and scripts the window keeps from « Ouvrir » all the same. */
const MORE = ['html', 'htm', 'xhtml', 'xht', 'shtml', 'svg', 'bash', 'zsh', 'mjs', 'cjs', 'pl', 'rb', 'apk'];

const ALL = new Set([...RUNNABLE, ...MORE]);

/** Whether « Ouvrir » of the file `name` is refused: its extension, whatever its case, its final dots and spaces, its stream. */
export function opensOutside(name: string): boolean {
  const last =
    name
      .trim()
      .replace(/[. /\\]+$/, '')
      .split(/[/\\]/)
      .pop() ?? '';
  return last.split(':').some((part) => {
    const p = part.trim().replace(/[. ]+$/, '');
    const dot = p.lastIndexOf('.');
    return dot >= 0 && ALL.has(p.slice(dot + 1).toLowerCase());
  });
}

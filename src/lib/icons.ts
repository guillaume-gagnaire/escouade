// The icons of the app (lot 1 of the 2.0 design): the paths of the preview's symbols (design/app-redesign/index.html,
// <symbol id="i-…">), drawn at 1.5px on a 16 grid, one family. IconSprite.svelte puts them in the page once, as
// <symbol id="i-<name>">; Icon.svelte (and the rich tooltip, lib/tip.ts) shows one by its name.

export const ICONS = {
  plus: '<path d="M8 3.25v9.5M3.25 8h9.5" />',
  search: '<circle cx="7" cy="7" r="4.25" /><path d="M10.25 10.25l3 3" />',
  overview:
    '<rect x="2.75" y="2.75" width="4" height="4" rx="1" /><rect x="9.25" y="2.75" width="4" height="4" rx="1" /><rect x="2.75" y="9.25" width="4" height="4" rx="1" /><rect x="9.25" y="9.25" width="4" height="4" rx="1" />',
  code: '<path d="M5.5 4.5L2 8l3.5 3.5M10.5 4.5L14 8l-3.5 3.5" />',
  stats: '<path d="M3.25 13V8.75M8 13V3M12.75 13V6" />',
  agents:
    '<path d="M2.75 4.25c0-.83.67-1.5 1.5-1.5h7.5c.83 0 1.5.67 1.5 1.5v5.5c0 .83-.67 1.5-1.5 1.5H7.5l-2.75 2.25v-2.25h-.5c-.83 0-1.5-.67-1.5-1.5z" />',
  kanban:
    '<rect x="2.5" y="2.75" width="3" height="10.5" rx="1" /><rect x="6.5" y="2.75" width="3" height="7" rx="1" /><rect x="10.5" y="2.75" width="3" height="4.5" rx="1" />',
  read: '<path d="M1.75 8S4 3.75 8 3.75 14.25 8 14.25 8 12 12.25 8 12.25 1.75 8 1.75 8z" /><circle cx="8" cy="8" r="1.75" />',
  edit: '<path d="M10.5 2.75l2.75 2.75L6 12.75H3.25V10z" />',
  bash: '<path d="M3 4.5l3.5 3.5L3 11.5M8.5 11.75H13" />',
  check: '<path d="M3.5 8.5l3 3 6-6.5" />',
  'check-c': '<circle cx="8" cy="8" r="5.75" /><path d="M5.6 8.2l1.7 1.7 3.2-3.5" />',
  circle: '<circle cx="8" cy="8" r="5.75" />',
  branch:
    '<circle cx="4.75" cy="3.75" r="1.5" /><circle cx="4.75" cy="12.25" r="1.5" /><circle cx="11.25" cy="5.25" r="1.5" /><path d="M4.75 5.25v5.5M11.25 6.75c0 2.6-2.1 3.4-5.6 4.25" />',
  clip: '<path d="M12.5 7.25l-4.6 4.6a2.75 2.75 0 0 1-3.9-3.9l5-5a1.85 1.85 0 0 1 2.6 2.6l-4.95 4.95a.9.9 0 0 1-1.3-1.3L9.75 4.8" />',
  up: '<path d="M8 12.75V3.5M4 7.25L8 3.25l4 4" />',
  stop: '<rect x="4.5" y="4.5" width="7" height="7" rx="1.5" fill="currentColor" stroke="none" />',
  sliders:
    '<path d="M2.75 5h5.5M11.25 5h2M2.75 11h2M7.75 11h5.5" /><circle cx="9.75" cy="5" r="1.5" /><circle cx="6.25" cy="11" r="1.5" />',
  sound: '<path d="M2.75 6.25h2.25L8.25 3.5v9L5 9.75H2.75z" /><path d="M10.75 5.9a3 3 0 0 1 0 4.2M12.5 4.25a5.4 5.4 0 0 1 0 7.5" />',
  down: '<path d="M4.5 6.25L8 9.75l3.5-3.5" />',
  right: '<path d="M6.25 4.5L9.75 8l-3.5 3.5" />',
  play: '<path d="M5 3.75v8.5L11.75 8z" />',
  folder: '<path d="M2.75 4.25c0-.55.45-1 1-1h2.9l1.4 1.5h4.2c.55 0 1 .45 1 1v6c0 .55-.45 1-1 1H3.75c-.55 0-1-.45-1-1z" />',
  term: '<rect x="2.25" y="3" width="11.5" height="10" rx="2" /><path d="M5 6.5L6.75 8 5 9.5M8.25 9.75H11" />',
  'lay-1': '<rect x="2.25" y="3" width="11.5" height="10" rx="1.75" />',
  'lay-2': '<rect x="2.25" y="3" width="11.5" height="10" rx="1.75" /><path d="M9.25 3v10" />',
  import: '<path d="M8 2.75v7.5M5 7.5l3 3 3-3M3 12.75h10" />',
  merge:
    '<circle cx="4.75" cy="3.75" r="1.5" /><circle cx="4.75" cy="12.25" r="1.5" /><circle cx="11.25" cy="9.25" r="1.5" /><path d="M4.75 5.25v5.5M4.75 5.5c0 2.4 2.4 3.75 5 3.75" />',
  pr: '<circle cx="4.75" cy="3.75" r="1.5" /><circle cx="4.75" cy="12.25" r="1.5" /><circle cx="11.25" cy="12.25" r="1.5" /><path d="M4.75 5.25v5.5M11.25 10.75V6.5c0-1-.75-1.75-1.75-1.75H7.5M9 3.25l-1.5 1.5L9 6.25" />',
  push: '<path d="M8 13V4M4.5 7.25L8 3.75l3.5 3.5" />',
  link: '<path d="M6.5 9.5l3-3M7 4.75l.9-.9a2.5 2.5 0 0 1 3.55 3.55l-.9.9M9 11.25l-.9.9A2.5 2.5 0 0 1 4.55 8.6l.9-.9" />',
  spark: '<path d="M8 2v12M2 8h12" /><path d="M3.8 3.8l8.4 8.4M3.8 12.2l8.4-8.4" stroke-width="1.2" />',
  restart: '<path d="M12.75 8a4.75 4.75 0 1 1-1.4-3.37M12.75 2.75v2.5h-2.5" />',
  shield: '<path d="M8 2.25l4.75 1.75v3.75c0 3-2.1 5.1-4.75 6-2.65-.9-4.75-3-4.75-6V4z" /><path d="M8 5.75v2.75M8 10.75v.01" />',
  ext: '<path d="M9.5 2.75h3.75V6.5M13.25 2.75L7.5 8.5M11.5 9.5v3c0 .41-.34.75-.75.75h-7a.75.75 0 0 1-.75-.75v-7c0-.41.34-.75.75-.75h3" />',
  lang: '<path d="M2.75 4h6.5M6 2.75V4M4.25 4c.4 2.6 2 4.4 4.25 5.25M8 4c-.5 2.8-2.4 5-5.25 6M9 13.25l2.5-6 2.5 6M9.8 11.5h3.4" />',
  user: '<circle cx="8" cy="5.75" r="2.75" /><path d="M3 13.25c.6-2.4 2.6-3.75 5-3.75s4.4 1.35 5 3.75" />',
  /** A ticket shared with the team (ESC-190). */
  team: '<circle cx="6" cy="5.75" r="2.25" /><path d="M2 13c.5-2 2-3.25 4-3.25S9.5 11 10 13M10.5 3.6a2.25 2.25 0 0 1 0 4.3M11.6 9.9c1.3.45 2.1 1.5 2.4 3.1" />',
  bell: '<path d="M4.25 10.75V7.5a3.75 3.75 0 0 1 7.5 0v3.25l1 1.5H3.25zM6.5 13.25a1.6 1.6 0 0 0 3 0" />',
  globe:
    '<circle cx="8" cy="8" r="5.75" /><path d="M2.25 8h11.5M8 2.25c1.6 1.6 2.4 3.5 2.4 5.75S9.6 12.15 8 13.75C6.4 12.15 5.6 10.25 5.6 8S6.4 3.85 8 2.25z" />',
  info: '<circle cx="8" cy="8" r="5.75" /><path d="M8 7.25v3.75M8 5v.01" />',
  trash: '<path d="M3.25 4.5h9.5M6.25 4.5V3.25h3.5V4.5M4.5 4.5l.6 8.25h5.8l.6-8.25" />',
  sparkle: '<path d="M8 2.5l1.3 3.7 3.7 1.3-3.7 1.3L8 12.5l-1.3-3.7L3 7.5l3.7-1.3z" />',
  min: '<path d="M3 8.5h10" />',
  max: '<rect x="3.5" y="3.5" width="9" height="9" rx="0.5" />',
  // The window maximized: the button restores it (not in the preview, which draws the window restored).
  // The commits to pull, beside the branch (the mirror of `up`), and the sound of the notifications turned off.
  pull: '<path d="M8 3.25v9.25M4 8.75l4 4 4-4" />',
  mute: '<path d="M2.75 6.25h2.25L8.25 3.5v9L5 9.75H2.75z" /><path d="M10.75 6.25l3 3.5M13.75 6.25l-3 3.5" />',
  restore: '<path d="M5.5 3.5h7v7" /><rect x="3.5" y="5.5" width="7" height="7" rx="0.5" />',
  close: '<path d="M3.5 3.5l9 9M12.5 3.5l-9 9" />',
  // The editor's files (lot 9, not in the preview, which leaves out the tree's actions): a new file, a new folder,
  // every folder collapsed.
  'file-plus': '<path d="M8.5 2.25H4v11.5h4M8.5 2.25l3.5 3.5M8.5 2.25v3.5H12M12 5.75V8.5M11.75 10.25v3.5M10 12h3.5" />',
  'folder-plus':
    '<path d="M8 12.75H3.75c-.55 0-1-.45-1-1V4.25c0-.55.45-1 1-1h2.9l1.4 1.5h4.2c.55 0 1 .45 1 1V8.5M11.75 10.25v3.5M10 12h3.5" />',
  collapse: '<rect x="5.5" y="5.5" width="8" height="8" rx="1.5" /><path d="M2.75 10.5V4.25c0-.83.67-1.5 1.5-1.5h6.25M7.5 9.5h4" />',
  // The Enter key, in the key of « Envoyer » (not in the preview, which writes it with a character).
  enter: '<path d="M12.5 3.5v4.25a2 2 0 0 1-2 2H4M6.75 6.75L4 9.75l2.75 3" />',
  mic: '<rect x="5.75" y="2.25" width="4.5" height="7.5" rx="2.25" /><path d="M3.5 7.75a4.5 4.5 0 0 0 9 0M8 12.25v1.5" />',
  wave: '<path d="M2.75 7v2M5.25 5v6M7.75 3v10M10.25 5.5v5M12.75 7v2" />',
  image:
    '<rect x="2.25" y="3" width="11.5" height="10" rx="1.75" /><circle cx="6" cy="6.5" r="1.1" /><path d="M2.5 11.25l3.25-3 2.5 2.25 2-1.75 3.25 2.75" />',
  file: '<path d="M4 2.25h5l3 3v8.5H4z" /><path d="M9 2.25v3h3M6 8.5h4M6 10.75h4" />',
  remote: '<circle cx="8" cy="9" r="1.25" /><path d="M5.6 6.6a3.4 3.4 0 0 1 4.8 0M3.7 4.7a6 6 0 0 1 8.6 0M8 10.25v3" />',
  swap: '<path d="M3 5.25h9.5M10 2.75l2.5 2.5L10 7.75M13 10.75H3.5M6 8.25l-2.5 2.5L6 13.25" />',
  sub: '<path d="M4 2.75v5.5c0 1.1.9 2 2 2h6.25M9.75 7.75l2.5 2.5-2.5 2.5" />',
  clock: '<circle cx="8" cy="8" r="5.75" /><path d="M8 5v3.25l2.25 1.5" />',
  download: '<path d="M8 2.75v7.5M5 7.5l3 3 3-3M3 12.75h10" />',
  pause: '<path d="M5.75 4v8M10.25 4v8" />',
  more: '<circle cx="3.75" cy="8" r=".9" fill="currentColor" /><circle cx="8" cy="8" r=".9" fill="currentColor" /><circle cx="12.25" cy="8" r=".9" fill="currentColor" />',
  lock: '<rect x="3.25" y="7" width="9.5" height="6.5" rx="1.5" /><path d="M5.5 7V5.25a2.5 2.5 0 0 1 5 0V7" />',
  plug: '<path d="M6 2.75v3M10 2.75v3M4.25 5.75h7.5v2a3.75 3.75 0 0 1-7.5 0zM8 11.5v2" />',
  // A plugin (PLG-4), as the preview's: the view Plugins and Réglages › Plugins.
  puzzle:
    '<path d="M6 2.75a1.5 1.5 0 0 1 3 0V4h2.25c.41 0 .75.34.75.75V7h1.25a1.5 1.5 0 0 1 0 3H12v2.25c0 .41-.34.75-.75.75H9v-1.25a1.5 1.5 0 0 0-3 0V13H3.75a.75.75 0 0 1-.75-.75V10h1.25a1.5 1.5 0 0 0 0-3H3V4.75c0-.41.34-.75.75-.75H6z" />',
  star: '<path d="M8 2.6l1.6 3.4 3.7.4-2.8 2.5.8 3.7L8 10.7l-3.3 1.9.8-3.7-2.8-2.5 3.7-.4z" />',
  gift: '<path d="M8 3.5v10M3 6.75h10v2.5H3zM4 9.25v4h8v-4M8 6.75C6.75 4.5 4.5 4 4.5 5.4 4.5 6.4 6.25 6.75 8 6.75 9.75 6.75 11.5 6.4 11.5 5.4 11.5 4 9.25 4.5 8 6.75z" />',
  // A workflow (ESC-191): its steps in a cycle.
  flow: '<circle cx="4.5" cy="3.5" r="1.75" /><circle cx="4.5" cy="12.5" r="1.75" /><path d="M4.5 5.25v5.5M6.25 12.5h3.5a2.5 2.5 0 0 0 2.5-2.5V6a2.5 2.5 0 0 0-2.5-2.5h-1.5M9.5 1.75 7.75 3.5l1.75 1.75" />',
  // The two views of the Workflow tab (ESC-245): the steps on a rail, the diagram; then enlarge and copy.
  rail: '<circle cx="4" cy="3.5" r="1.5" /><circle cx="4" cy="12.5" r="1.5" /><path d="M4 5v6M7.5 3.5h6M7.5 12.5h6M7.5 8h4" />',
  diagram:
    '<rect x="5.25" y="1.75" width="5.5" height="3.5" rx="1" /><rect x="1.75" y="10.75" width="5" height="3.5" rx="1" /><rect x="9.25" y="10.75" width="5" height="3.5" rx="1" /><path d="M8 5.25v2.5M4.25 10.75V9.5c0-.7.55-1.25 1.25-1.25h5c.7 0 1.25.55 1.25 1.25v1.25" />',
  expand: '<path d="M9.5 2.75h3.75V6.5M6.5 13.25H2.75V9.5M13.25 2.75 9 7M2.75 13.25 7 9" />',
  copy: '<rect x="5.25" y="5.25" width="8" height="8" rx="1.5" /><path d="M10.75 5.25V3.5c0-.7-.55-1.25-1.25-1.25h-6c-.7 0-1.25.55-1.25 1.25v6c0 .7.55 1.25 1.25 1.25h1.75" />',
} as const;

export type IconName = keyof typeof ICONS;

/** The id of an icon's symbol in the page. */
export const iconId = (name: IconName) => `i-${name}`;

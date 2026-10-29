// The projects' rail (ESC-158, the 2.0 design): a tile per open project, on the left, as Slack's workspaces. What a
// tile says of its project, the project's menu (its tile's right click, its name in the title bar) and its order.

import { openPath } from './opener';
import { t } from './i18n';
import { api } from './ipc';
import type { MenuItem } from './menu.svelte';
import { askCloseProject } from './project-actions';
import { app } from './state.svelte';
import { alertColor, mostPressing } from './tab-strip';
import { projectPalette } from './plugin-theme.svelte';
import type { Project } from './types';

/**
 * The two letters of a tile: the initials of the name's first two words (`demo-api` → « DA »), else its first two
 * letters (`infra` → « In »). A word starts after a dash, an underscore, a space or a dot.
 */
export function initials(name: string): string {
  const words = name.split(/[-_ .]+/).filter(Boolean);
  if (words.length > 1) return (first(words[0]) + first(words[1])).toUpperCase();
  const [a = '', b = ''] = [...(words[0] ?? name)];
  return a.toUpperCase() + b;
}

/** The first letter of a word, an accented or astral one whole. */
const first = (word: string) => [...word][0] ?? '';

/**
 * The most letters « Lettres de la tuile » takes (ESC-167): three still fit in the tile's 36 px, at 11 px; a fourth
 * would touch its edges.
 */
export const MAX_TILE_LETTERS = 3;

const graphemes = (text: string) => [...new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(text)].map((s) => s.segment);
const EMOJI = /\p{Extended_Pictographic}/u;
/** A letter or a digit, with its accents (a keycap too). */
const LETTER = /^[\p{L}\p{N}][\p{M}‍]*$/u;

/**
 * « Lettres de la tuile » as it may be: up to three letters or digits, or one emoji alone; the rest of what is typed
 * is left out (spaces, signs, a fourth letter).
 */
export function cleanLetters(text: string): string {
  const gs = graphemes(text.normalize('NFC')).filter((g) => g.trim());
  if (gs[0] && EMOJI.test(gs[0])) return gs[0];
  return gs
    .filter((g) => LETTER.test(g))
    .slice(0, MAX_TILE_LETTERS)
    .join('');
}

/** What a project's tile says: the letters chosen, else the initials of its name. */
export const tileText = (p: Pick<Project, 'name' | 'tileLetters'>) => p.tileLetters?.trim() || initials(p.name);

/**
 * The most letters « Libellé sous la tuile » takes (ESC-168): twelve still fit whole under the tile of a rail at 104 px,
 * and beside it from 160 px; the name of a project is longer, often, and cut there.
 */
export const MAX_LABEL = 12;

/** « Libellé sous la tuile » as it may be typed: on one line, its spaces one, at most twelve letters (an emoji one). */
export function cleanLabel(text: string): string {
  return graphemes(
    text
      .normalize('NFC')
      .replace(/[\s\p{Cc}]+/gu, ' ')
      .replace(/^ /, ''),
  )
    .slice(0, MAX_LABEL)
    .join('');
}

/** What a project's label says: the text chosen, else its name. */
export const labelText = (p: Pick<Project, 'name' | 'tileLabelText'>) => p.tileLabelText?.trim() || p.name;

/** How the tile's letters are drawn: an emoji bigger, three letters smaller, else as the initials. */
export function tileSize(text: string): 'emoji' | 'three' | null {
  const gs = graphemes(text);
  if (gs.length === 1 && EMOJI.test(gs[0])) return 'emoji';
  return gs.length >= 3 ? 'three' : null;
}

/** What a project's tile says: its agents waiting and at work, its changes, and the blink of an agent to look at. */
export interface TileState {
  waiting: number;
  running: number;
  changes: number;
  /** The colour of the most pressing agent to look at (an error, a question, an end), or null. */
  alert: string | null;
  /** The agents to look at, by name, for the tooltip. */
  attention: string[];
}

export function tileState(p: Project): TileState {
  const agents = Object.values(app.agents).filter((a) => a.projectId === p.id && !a.archived);
  const seen = app.attentionIn(p.id);
  return {
    waiting: agents.filter((a) => a.status === 'waiting').length,
    running: agents.filter((a) => a.status === 'running').length,
    changes: app.git[p.id]?.total ?? 0,
    alert: mostPressing(seen.map((a) => alertColor(a.status))),
    attention: seen.map((a) => a.name),
  };
}

async function save(p: Project) {
  const i = app.projects.findIndex((x) => x.id === p.id);
  if (i >= 0) app.projects[i] = p;
  await app.run(api.updateProject(p));
}

/** The projects in this order, saved. */
export function setOrder(list: Project[]) {
  app.projects = list;
  app.run(api.reorderProjects(list.map((p) => p.id)));
}

/** Moves the project up (-1) or down (+1) the rail; nothing past its ends. True when it moved. */
export function moveProject(id: string, by: -1 | 1): boolean {
  const list = [...app.projects];
  const from = list.findIndex((p) => p.id === id);
  const to = from + by;
  if (from < 0 || to < 0 || to >= list.length) return false;
  [list[from], list[to]] = [list[to], list[from]];
  setOrder(list);
  return true;
}

/** The project dragged to `index` (the place before the tile there; the length: after the last one). */
export function dropProject(id: string, index: number) {
  const list = [...app.projects];
  const from = list.findIndex((p) => p.id === id);
  if (from < 0) return;
  const [moved] = list.splice(from, 1);
  list.splice(index > from ? index - 1 : index, 0, moved);
  if (list.some((p, i) => p.id !== app.projects[i].id)) setOrder(list);
}

/** The project's menu: on its tile (with Monter and Descendre, the keyboard's way to reorder) and on its name above. */
export function projectMenu(p: Project): MenuItem[] {
  const i = app.projects.findIndex((x) => x.id === p.id);
  return [
    {
      label: t('nav.titleBar.menu.rename'),
      icon: 'edit',
      onClick: () => {
        app.modal = {
          kind: 'rename',
          title: t('nav.titleBar.menu.renameTitle'),
          value: p.name,
          onSubmit: (name) => save({ ...p, name }),
        };
      },
    },
    { label: t('common.color'), colors: { values: projectPalette(p.color), selected: p.color, onPick: (color) => save({ ...p, color }) } },
    { label: '', separator: true },
    {
      label: p.worktreePerAgent ? t('nav.titleBar.menu.worktreeOff') : t('nav.titleBar.menu.worktreeOn'),
      icon: 'branch',
      hint: t('nav.titleBar.menu.worktreeHint'),
      onClick: () => save({ ...p, worktreePerAgent: !p.worktreePerAgent }),
    },
    {
      label: t('nav.titleBar.menu.settings'),
      icon: 'sliders',
      onClick: () => {
        app.modal = { kind: 'settings', tab: 'projects', projectId: p.id };
      },
    },
    {
      label: t('nav.titleBar.menu.openFolder'),
      icon: 'folder',
      onClick: () => openPath(p.path).catch((err) => app.toast(String(err), 'error')),
    },
    { label: '', separator: true },
    { label: t('nav.rail.up'), icon: 'up', hint: t('nav.rail.upKey'), disabled: i <= 0, onClick: () => moveProject(p.id, -1) },
    {
      label: t('nav.rail.down'),
      icon: 'down',
      hint: t('nav.rail.downKey'),
      disabled: i < 0 || i >= app.projects.length - 1,
      onClick: () => moveProject(p.id, 1),
    },
    { label: '', separator: true },
    {
      label: t('nav.titleBar.menu.close'),
      icon: 'close',
      danger: true,
      detail: t('nav.rail.closeDetail'),
      onClick: () => askCloseProject(p),
    },
  ];
}

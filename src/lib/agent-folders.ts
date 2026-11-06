// Where an agent or a folder lands in the sidebar when dragged or moved with the keyboard: the `Place` the core's
// `move_item` takes (`api.agentFolderMove`). Pure: the window only draws the drop line and sends the place.

import type { SidebarItem } from './sidebar';
import type { Project } from './types';

/** `folderId`: the target folder (null = project level). `index`: the rank in that level's stored list once the item is taken out of it; null = bottom. */
export type Place = { folderId: string | null; index: number | null };

/** The stored lists, as the core settles them: the project level (ids of loose agents and folders), and each folder's members. */
function stored(project: Project, items: SidebarItem[]) {
  const folders = project.agentFolders ?? [];
  const members = new Set(folders.flatMap((f) => f.agents));
  const top: string[] = [];
  const add = (id: string) => !top.includes(id) && !members.has(id) && top.push(id);
  (project.agentOrder ?? []).forEach(add);
  folders.forEach((f) => add(f.id));
  items
    .flatMap((it) => (it.kind === 'agent' ? [it.agent] : []))
    .sort((a, b) => a.createdAt - b.createdAt)
    .forEach((a) => add(a.id));
  return (folderId: string | null) => (folderId ? [...new Set(folders.find((f) => f.id === folderId)?.agents ?? [])] : top);
}

/** A level as shown: ids in shown order and which are lifted. A folder's principal is on top, its waiting agents come next (lifted). */
function shown(items: SidebarItem[], level: string | null): { id: string; lifted: boolean }[] {
  if (!level)
    return items.map((it) => (it.kind === 'agent' ? { id: it.agent.id, lifted: it.lifted } : { id: it.folder.id, lifted: false }));
  const f = items.find((it) => it.kind === 'folder' && it.folder.id === level);
  if (f?.kind !== 'folder') return [];
  return [
    ...(f.principal ? [{ id: f.principal.id, lifted: false }] : []),
    ...f.agents.map((a) => ({ id: a.id, lifted: a.status === 'waiting' })),
  ];
}

/** The level `id` is in now (a folder's id, null = project level). */
function levelOf(project: Project, id: string): string | null {
  return (project.agentFolders ?? []).find((f) => f.agents.includes(id))?.id ?? null;
}

/**
 * Where `dragged` lands when dropped in level `level` (null = project level). `beforeShown`, chosen by the UI: the id of
 * the shown item (an agent card or a folder header) of that level the drop line sits before, or null for the line at the
 * end of the level. The neighbour is the first item at or after it in shown order that is not lifted (a loose agent that
 * waits, a waiting member under its folder's principal: they sit on top whatever their stored rank) and is not `dragged`;
 * the index is its rank in the stored list without `dragged`, none → bottom. Null: a folder into a folder, or no change.
 */
export function dropPlace(
  project: Project,
  items: SidebarItem[],
  dragged: string,
  level: string | null,
  beforeShown: string | null,
): Place | null {
  if (level && items.some((it) => it.kind === 'folder' && it.folder.id === dragged)) return null;
  const list = stored(project, items)(level).filter((i) => i !== dragged);
  const row = shown(items, level);
  const from = beforeShown === null ? -1 : row.findIndex((r) => r.id === beforeShown);
  const next = from < 0 ? undefined : row.slice(from).find((r) => !r.lifted && r.id !== dragged);
  const rank = next ? list.indexOf(next.id) : -1;
  const index = rank < 0 ? null : rank;
  const here = stored(project, items)(levelOf(project, dragged)).indexOf(dragged);
  if (levelOf(project, dragged) === level && (index ?? list.length) === here) return null;
  return { folderId: level, index };
}

/**
 * Where `item` lands moved one step (`by`) on the stored order; null at the ends. An agent at the edge of a folder leaves
 * it, above or below the folder; a loose agent arriving on a folder enters it (at the top going down, at the bottom going
 * up); a folder steps over the whole neighbour. Inside a folder the principal is always shown first whatever its stored
 * rank, so it is taken as sitting at rank 0, and an agent never steps above it (it leaves the folder instead).
 */
export function stepPlace(project: Project, items: SidebarItem[], item: string, by: -1 | 1): Place | null {
  const of = stored(project, items);
  const seen = new Set(
    items.flatMap((it) =>
      it.kind === 'agent' ? [it.agent.id] : [it.folder.id, ...it.agents.map((a) => a.id), ...(it.principal ? [it.principal.id] : [])],
    ),
  );
  const level = levelOf(project, item);
  const folder = (project.agentFolders ?? []).find((f) => f.id === level);
  let row = of(level).filter((i) => seen.has(i));
  if (folder?.principal && row.includes(folder.principal)) row = [folder.principal, ...row.filter((i) => i !== folder.principal)];
  const at = row.indexOf(item);
  if (at < 0) return null;
  const k = at + by;
  const rest = row.filter((i) => i !== item);
  const rank = (level: string | null, id: string | undefined) => {
    const r =
      id === undefined
        ? -1
        : of(level)
            .filter((i) => i !== item)
            .indexOf(id);
    return r < 0 ? null : r;
  };
  const floor = folder?.principal && item !== folder.principal ? 1 : 0;
  if (level && (k < floor || k >= row.length)) {
    // Out of the folder: next to it at the project level.
    const top = of(null).filter((i) => seen.has(i));
    const f = top.indexOf(level);
    return { folderId: null, index: rank(null, by < 0 ? level : top[f + 1]) };
  }
  if (level) return { folderId: level, index: rank(level, rest[k]) };
  if (k < 0 || k >= row.length) return null;
  const n = row[k];
  const isFolder = (project.agentFolders ?? []).some((f) => f.id === n);
  const mine = (project.agentFolders ?? []).some((f) => f.id === item);
  if (isFolder && !mine) return { folderId: n, index: by > 0 ? 0 : null };
  return { folderId: null, index: rank(null, by < 0 ? n : row[k + 1]) };
}

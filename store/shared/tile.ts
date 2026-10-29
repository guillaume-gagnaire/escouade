// A plugin's tile (design/store/, `.tile`): its two letters on a hue drawn from its id. Shared by the server's lists
// (server/utils/listing.ts) and the author's account, which draws the tiles of plugins it reads by id.

/** The hue of a tile, drawn from the id: one of twelve, as the avatars. */
export function hueOf(id: string): number {
  let h = 0;
  for (const c of id) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return (h % 12) * 30;
}

/** The two letters of a tile: the name's first two. */
export const markOf = (name: string) => [...name.replace(/^[^\p{L}\p{N}]+/u, '')].slice(0, 2).join('') || '··';

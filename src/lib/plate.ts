// The plate of a space (ESC-537, S6), on its own: SpacePlate.svelte shows it on the desktop and on the phone (MO-1,
// apps/mobile), which takes it without the desktop's state.

/** The space's plate: its first letter (a screen stands for this computer). */
export function plateOf(name: string): string {
  const letter = [...name.trim()].find((c) => /\p{L}|\p{N}/u.test(c));
  return (letter ?? '?').toLocaleUpperCase();
}

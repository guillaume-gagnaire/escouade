import type { Tree } from '../types';

// The errors the backend sends as codes, written here.
export default {
  /** `NOT_FOUND:<path>`: a file of the editor that is not (or no longer) on disk. */
  notFound: '{path} introuvable',
} as const satisfies Tree;

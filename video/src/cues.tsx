import { createContext, useContext } from 'react';
import { useLang } from './lang';
import { FPS, sceneOf, type Placed, type SceneId } from './timeline';

/** The scene being drawn: its animations follow its lines. */
export const SceneContext = createContext<SceneId | null>(null);

export function useScene(): Placed {
  const id = useContext(SceneContext);
  if (!id) throw new Error('Un plan se dessine dans un SceneContext');
  return sceneOf(id);
}

const norm = (w: string) =>
  w
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '');

/** Frames from the scene's start of a line's start, its end, and the moment the voice says one of its words. */
export interface Cues {
  /** When a line starts. */
  at: (line: string) => number;
  /** When a line ends. */
  end: (line: string) => number;
  /** When the voice starts the `nth` time it says `word` in a line (accents, case and punctuation aside). */
  word: (line: string, word: string, nth?: number) => number;
  /** The scene's length. */
  length: number;
}

export function cuesOf(s: Placed): Cues {
  const line = (id: string) => {
    const l = s.lines.find((x) => x.id === id);
    if (!l) throw new Error(`Pas de réplique « ${id} » dans ${s.id}`);
    return l;
  };
  return {
    at: (id) => s.cues[line(id).id],
    end: (id) => s.cues[id] + line(id).durationInFrames,
    word: (id, word, nth = 0) => {
      const l = line(id);
      // A whole word (« mode » is not « modèle »); « d'effort » is said as « effort »: the word after its elision counts too.
      const says = (w: string) => [w, w.split(/['’]/).pop()!].some((part) => norm(part) === norm(word));
      const found = l.words.filter((w) => says(w.word))[nth];
      if (!found) throw new Error(`« ${word} » n'est pas dit dans ${s.id}.${id}`);
      return s.cues[id] + Math.round(found.start * FPS);
    },
    length: s.durationInFrames,
  };
}

export const useCues = (): Cues => cuesOf(useScene());

/** The scene's title, on screen, in the language of the picture. */
export function useTitle(): string {
  const scene = useScene();
  return useLang() === 'en' ? scene.titleEn : scene.title;
}

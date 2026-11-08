// What a view drawn in a pane of the split screen (ESC-392) knows of its pane: whether it is the active one, which
// alone takes the focus (a terminal mounted, a composer asked to focus). Out of any pane (the content shown alone):
// none, and the view acts as before.

import { getContext, setContext } from 'svelte';

export interface PaneCtx {
  readonly key: string;
  readonly active: boolean;
}

const KEY = Symbol('pane');

export const setPaneCtx = (ctx: PaneCtx) => setContext(KEY, ctx);

export const paneCtx = (): PaneCtx | undefined => getContext<PaneCtx | undefined>(KEY);

/** The view may take the focus: shown alone, or in the active pane. */
export const mayFocus = (ctx: PaneCtx | undefined) => !ctx || ctx.active;

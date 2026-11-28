// The tabs and the system's back (MO-1, MO-5): one step at a time, the sheet, the conversation, the line unfolded, then
// « À toi », then out of the app.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { waitFor } from '@testing-library/svelte';
import { back, closeSheet, followHistory, goTab, nav, openSheet, openTalk, tabOf, unfold } from './nav.svelte';

describe('the system’s back', () => {
  beforeEach(() => {
    nav.tab = 'inbox';
    nav.sheet = null;
    nav.talk = null;
    nav.open = null;
  });

  it('closes the sheet, then brings « À toi » back, then leaves', () => {
    goTab('kanban');
    openSheet('create');
    expect(back()).toBe(true);
    expect(nav).toEqual({ tab: 'kanban', sheet: null, talk: null, open: null });
    expect(back()).toBe(true);
    expect(nav.tab).toBe('inbox');
    expect(back()).toBe(false);
  });

  it('opens the tab the page’s address names, none else', () => {
    expect(tabOf('#kanban')).toBe('kanban');
    expect(tabOf('#agents')).toBe('agents');
    expect(tabOf('')).toBeNull();
    expect(tabOf('#reglages')).toBeNull();
  });

  it('a tab chosen closes the sheet', () => {
    openSheet('create');
    goTab('agents');
    expect(nav).toEqual({ tab: 'agents', sheet: null, talk: null, open: null });
    closeSheet();
    expect(nav.sheet).toBeNull();
  });

  it('closes the viewer above a conversation, then the conversation, then folds the line, then leaves (MO-5)', () => {
    unfold('here:ask:1');
    openTalk('here', 'a1');
    openSheet('viewer');
    expect(back()).toBe(true);
    expect(nav).toMatchObject({ sheet: null, talk: { space: 'here', agent: 'a1' }, open: 'here:ask:1' });
    expect(back()).toBe(true);
    expect(nav.talk).toBeNull();
    expect(nav.open).toBe('here:ask:1');
    expect(back()).toBe(true);
    expect(nav.open).toBeNull();
    expect(back()).toBe(false);
  });

  it('a tab chosen closes the conversation', () => {
    openTalk('here', 'a1');
    goTab('kanban');
    expect(nav.talk).toBeNull();
  });
});

describe('a browser’s back, through its history', () => {
  let stop: () => void;
  beforeEach(() => {
    nav.tab = 'inbox';
    nav.sheet = null;
    nav.talk = null;
    nav.open = null;
    stop = followHistory(window);
  });
  afterEach(() => stop());

  it('is a step, and the entry above the page is put back while the app stays', async () => {
    const depth = history.length;
    goTab('agents');
    openSheet('create');
    history.back();
    await waitFor(() => expect(nav.sheet).toBeNull());
    await waitFor(() => expect(history.state?.escouade).toBe('back'));
    history.back();
    await waitFor(() => expect(nav.tab).toBe('inbox'));
    await waitFor(() => expect(history.state?.escouade).toBe('back'));
    expect(history.length).toBe(depth);
  });
});

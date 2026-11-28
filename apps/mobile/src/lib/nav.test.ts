// The tabs and the system's back (MO-1): one step at a time, the sheet, then « À toi », then out of the app.
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { waitFor } from '@testing-library/svelte';
import { back, closeSheet, followHistory, goTab, nav, openSheet, tabOf } from './nav.svelte';

describe('the system’s back', () => {
  beforeEach(() => {
    nav.tab = 'inbox';
    nav.sheet = null;
  });

  it('closes the sheet, then brings « À toi » back, then leaves', () => {
    goTab('kanban');
    openSheet('create');
    expect(back()).toBe(true);
    expect(nav).toEqual({ tab: 'kanban', sheet: null });
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
    expect(nav).toEqual({ tab: 'agents', sheet: null });
    closeSheet();
    expect(nav.sheet).toBeNull();
  });
});

describe('a browser’s back, through its history', () => {
  let stop: () => void;
  beforeEach(() => {
    nav.tab = 'inbox';
    nav.sheet = null;
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

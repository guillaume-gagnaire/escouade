import { describe, expect, it } from 'vitest';
import { dropPlace, stepPlace } from './agent-folders';
import { sidebarItems } from './sidebar';
import { agent, project } from '../test/ipc';
import type { Agent, AgentFolder } from './types';

const ag = (id: string, over: Partial<Agent> = {}) => agent({ id, name: id, createdAt: Number(id.replace(/\D/g, '')) || 1, ...over });
const folder = (id: string, agents: string[], principal: string | null = null): AgentFolder => ({
  id,
  name: id,
  named: true,
  principal,
  agents,
});
function setup(agents: Agent[], agentFolders: AgentFolder[], agentOrder: string[]) {
  const p = project({ agentFolders, agentOrder });
  return { p, items: sidebarItems(p, agents) };
}

describe('dropPlace', () => {
  it('a waiting agent dropped keeps its lift', () => {
    // Shown: w (lifted, waiting), a, b. Dropping c before w lands before a, the first one not lifted.
    const { p, items } = setup([ag('w', { status: 'waiting' }), ag('a'), ag('b'), ag('c')], [], ['a', 'w', 'b', 'c']);
    expect(dropPlace(p, items, 'c', null, 'w')).toEqual({ folderId: null, index: 0 });
  });
  it('a folder never drops into a folder', () => {
    const { p, items } = setup([ag('a'), ag('b')], [folder('f', ['a']), folder('g', ['b'])], ['f', 'g']);
    expect(dropPlace(p, items, 'g', 'f', 'a')).toBeNull();
  });
  it('dropping where it already is does nothing', () => {
    const { p, items } = setup([ag('a'), ag('b'), ag('c')], [], ['a', 'b', 'c']);
    expect(dropPlace(p, items, 'b', null, 'c')).toBeNull();
    expect(dropPlace(p, items, 'b', null, 'b')).toBeNull();
    expect(dropPlace(p, items, 'c', null, null)).toBeNull();
    expect(dropPlace(p, items, 'a', null, null)).toEqual({ folderId: null, index: null });
  });
  it('drops into a folder before a member', () => {
    const { p, items } = setup([ag('a'), ag('b'), ag('c')], [folder('f', ['b', 'c'], 'b')], ['a', 'f']);
    expect(dropPlace(p, items, 'a', 'f', 'c')).toEqual({ folderId: 'f', index: 1 });
    expect(dropPlace(p, items, 'a', 'f', null)).toEqual({ folderId: 'f', index: null });
  });
});

describe('stepPlace', () => {
  it('Alt+Down at the bottom of a folder takes the agent out below it', () => {
    const { p, items } = setup([ag('a'), ag('b'), ag('c')], [folder('f', ['a', 'b'])], ['f', 'c']);
    expect(stepPlace(p, items, 'b', 1)).toEqual({ folderId: null, index: 1 });
    expect(stepPlace(p, items, 'a', -1)).toEqual({ folderId: null, index: 0 });
  });
  it('Alt+Down onto a folder enters it at the top', () => {
    const { p, items } = setup([ag('a'), ag('b')], [folder('f', ['b'])], ['a', 'f']);
    expect(stepPlace(p, items, 'a', 1)).toEqual({ folderId: 'f', index: 0 });
  });
  it('Alt+Up onto a folder enters it at the bottom', () => {
    const { p, items } = setup([ag('a'), ag('b')], [folder('f', ['b'])], ['f', 'a']);
    expect(stepPlace(p, items, 'a', -1)).toEqual({ folderId: 'f', index: null });
  });
  it('a folder steps over a whole folder', () => {
    const { p, items } = setup([ag('a'), ag('b'), ag('c')], [folder('f', ['a']), folder('g', ['b'])], ['f', 'g', 'c']);
    expect(stepPlace(p, items, 'f', 1)).toEqual({ folderId: null, index: 1 });
    expect(stepPlace(p, items, 'g', -1)).toEqual({ folderId: null, index: 0 });
    expect(stepPlace(p, items, 'f', -1)).toBeNull();
    expect(stepPlace(p, items, 'c', 1)).toBeNull();
  });
  it('steps inside a folder', () => {
    const { p, items } = setup([ag('a'), ag('b'), ag('c')], [folder('f', ['a', 'b', 'c'])], ['f']);
    expect(stepPlace(p, items, 'a', 1)).toEqual({ folderId: 'f', index: 1 });
    expect(stepPlace(p, items, 'b', 1)).toEqual({ folderId: 'f', index: null });
  });
});

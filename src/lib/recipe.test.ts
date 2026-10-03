import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { openAddress, parseTestId, recipeCommands, testCommand, testId } from './recipe';

const a = agent({
  id: 'a1',
  recipe: {
    prepare: [{ command: 'npm i', dir: 'web' }],
    processes: [
      { name: 'web', command: 'npm run dev', dir: 'web', env: {}, url: 'http://localhost:4101' },
      { name: '', command: 'node api.js', dir: '', env: {}, url: '' },
    ],
    open: '',
  },
});

describe('recipe', () => {
  it('names each step of the recipe as a launch command', () => {
    expect(recipeCommands(a, 'pwsh')).toEqual({
      prepare: [{ id: 'test:a1:prep:0', name: 'Préparation 1', command: 'npm i', shell: 'pwsh', cwd: 'web' }],
      processes: [
        { id: 'test:a1:run:0', name: 'web', command: 'npm run dev', shell: 'pwsh', cwd: 'web' },
        { id: 'test:a1:run:1', name: 'processus 2', command: 'node api.js', shell: 'pwsh', cwd: '' },
      ],
    });
    expect(parseTestId('test:a1:run:1')).toEqual({ agentId: 'a1', kind: 'run', index: 1 });
    expect(parseTestId('c1')).toBeNull();
    expect(testCommand('test:a1:prep:0', { a1: a }, 'pwsh')?.command).toBe('npm i');
    expect(testCommand('test:a1:run:5', { a1: a }, 'pwsh')).toBeNull();
    expect(recipeCommands(agent(), 'pwsh')).toEqual({ prepare: [], processes: [] });
  });

  it('reads back the id it builds, and nothing that is not one', () => {
    expect(parseTestId(testId('a1', 'prep', 12))).toEqual({ agentId: 'a1', kind: 'prep', index: 12 });
    expect(parseTestId('test:a1:other:0')).toBeNull();
    expect(parseTestId('test:a1:run:x')).toBeNull();
    expect(parseTestId('xtest:a1:run:0')).toBeNull();
    // An agent that is gone, or has no recipe any more, leaves no command.
    expect(testCommand('test:a2:run:0', { a1: a }, 'pwsh')).toBeNull();
    expect(testCommand('test:a1:run:0', { a1: agent({ id: 'a1' }) }, 'pwsh')).toBeNull();
    expect(testCommand('c1', { a1: a }, 'pwsh')).toBeNull();
  });

  it('opens the address of the feature, else the first process’s', () => {
    expect(openAddress(a)).toBe('http://localhost:4101');
    expect(openAddress(agent({ recipe: { ...a.recipe!, open: 'http://localhost:4101/connexion' } }))).toBe(
      'http://localhost:4101/connexion',
    );
    expect(openAddress(agent())).toBeNull();
    // A blank address counts for none: the first process that has one is taken.
    const spaces = agent({
      recipe: { prepare: [], processes: [{ ...a.recipe!.processes[1] }, { ...a.recipe!.processes[0] }], open: '  ' },
    });
    expect(openAddress(spaces)).toBe('http://localhost:4101');
    expect(openAddress(agent({ recipe: { prepare: [], processes: [{ ...a.recipe!.processes[1] }], open: '' } }))).toBeNull();
  });
});

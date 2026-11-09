import { describe, expect, it } from 'vitest';
import { interactiveCommands, interactiveOf } from './interactive';

describe('interactiveOf (ESC-401)', () => {
  it('sends /login and /logout to the agent’s account, whatever the harness', () => {
    for (const engine of ['claude', 'codex', 'copilot', 'cursor', 'opencode', null]) {
      expect(interactiveOf(engine, '/login')).toEqual({ kind: 'login' });
      expect(interactiveOf(engine, '  /logout ')).toEqual({ kind: 'logout' });
    }
  });

  it('opens the composer’s menu for /model and /effort alone, and lets their process take them with an argument', () => {
    expect(interactiveOf('claude', '/model')).toEqual({ kind: 'menu', menu: 'model' });
    expect(interactiveOf('codex', '/effort')).toEqual({ kind: 'menu', menu: 'effort' });
    expect(interactiveOf('claude', '/model opus')).toBeNull();
    expect(interactiveOf('claude', '/effort high')).toBeNull();
  });

  it('sends the commands Escouade has no screen for to the Claude Code view, as typed', () => {
    expect(interactiveOf('claude', '/hooks')).toEqual({ kind: 'view', command: '/hooks' });
    expect(interactiveOf('claude', '/permissions')).toEqual({ kind: 'view', command: '/permissions' });
    expect(interactiveOf('claude', '/add-dir ../lib')).toEqual({ kind: 'view', command: '/add-dir ../lib' });
    // Alone only: `/config key=value` and `/mcp reconnect` run in the stream.
    expect(interactiveOf('claude', '/config')).toEqual({ kind: 'view', command: '/config' });
    expect(interactiveOf('claude', '/config theme=dark')).toBeNull();
    expect(interactiveOf('claude', '/mcp reconnect')).toBeNull();
  });

  it('opens Escouade’s own gesture where it has one', () => {
    expect(interactiveOf('claude', '/usage')).toEqual({ kind: 'settings', page: 'accounts' });
    expect(interactiveOf('claude', '/remote-control')).toEqual({ kind: 'remote' });
    expect(interactiveOf('claude', '/plan')).toEqual({ kind: 'plan', rest: '' });
    expect(interactiveOf('claude', '/plan refais le cache')).toEqual({ kind: 'plan', rest: 'refais le cache' });
  });

  it('keeps the agent’s session: /resume, /branch, /fork, /teleport and /exit are never sent nor opened', () => {
    for (const c of ['/resume', '/branch', '/fork', '/teleport'])
      expect(interactiveOf('claude', c)).toEqual({ kind: 'kept', why: 'session' });
    expect(interactiveOf('claude', '/exit')).toEqual({ kind: 'kept', why: 'exit' });
  });

  it('leaves alone what the stream runs, the skills, text and the other harnesses’ own commands', () => {
    for (const c of ['/compact', '/clear', '/context', '/cost', '/commit', '/gsd:next 3', 'Relis /hooks', 'bonjour'])
      expect(interactiveOf('claude', c)).toBeNull();
    // Not tried there one by one: Codex's own /status goes as it did.
    expect(interactiveOf('codex', '/status')).toBeNull();
    expect(interactiveOf('codex', '/hooks')).toBeNull();
  });

  it('lists the commands for the « / » menu with where each opens', () => {
    const claude = interactiveCommands('claude');
    expect(claude).toContainEqual({ name: 'login', where: 'account' });
    expect(claude).toContainEqual({ name: 'model', where: 'menu' });
    expect(claude).toContainEqual({ name: 'hooks', where: 'view' });
    expect(claude).toContainEqual({ name: 'resume', where: 'kept' });
    expect(interactiveCommands('codex').map((c) => c.name)).toEqual(['login', 'logout', 'model', 'effort']);
  });
});

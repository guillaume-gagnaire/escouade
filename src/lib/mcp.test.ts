import { beforeEach, describe, expect, it } from 'vitest';
import { fakeBackend } from '../test/ipc';
import { maskToken, mcp } from './mcp.svelte';
import type { McpActivityEntry, McpDeclaration } from './types';

const entry = (n: number, over: Partial<McpActivityEntry> = {}): McpActivityEntry => ({
  at: n,
  caller: 'Claude (hors Escouade)',
  tool: 'list_projects',
  summary: `call ${n}`,
  outcome: 'ok',
  message: null,
  ...over,
});

const declaration = (account: string, over: Partial<McpDeclaration> = {}): McpDeclaration => ({
  account,
  ok: true,
  error: null,
  command: 'claude mcp add',
  ...over,
});

describe('the MCP server in the window', () => {
  beforeEach(() => mcp.reset());

  it('hides the token of a command, wherever the shell has it', () => {
    const token = 'k3Zr9-Q_token0123456789';
    const add = (end: string) => `claude mcp add --header "Authorization: Bearer ${token}"${end}`;
    expect(maskToken(add(''))).toBe('claude mcp add --header "Authorization: Bearer ••••••••"');
    expect(maskToken(add('; $env:CLAUDE_CONFIG_DIR = $old'))).toBe(
      'claude mcp add --header "Authorization: Bearer ••••••••"; $env:CLAUDE_CONFIG_DIR = $old',
    );
    expect(maskToken(`CLAUDE_CONFIG_DIR='/p' claude mcp add --header 'Authorization: Bearer ${token}'`)).toBe(
      "CLAUDE_CONFIG_DIR='/p' claude mcp add --header 'Authorization: Bearer ••••••••'",
    );
    expect(maskToken('claude mcp remove escouade --scope user')).toBe('claude mcp remove escouade --scope user');
  });

  it('takes the status, the declarations and each call as the backend sends them', () => {
    mcp.take({ type: 'mcpStatus', status: { running: true, port: 47123, error: null } });
    mcp.take({ type: 'mcpDeclared', declared: [declaration('principal')] });
    mcp.take({ type: 'mcpActivity', entry: entry(1) });
    mcp.take({ type: 'mcpActivity', entry: entry(2) });
    expect(mcp.status).toEqual({ running: true, port: 47123, error: null });
    expect(mcp.declared.map((d) => d.account)).toEqual(['principal']);
    expect(mcp.activity.map((e) => e.at)).toEqual([1, 2]);
  });

  it('keeps the last 200 calls, as the backend does', () => {
    for (let n = 1; n <= 205; n++) mcp.take({ type: 'mcpActivity', entry: entry(n) });
    expect(mcp.activity).toHaveLength(200);
    expect(mcp.activity[0].at).toBe(6);
    expect(mcp.activity.at(-1)?.at).toBe(205);
  });

  it('reads what the backend has, and keeps a call it had been told of that the reading does not hold yet', async () => {
    mcp.take({ type: 'mcpActivity', entry: entry(2) });
    mcp.take({ type: 'mcpActivity', entry: entry(5) });
    fakeBackend({
      mcp_status: () => ({ running: true, port: 47123, error: null }),
      mcp_declare_status: () => [declaration('principal'), declaration('pro')],
      // Call 2 is in both; call 5 happened after the backend answered.
      mcp_activity: () => [entry(1), entry(2), entry(3)],
    });
    await mcp.load();
    expect(mcp.activity.map((e) => e.at)).toEqual([1, 2, 3, 5]);
    expect(mcp.status.running).toBe(true);
    expect(mcp.declared.map((d) => d.account)).toEqual(['principal', 'pro']);
  });

  it('prefers what an event brought while it was reading: that is newer', async () => {
    let answer: (v: unknown) => void = () => {};
    const pending = new Promise((r) => (answer = r));
    fakeBackend({
      mcp_status: () => pending,
      mcp_declare_status: () => [declaration('principal')],
      mcp_activity: () => [],
    });
    const loading = mcp.load();
    mcp.take({ type: 'mcpStatus', status: { running: false, port: 47200, error: 'busy' } });
    answer({ running: true, port: 47123, error: null });
    await loading;
    expect(mcp.status).toEqual({ running: false, port: 47200, error: 'busy' });
    // What it read of the declarations, none having come meanwhile, stands.
    expect(mcp.declared.map((d) => d.account)).toEqual(['principal']);
  });

  it('forgets the calls in the window and in the backend', async () => {
    const be = fakeBackend({ mcp_clear_activity: () => null });
    mcp.take({ type: 'mcpActivity', entry: entry(1) });
    await mcp.clear();
    expect(mcp.activity).toEqual([]);
    expect(be.called('mcp_clear_activity')).toHaveLength(1);
  });
});

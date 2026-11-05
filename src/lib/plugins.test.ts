import { describe, expect, it } from 'vitest';
import { askOf, sourceUrl } from './plugins';

// Where a store plugin comes from (ESC-313): its repository, or its folder at its tag when it is in one.
describe('the source of a store plugin', () => {
  it('links the folder at the tag, and the repository for a plugin at its root or an old version.json', () => {
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'slack@1.0.0', path: 'plugins/slack' })).toBe(
      'https://github.com/o/r/tree/slack@1.0.0/plugins/slack',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r/', tag: 'plugins/x/v1.0.0', path: 'plugins/x' })).toBe(
      'https://github.com/o/r/tree/plugins/x/v1.0.0/plugins/x',
    );
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0', path: '' })).toBe('https://github.com/o/r');
    expect(sourceUrl({ repository: 'https://github.com/o/r', tag: 'v1.0.0' })).toBe('https://github.com/o/r');
  });
});

describe('the network a plugin asks for', () => {
  const inWasm = { run: { js: 'dist/plugin.mjs' } } as never;

  it('names a host, what Escouade applies under it', () => {
    const ask = askOf('network:api.exemple.dev', inWasm, 'fr');
    expect(ask.detail).toBe('api.exemple.dev');
    expect(ask.note).toBeTruthy();
  });

  it('says the instances the user gives, and the local server of its harness, without a host to name', () => {
    const account = askOf('network:{account}', inWasm, 'fr');
    expect(account.text).toBe('Se connecte aux instances que tu lui donnes');
    expect(account.detail).toBe('');
    const shared = askOf('network:local:shared', inWasm, 'fr');
    expect(shared.text).toBe('Parle au serveur local du harness, lancé par Escouade');
    expect(shared.detail).toBe('');
  });
});

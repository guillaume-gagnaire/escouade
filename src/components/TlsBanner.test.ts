import { cleanup, render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { app } from '../lib/state.svelte';
import { fakeBackend, resetApp } from '../test/ipc';
import TlsBanner from './TlsBanner.svelte';

describe('TlsBanner (SEC-07)', () => {
  beforeEach(() => {
    resetApp();
    fakeBackend();
  });
  afterEach(cleanup);

  it('is absent while the certificates are checked', () => {
    render(TlsBanner);
    expect(screen.queryByTestId('tls-band')).toBeNull();
  });

  it('stays while the check is skipped, names the proxy without its password, and leads to the file of authorities', async () => {
    app.settings.insecureTls = true;
    app.settings.proxyUrl = 'http://ada:secret@127.0.0.1:9000';
    render(TlsBanner);
    const band = screen.getByTestId('tls-band');
    expect(band).toHaveTextContent('Certificats TLS non vérifiés par le proxy 127.0.0.1:9000');
    expect(band).not.toHaveTextContent('secret');
    await userEvent.click(screen.getByRole('button', { name: 'Choisir le certificat racine' }));
    expect(app.modal).toEqual({ kind: 'settings', page: 'advanced', at: 'network' });
  });

  it('says the setting does nothing with no proxy', () => {
    app.settings.insecureTls = true;
    app.settings.proxyUrl = '';
    render(TlsBanner);
    expect(screen.getByTestId('tls-band')).toHaveTextContent('sans proxy : elle n’a pas d’effet');
  });
});

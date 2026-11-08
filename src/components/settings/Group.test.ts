import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import Host from './Group.host.test.svelte';

describe('Group in « Avancé »', () => {
  afterEach(() => setLang('fr'));

  it('folds the rare settings at the foot of a page, naming those of the register it hides', async () => {
    render(Host, { advanced: 'accounts' });
    const fold = screen.getByRole('button', { name: /^Avancé/ });
    expect(fold).toHaveAttribute('aria-expanded', 'false');
    expect(fold).toHaveTextContent(
      "AvancéOrdre des harness · Modèle de passation · Chemin de l'exécutable · Mettre à jour Claude Code automatiquement · Projection des quotas",
    );
    expect(screen.queryByText('la rangée cachée')).toBeNull();
    await userEvent.click(fold);
    expect(fold).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByText('la rangée cachée')).toBeInTheDocument();
    expect(screen.getByText('Ce qui sert rarement.')).toBeInTheDocument();
    // Unfolded, it no longer names them: they are there.
    expect(fold).toHaveTextContent(/^Avancé$/);
    await userEvent.click(fold);
    expect(screen.queryByText('la rangée cachée')).toBeNull();
  });

  it('takes a title and the names given, and follows the language', () => {
    render(Host, { title: 'Zone sensible', hides: ['Fermer le projet'] });
    expect(screen.getByRole('button', { name: /^Zone sensible/ })).toHaveTextContent('Zone sensibleFermer le projet');
    setLang('en');
    flushSync();
    expect(screen.getByRole('button', { name: /^Zone sensible/ })).toBeInTheDocument();
  });

  it('says « Advanced » in English', () => {
    setLang('en');
    render(Host, { advanced: 'general' });
    expect(screen.getByRole('button', { name: /^Advanced/ })).toHaveTextContent('Advanced');
  });
});

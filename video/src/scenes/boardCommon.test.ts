import { describe, expect, it, vi } from 'vitest';

// The board's data, outside Remotion: its fonts are not loaded.
vi.mock('@remotion/google-fonts/HankenGrotesk', () => ({ loadFont: () => ({ fontFamily: 'Hanken Grotesk' }) }));
vi.mock('@remotion/google-fonts/JetBrainsMono', () => ({ loadFont: () => ({ fontFamily: 'JetBrains Mono' }) }));

const { boardOf, placesText, ticketAgent } = await import('./boardCommon');
const { fmtOf, trOf } = await import('../lang');

describe('the name of a ticket’s agent', () => {
  it('is its key and its title, as a slug cut at 40 characters on a word', () => {
    expect(ticketAgent('DEM-3', 'Pagination de /users')).toBe('dem-3-pagination-de-users');
    expect(ticketAgent('DEM-1', 'Réinitialisation du mot de passe')).toBe('dem-1-reinitialisation-du-mot-de-passe');
    // Cut in the middle of « connexion »: back to the word before.
    expect(ticketAgent('DEM-6', 'Limiter les tentatives de connexion')).toBe('dem-6-limiter-les-tentatives-de');
    // Cut exactly at the end of a word: it stays.
    expect(ticketAgent('DEM-8', 'Relancer les factures impayées par e-mail')).toBe('dem-8-relancer-les-factures-impayees-par');
    expect(ticketAgent('DEM-7', 'Wrong rounding on foreign-currency invoices')).toBe('dem-7-wrong-rounding-on-foreign-currency');
    expect(ticketAgent('DEM-8', 'Send email reminders for unpaid invoices')).toBe('dem-8-send-email-reminders-for-unpaid');
  });
});

describe('the demo board', () => {
  it('names its agents as before in French, and after its English titles in English', () => {
    const fr = boardOf(fmtOf('fr'));
    expect(fr.AGENT_CSV).toBe('dem-5-export-csv-des-factures');
    expect(fr.AGENT_LOGIN).toBe('dem-6-limiter-les-tentatives-de');
    expect(fr.SEARCH.agent!.name).toBe('dem-4-recherche-plein-texte-des-clients');
    expect(fr.DONE.map((t) => t.agent!.name)).toEqual([
      'dem-3-pagination-de-users',
      'dem-2-webhooks-stripe',
      'dem-1-reinitialisation-du-mot-de-passe',
    ]);
    const en = boardOf(fmtOf('en'));
    expect(en.AGENT_CSV).toBe('dem-5-invoice-csv-export');
    expect(en.AGENT_LOGIN).toBe('dem-6-limit-login-attempts');
    expect(en.DONE.map((t) => t.agent!.name)).toEqual(['dem-3-pagination-for-users', 'dem-2-webhooks-stripe', 'dem-1-password-reset']);
  });

  it('writes the cards’ numbers the language’s way', () => {
    expect(boardOf(fmtOf('fr')).DONE[0].doneMeta).toBe('2 boucles · 0,61 $');
    expect(boardOf(fmtOf('en')).DONE[0].doneMeta).toBe('2 loops · $0.61');
    expect(boardOf(fmtOf('fr')).boardAgents([{ name: 'a', tag: 't' }])[2]).toMatchObject({ tokens: '38 k', cost: '≈ 0,41 $' });
    expect(boardOf(fmtOf('en')).boardAgents([{ name: 'a', tag: 't' }])[2]).toMatchObject({ tokens: '38k', cost: '≈ $0.41' });
  });

  it('says how many places are left, in each language', () => {
    expect(placesText(trOf('fr'), 0)).toBe('Toutes les places sont prises');
    expect(placesText(trOf('fr'), 1)).toBe('1 place libre');
    expect(placesText(trOf('fr'), 3)).toBe('3 places libres');
    expect(placesText(trOf('en'), 0)).toBe('All slots taken');
    expect(placesText(trOf('en'), 1)).toBe('1 free slot');
    expect(placesText(trOf('en'), 3)).toBe('3 free slots');
  });
});

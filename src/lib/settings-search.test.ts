import { afterEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { SETTINGS_ROWS, pageInfo } from './settings-pages';
import { rowElement, searchSettings } from './settings-search';

const PROJECTS = [{ id: 'p1' }, { id: 'p2' }];

/** Upper case, the accents taken off: what someone types in a hurry. */
const rough = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toUpperCase();

const found = (query: string, id: string, current: string | null = 'p1') =>
  searchSettings(query, PROJECTS, current, 10_000).some((h) => h.row?.id === id);

afterEach(() => setLang('fr'));

describe('the search of the settings', () => {
  // Each row of the register, by its label in each language, whatever the language of the interface: as written, then
  // in capitals without its accents.
  for (const ui of ['fr', 'en'] as const)
    for (const lang of ['fr', 'en'] as const)
      it(`finds each row by its label in ${lang}, the interface in ${ui}`, () => {
        setLang(ui);
        const missed = SETTINGS_ROWS.filter((r) => {
          const label = r.labelIn(lang);
          return !found(label, r.id) || !found(rough(label), r.id);
        }).map((r) => `${r.id} (« ${r.labelIn(lang)} »)`);
        expect(missed).toEqual([]);
      });

  it('opens a row of a project in each project, the one chosen first', () => {
    const hits = searchSettings('worktree par agent', PROJECTS, 'p2').filter((h) => h.row?.id === 'worktreePerAgent');
    expect(hits.map((h) => h.projectId)).toEqual(['p2', 'p1']);
    expect(hits.every((h) => h.page === 'git')).toBe(true);
  });

  it('puts a label that starts with the words first: « proxy » is Avancé › Réseau', () => {
    const [first] = searchSettings('proxy', PROJECTS);
    expect(first.row?.id).toBe('proxyUrl');
    expect(pageInfo(first.page).id).toBe('advanced');
  });

  it('finds « bêta » and « beta » on Général', () => {
    for (const q of ['bêta', 'BETA', 'beta versions']) {
      const [first] = searchSettings(q, PROJECTS);
      expect(first.row?.id, q).toBe('betaUpdates');
      expect(first.page).toBe('general');
    }
  });

  it('reads the descriptions too, after the labels', () => {
    // « Exclusions » says NO_PROXY in its hint alone.
    const hits = searchSettings('NO_PROXY', PROJECTS);
    expect(hits[0]?.row?.id).toBe('noProxy');
  });

  it('names the rows folded into « Avancé »', () => {
    const hit = searchSettings('identifiant', PROJECTS).find((h) => h.row?.id === 'userId');
    expect(hit?.row?.advanced).toBe(true);
    expect(hit?.page).toBe('general');
  });

  it('finds the pages by their name', () => {
    expect(searchSettings('confidentialite', PROJECTS, 'p1')[0]).toMatchObject({ page: 'privacy', projectId: 'p1', row: null });
    expect(searchSettings('connected services', PROJECTS)[0]).toMatchObject({ page: 'services', row: null });
  });

  it('answers nothing to an empty query, nor to words no setting holds', () => {
    expect(searchSettings('   ', PROJECTS)).toEqual([]);
    expect(searchSettings('zzqx', PROJECTS)).toEqual([]);
  });

  it('finds the row on the page by its label, its hint after it, else the group named so', () => {
    document.body.innerHTML = `
      <div class="sg"><h2>Réseau</h2>
        <div class="sr" id="a"><div class="lab"><b class="l">Proxy HTTP(S)&nbsp;<span class="h">ex. http://proxy:3128</span></b></div></div>
        <div class="sr" id="b"><div class="lab"><b class="l">Exclusions</b></div></div>
      </div>
      <div class="sg" id="g"><h2>Instances connectées</h2></div>`;
    const row = (id: string) => SETTINGS_ROWS.find((r) => r.id === id)!;
    expect(rowElement(document.body, row('proxyUrl'))?.id).toBe('a');
    expect(rowElement(document.body, row('noProxy'))?.id).toBe('b');
    expect(rowElement(document.body, row('forges'))?.id).toBe('g');
    expect(rowElement(document.body, row('sound'))).toBeNull();
  });
});

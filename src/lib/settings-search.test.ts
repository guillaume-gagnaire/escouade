import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { BARE_ENGINE, CLAUDE_ENGINE, resetApp } from '../test/ipc';
import { firstHandPlugins } from '../test/plugins';
import { setLang } from './i18n';
import { plugins } from './plugins-view.svelte';
import { app } from './state.svelte';
import { SETTINGS_ROWS, pageInfo } from './settings-pages';
import { rowElement, searchSettings } from './settings-search';

const PROJECTS = [{ id: 'p1' }, { id: 'p2' }];

/** Upper case, the accents taken off: what someone types in a hurry. */
const rough = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toUpperCase();

const rowLabel = (id: string) => SETTINGS_ROWS.find((r) => r.id === id)!.labelIn('fr');
const found = (query: string, id: string, current: string | null = 'p1') =>
  searchSettings(query, PROJECTS, current, 10_000).some((h) => h.row?.id === id);

/** Claude Code, Codex, and OpenCode (its models its providers', the texts written on the model chosen): every row is on its page. */
const OPENCODE = { ...BARE_ENGINE, id: 'opencode' as const, models: [], writerTakesModel: true };

beforeEach(() => {
  resetApp();
  app.engines = [CLAUDE_ENGINE, BARE_ENGINE, OPENCODE];
  app.enginesFound = ['claude', 'codex', 'opencode'];
  plugins.list = firstHandPlugins();
  plugins.loaded = true;
});

afterEach(() => {
  setLang('fr');
  plugins.list = [];
  plugins.loaded = false;
});

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

  // ESC-407: what an installed plugin brings alone is found; its plugin gone, its row is no longer on its page, nor found.
  it('finds no row of what no installed plugin brings', () => {
    // The core keeps declaring its own harnesses: their plugins decide. Claude Code's and OpenCode's uninstalled.
    plugins.list = firstHandPlugins('claude', 'opencode');
    for (const id of ['engineOrder', 'selfUpdate', 'handoverOnLimit', 'handoverModel', 'favoriteModels', 'writerModel'])
      expect(found(rowLabel(id), id), id).toBe(false);
    expect(found(rowLabel('claudePath'), 'claudePath')).toBe(true);
    // No harness's plugin at all.
    plugins.list = firstHandPlugins('claude', 'opencode', 'codex');
    for (const id of ['accountsList', 'claudePath', 'terminalView', 'todoTools']) expect(found(rowLabel(id), id), id).toBe(false);
    // No source of tickets installed: their sync, their import and the mapping of their states go; the groups that say so stay.
    plugins.list = firstHandPlugins('jira', 'trello', 'github', 'linear', 'clickup', 'gitlab', 'azure');
    for (const id of ['sync', 'autoImport', 'mapping']) expect(found(rowLabel(id), id), id).toBe(false);
    expect(found(rowLabel('ticketAccounts'), 'ticketAccounts')).toBe(true);
    // Back with their plugins.
    plugins.list = firstHandPlugins();
    for (const id of ['sync', 'engineOrder', 'selfUpdate', 'handoverOnLimit', 'favoriteModels'])
      expect(found(rowLabel(id), id), id).toBe(true);
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

  it('finds « Économies du cache dans la barre d’état » in Comptes et harness › Avancé by « cache » and « status bar » (ESC-421)', () => {
    for (const query of ['economies du cache', 'barre d’état', 'cache savings']) {
      const hit = searchSettings(query, PROJECTS).find((h) => h.row?.id === 'cacheSavingsInBar');
      expect(hit?.page, query).toBe('accounts');
      expect(hit?.row?.advanced).toBe(true);
    }
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

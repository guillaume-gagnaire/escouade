import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { beforeEach, describe, expect, it } from 'vitest';
import {
  anyCan,
  can,
  canFork,
  capsOf,
  choosesEngine,
  effortsOf,
  engineName,
  engineOf,
  engineOptions,
  favoritesOf,
  modelCost,
  modelMenu,
  modelsOf,
  modelTakes,
  modesOf,
  offered,
  readingModels,
  searchesModels,
  setFavorite,
  showsCost,
  showsMark,
  speakerOf,
  writerOf,
  writerOptions,
} from './engines';
import { setLang } from './i18n';
import { EFFORTS, modelLabel, modelOptions, MODES } from './models';
import { app } from './state.svelte';
import { agent, BARE_ENGINE, CLAUDE_ENGINE, fakeBackend, resetApp } from '../test/ipc';

describe('the capabilities of a harness', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('are read from what the harness declares, the default one for what has none yet', () => {
    expect(engineOf('claude')?.name).toBe('Claude Code');
    expect(engineOf(null)?.name).toBe('Claude Code');
    expect(engineOf('codex')?.name).toBe('Harness nu');
    // One the backend does not declare: nothing of it is shown.
    expect(capsOf('cursor')).toBeNull();
    expect(can({ engine: 'cursor' }, 'remoteControl')).toBe(false);
  });

  it('name a harness this version lacks « harness absent », its functions hidden (INT-3)', () => {
    setLang('fr');
    expect(engineName('claude')).toBe('Claude Code');
    // Written into the app, not yet declared: its usual name.
    expect(engineName('gemini')).toBe('Gemini CLI');
    // Neither: a later version's, a plugin's gone.
    expect(engineName('hermes')).toBe('hermes (harness absent)');
    expect(capsOf('hermes')).toBeNull();
    expect(canFork({ engine: 'hermes' })).toBe(false);
    setLang('en');
    expect(engineName('hermes')).toBe('hermes (harness missing)');
    // No harness yet: the default one's.
    expect(engineName(null)).toBe('Claude Code');
    setLang('fr');
  });

  it('show what Claude Code does today and hide what a bare harness lacks', () => {
    const claude = { engine: 'claude' as const };
    const bare = { engine: 'codex' as const };
    for (const flag of ['terminalView', 'remoteControl', 'backgroundTasks', 'effort', 'images', 'pdf', 'slashCommands'] as const) {
      expect(can(claude, flag)).toBe(true);
      expect(can(bare, flag)).toBe(false);
    }
    expect(canFork(claude)).toBe(true);
    expect(canFork(bare)).toBe(false);
    expect(showsCost(claude)).toBe(true);
    expect(showsCost(bare)).toBe(false);
    expect(modesOf(claude, MODES).map((m) => m.value)).toEqual(['auto', 'default', 'plan', 'acceptEdits', 'bypassPermissions']);
    expect(modesOf(bare, MODES).map((m) => m.value)).toEqual(['default', 'plan']);
  });

  it('tell whether any harness has a function, for a setting of them all', () => {
    expect(anyCan('remoteControl')).toBe(true);
    app.engines = [BARE_ENGINE];
    expect(anyCan('remoteControl')).toBe(false);
  });
});

describe('the harnesses offered (ESC-29)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('are the harnesses found: a harness that is not found is offered nowhere', () => {
    app.enginesFound = ['claude'];
    expect(offered().map((e) => e.id)).toEqual(['claude']);
    expect(choosesEngine()).toBe(false);
    app.enginesFound = ['claude', 'codex'];
    expect(choosesEngine()).toBe(true);
    expect(engineOptions()).toEqual([
      { value: 'claude', label: 'Claude Code' },
      { value: 'codex', label: 'Harness nu' },
    ]);
    // The one a ticket or a project has stays listed, though it is not found any more.
    app.enginesFound = ['claude'];
    expect(engineOptions('codex').map((o) => o.value)).toEqual(['claude', 'codex']);
  });

  it('show their brand only once agents run on two of them, archived ones aside', () => {
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2' })] });
    expect(showsMark()).toBe(false);
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', engine: 'codex', archived: true })] });
    expect(showsMark()).toBe(false);
    resetApp({ agents: [agent({ id: 'a1' }), agent({ id: 'a2', engine: 'codex' })] });
    expect(showsMark()).toBe(true);
  });

  it('give the composer the models and efforts of the harness, each model with what it takes', () => {
    const claude = { engine: 'claude' as const };
    const bare = { engine: 'codex' as const };
    expect(modelsOf(claude).map((m) => m.id)).toEqual(['fable', 'opus', 'sonnet', 'haiku']);
    expect(modelsOf(bare).map((m) => m.id)).toEqual(['gpt-5.5', 'gpt-5.5-mini']);
    expect(effortsOf(claude, EFFORTS).map((e) => e.value)).toEqual(['low', 'medium', 'high', 'xhigh', 'max']);
    expect(effortsOf(bare, EFFORTS)).toEqual([]);
    // As Claude Code's rule: all but Haiku, its full ids too; a model it does not declare takes them.
    expect(modelTakes(claude, 'opus', 'effort')).toBe(true);
    expect(modelTakes(claude, 'haiku', 'effort')).toBe(false);
    expect(modelTakes(claude, 'claude-haiku-4-5-20251001', 'auto')).toBe(false);
    expect(modelTakes(claude, 'claude-opus-5-5', 'auto')).toBe(true);
    // The alias haiku runs Haiku 5.5, with the mode auto, from Claude Code 2.1.293 on.
    expect(modelTakes(claude, 'haiku', 'auto')).toBe(true);
    expect(modelTakes(bare, 'gpt-5.5', 'effort')).toBe(false);
  });
});

describe('the mode auto as the claude of the account told it (ESC-121)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('follows what the claude of the agent’s account told, else the rule', () => {
    const pro = { engine: 'claude' as const, account: 'pro' };
    const principal = { engine: 'claude' as const, account: 'principal' };
    // Nothing told: the rule, the alias as from Claude Code 2.1.293 on.
    expect(modelTakes(pro, 'haiku', 'auto')).toBe(true);
    // Pro's claude is a 2.1.292: its haiku runs Haiku 4.5, without the mode; Principal's a 2.1.302.
    app.accountModels = {
      pro: {
        version: '2.1.292',
        models: [
          { value: 'opus', resolvedModel: 'claude-opus-5-5', supportsAutoMode: true },
          { value: 'haiku', resolvedModel: 'claude-haiku-4-5-20251001' },
        ],
      },
      principal: { version: '2.1.302', models: [{ value: 'haiku', resolvedModel: 'claude-haiku-5-5', supportsAutoMode: true }] },
    };
    expect(modelTakes(pro, 'haiku', 'auto')).toBe(false);
    expect(modelTakes(pro, 'opus', 'auto')).toBe(true);
    expect(modelTakes(principal, 'haiku', 'auto')).toBe(true);
    // A model its claude does not list: the rule.
    expect(modelTakes(principal, 'claude-haiku-4-5', 'auto')).toBe(false);
    expect(modelTakes(principal, 'sonnet', 'auto')).toBe(true);
    // Another harness keeps its declaration.
    expect(modelTakes({ engine: 'codex', account: 'pro' }, 'haiku', 'auto')).toBe(true);
  });
});

describe('the models of Claude Code as the claude of each account told them (ESC-124)', () => {
  const efforts = ['low', 'medium', 'high', 'xhigh', 'max'];
  const told = (id: string, label: string, more: Partial<import('./types').ToldModel> = {}) => ({
    id,
    label,
    efforts,
    defaultEffort: null,
    images: true,
    default: false,
    ...more,
  });
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings = {
      ...app.settings,
      accounts: [
        { id: 'principal', name: 'Principal', configDir: '', claudePath: '', active: true },
        { id: 'pro', name: 'Pro', configDir: 'C:/pro', claudePath: '', active: true },
        { id: 'off', name: 'Éteint', configDir: 'C:/off', claudePath: '', active: false },
      ],
    };
    app.accountModels = {
      // Principal has no Fable; Pro has Mythos more, which takes two efforts, and Haiku 4.5, which takes none.
      principal: {
        version: '2.1.302',
        models: [],
        catalog: [told('opus', 'Opus 5.5', { default: true }), told('sonnet', 'Sonnet 5.5'), told('haiku', 'Haiku 5.5')],
      },
      pro: {
        version: '2.1.302',
        models: [],
        catalog: [
          told('opus', 'Opus 5.5', { default: true }),
          told('mythos', 'Mythos', { efforts: ['high', 'max'] }),
          told('claude-haiku-4-5-20251001', 'Haiku 4.5', { efforts: [] }),
        ],
      },
      off: { version: '2.1.302', models: [], catalog: [told('secret', 'Secret')] },
    };
  });

  it('the composer of an agent shows its account’s: a model more is there, one it lacks is not', () => {
    const principal = { engine: 'claude' as const, account: 'principal' };
    const pro = { engine: 'claude' as const, account: 'pro', model: 'mythos' };
    expect(modelsOf(principal).map((m) => m.id)).toEqual(['opus', 'sonnet', 'haiku']);
    expect(modelsOf(pro)).toEqual([
      { id: 'opus', label: 'Opus 5.5' },
      { id: 'mythos', label: 'Mythos' },
      { id: 'claude-haiku-4-5-20251001', label: 'Haiku 4.5' },
    ]);
    // Each with its efforts, as told: Haiku 5.5 takes one, Haiku 4.5 none.
    expect(effortsOf(pro, EFFORTS).map((e) => e.value)).toEqual(['high', 'max']);
    expect(modelTakes(principal, 'haiku', 'effort')).toBe(true);
    expect(modelTakes(pro, 'claude-haiku-4-5-20251001', 'effort')).toBe(false);
    // Named as its claude names it, wherever it is shown, by its full id too (the statistics').
    expect(modelLabel('mythos')).toBe('Mythos');
    app.accountModels = {
      ...app.accountModels,
      pro: {
        ...app.accountModels.pro,
        models: [{ value: 'mythos', resolvedModel: 'claude-mythos-1', displayName: 'Mythos', supportsAutoMode: true }],
      },
    };
    expect(modelLabel('claude-mythos-1')).toBe('Mythos');
    expect(modelLabel('claude-opus-5-5')).toBe('Opus 5.5');
  });

  it('an account that told none yet, as before the first reading: the fixed list', () => {
    const fresh = { engine: 'claude' as const, account: 'nouveau' };
    expect(modelsOf(fresh).map((m) => m.id)).toEqual(['fable', 'opus', 'sonnet', 'haiku']);
    app.accountModels = {};
    expect(modelOptions([]).map((m) => m.value)).toEqual(['fable', 'opus', 'sonnet', 'haiku']);
  });

  it('the Kanban, a ticket and the settings offer those of the active accounts, each once', () => {
    expect(modelOptions([]).map((m) => m.value)).toEqual(['opus', 'sonnet', 'haiku', 'mythos', 'claude-haiku-4-5-20251001']);
    expect(modelOptions([]).find((m) => m.value === 'mythos')?.label).toBe('Mythos');
  });
});

describe('the models a harness lists for its account (ESC-37)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, caps: { ...BARE_ENGINE.caps, effort: true } }];
  });

  it('take the place of those it declares, each with its own efforts', () => {
    const codex = { engine: 'codex' as const, model: 'gpt-6-luna' };
    // Until it has listed them: its declaration's.
    expect(modelsOf(codex).map((m) => m.id)).toEqual(['gpt-5.5', 'gpt-5.5-mini']);
    app.engineModels = {
      codex: [
        { id: 'gpt-6-luna', label: 'GPT-6-Luna', efforts: ['low', 'medium', 'high'], defaultEffort: 'medium', images: true, default: true },
        { id: 'gpt-mini', label: 'GPT Mini', efforts: [], defaultEffort: null, images: false, default: false },
      ],
    };
    expect(modelsOf(codex)).toEqual([
      { id: 'gpt-6-luna', label: 'GPT-6-Luna' },
      { id: 'gpt-mini', label: 'GPT Mini' },
    ]);
    expect(effortsOf(codex, EFFORTS).map((e) => e.value)).toEqual(['low', 'medium', 'high']);
    expect(modelTakes(codex, 'gpt-6-luna', 'effort')).toBe(true);
    expect(modelTakes(codex, 'gpt-mini', 'effort')).toBe(false);
    expect(modelTakes(codex, 'gpt-mini', 'auto')).toBe(true);
    // Named as its harness names it, wherever a model is shown.
    expect(modelLabel('gpt-6-luna')).toBe('GPT-6-Luna');
    // Claude Code's menu is its own still.
    expect(modelsOf({ engine: 'claude' }).map((m) => m.id)).toEqual(['fable', 'opus', 'sonnet', 'haiku']);
  });
});

describe('the models of a Codex account (ESC-186)', () => {
  const told = (id: string, label: string) => ({ id, label, efforts: [], defaultEffort: null, images: false, default: false });
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
    app.settings.accounts = [
      { ...app.settings.accounts[0], id: 'travail', engine: 'codex', active: true },
      { ...app.settings.accounts[0], id: 'perso', engine: 'codex', active: true },
    ];
  });

  it('are its account’s once read, the stand-in until then, and those of every account where none is said', () => {
    const work = { engine: 'codex' as const, account: 'travail' };
    expect(readingModels(work)).toBe(true);
    expect(modelsOf(work).map((m) => m.id)).toEqual(['gpt-5.5', 'gpt-5.5-mini']);
    app.accountModels = {
      travail: { version: '0.163.0', models: [], catalog: [told('gpt-6-luna', 'GPT-6-Luna'), told('gpt-5.6-terra', 'GPT-5.6-Terra')] },
      perso: { version: '0.163.0', models: [], catalog: [told('gpt-6-luna', 'GPT-6-Luna'), told('gpt-6-sol', 'GPT-6-Sol')] },
    };
    expect(readingModels(work)).toBe(false);
    expect(modelsOf(work).map((m) => m.id)).toEqual(['gpt-6-luna', 'gpt-5.6-terra']);
    expect(modelsOf({ engine: 'codex', account: 'perso' }).map((m) => m.id)).toEqual(['gpt-6-luna', 'gpt-6-sol']);
    // The Kanban, a ticket: each model of its accounts once.
    expect(modelsOf({ engine: 'codex' }).map((m) => m.id)).toEqual(['gpt-6-luna', 'gpt-5.6-terra', 'gpt-6-sol']);
    // Claude Code is not concerned.
    expect(readingModels({ engine: 'claude' })).toBe(false);
  });
});

describe('what a model costs beside the others (ESC-39)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, id: 'copilot', caps: { ...BARE_ENGINE.caps, effort: true } }];
  });

  it('is its multiplier, else its tier of price, else nothing', () => {
    const copilot = { engine: 'copilot' as const };
    app.engineModels = {
      copilot: [
        { id: 'sonnet', label: 'Claude Sonnet 4.5', efforts: [], defaultEffort: null, images: true, default: true, multiplier: '3' },
        { id: 'mini', label: 'GPT-5 mini', efforts: [], defaultEffort: null, images: false, default: false, multiplier: '0.33' },
        { id: 'luna', label: 'GPT-6 Luna', efforts: [], defaultEffort: null, images: false, default: false, price: 'low' },
        { id: 'plain', label: 'Plain', efforts: [], defaultEffort: null, images: false, default: false },
      ],
    };
    expect(modelCost(copilot, 'sonnet')).toEqual({ label: '×3', title: 'Coûte 3 fois le tarif de base' });
    expect(modelCost(copilot, 'mini')?.label).toBe('×0,33');
    expect(modelCost(copilot, 'luna')?.label).toBe('coût bas');
    expect(modelCost(copilot, 'plain')).toBeUndefined();
    expect(modelCost({ engine: 'claude' }, 'opus')).toBeUndefined();
    setLang('en');
    expect(modelCost(copilot, 'mini')?.label).toBe('×0.33');
    expect(modelCost(copilot, 'luna')?.label).toBe('low cost');
    setLang('fr');
  });
});

describe('the name of the agents of a harness (ESC-31)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('is the one it declares: Claude for Claude Code, the default harness’s for what has none', () => {
    expect(speakerOf('claude')).toBe('Claude');
    expect(speakerOf(null)).toBe('Claude');
    expect(speakerOf('codex')).toBe('Nu');
  });

  it('is its name when it declares none, its id when the backend does not know it', () => {
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, speaker: '' }];
    expect(speakerOf('codex')).toBe('Harness nu');
    expect(speakerOf('opencode')).toBe('opencode');
  });
});

describe('the window', () => {
  /** Every source file of the window but the tests. */
  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return sources(path);
      return /\.(ts|svelte)$/.test(name) && !/\.test\.ts$/.test(name) ? [path] : [];
    });
  }

  it('never decides from the name of a harness, only from its capabilities', () => {
    const offenders: string[] = [];
    // `engine === 'claude'`, `.engine !== "codex"`, `engine == 'opencode'`, `case 'copilot':` after a switch on an engine…
    const byName =
      /engine\)?\s*[!=]==?\s*['"](claude|codex|copilot|cursor|opencode)['"]|['"](claude|codex|copilot|cursor|opencode)['"]\s*[!=]==?\s*\S*engine\b|switch\s*\([^)]*engine[^)]*\)/;
    // The search finds what it is for.
    for (const line of ["if (agent.engine === 'claude')", 'a.engine !== "codex"', "'opencode' === t.engine", 'switch (agent.engine) {']) {
      expect(byName.test(line), line).toBe(true);
    }
    expect(byName.test("can(agent, 'remoteControl')")).toBe(false);
    for (const file of sources(join(__dirname, '..'))) {
      readFileSync(file, 'utf8')
        .split('\n')
        .forEach((line, i) => {
          if (byName.test(line)) offenders.push(`${file}:${i + 1}: ${line.trim()}`);
        });
    }
    expect(offenders).toEqual([]);
  });
});

describe('« Modèle des textes rédigés »', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('offers the model of each harness found, and only those', () => {
    app.enginesFound = ['claude', 'codex'];
    expect(writerOptions()).toEqual([
      { value: 'claude', label: 'Claude Haiku' },
      { value: 'codex', label: 'Nu mini' },
    ]);
    app.enginesFound = ['codex'];
    expect(writerOptions()).toEqual([{ value: 'codex', label: 'Nu mini' }]);
    app.enginesFound = [];
    expect(writerOptions()).toEqual([]);
  });

  it('goes to the harness chosen while it is found, else to Claude Haiku, else to the first found', () => {
    app.enginesFound = ['codex', 'claude'];
    expect(writerOf(null)).toBe('claude');
    expect(writerOf('codex')).toBe('codex');
    app.enginesFound = ['claude'];
    expect(writerOf('codex')).toBe('claude');
    app.enginesFound = ['codex'];
    expect(writerOf(null)).toBe('codex');
    expect(writerOf('claude')).toBe('codex');
    app.enginesFound = [];
    expect(writerOf('claude')).toBeNull();
  });
});

describe('the models of the providers of a harness (ESC-44)', () => {
  beforeEach(() => {
    resetApp();
    app.engines = [CLAUDE_ENGINE, { ...BARE_ENGINE, id: 'opencode', models: [], caps: { ...BARE_ENGINE.caps, effort: true } }];
  });

  it('are searched, the favorites first in their order, each with its provider', async () => {
    const oc = { engine: 'opencode' as const, model: 'ollama/qwen' };
    expect(searchesModels(oc)).toBe(false);
    app.engineModels = {
      opencode: [
        {
          id: 'ollama/gpt-oss',
          label: 'gpt-oss',
          efforts: ['low', 'high'],
          defaultEffort: null,
          images: false,
          default: false,
          provider: 'Ollama (local)',
        },
        { id: 'ollama/qwen', label: 'Qwen', efforts: [], defaultEffort: null, images: false, default: true, provider: 'Ollama (local)' },
        {
          id: 'lmstudio/devstral',
          label: 'Devstral',
          efforts: [],
          defaultEffort: null,
          images: true,
          default: false,
          provider: 'LM Studio',
        },
      ],
    };
    expect(searchesModels(oc)).toBe(true);
    expect(modelMenu(oc).map((m) => m.id)).toEqual(['ollama/gpt-oss', 'ollama/qwen', 'lmstudio/devstral']);
    app.settings = { ...app.settings, favoriteModels: { opencode: ['lmstudio/devstral', 'ollama/qwen'] } };
    const menu = modelMenu(oc);
    expect(menu.map((m) => [m.id, m.favorite])).toEqual([
      ['lmstudio/devstral', true],
      ['ollama/qwen', true],
      ['ollama/gpt-oss', false],
    ]);
    expect(menu[0].provider).toBe('LM Studio');
    expect(favoritesOf('opencode')).toEqual(['lmstudio/devstral', 'ollama/qwen']);
    // A variant is the effort of the model that has it.
    expect(effortsOf({ engine: 'opencode', model: 'ollama/gpt-oss' }, EFFORTS).map((e) => e.value)).toEqual(['low', 'high']);
    expect(modelTakes(oc, 'ollama/qwen', 'effort')).toBe(false);
    // Marked at once, through the backend.
    const backend = fakeBackend({ favorite_model_set: () => ({ opencode: ['ollama/gpt-oss'] }) });
    await setFavorite('opencode', 'ollama/gpt-oss', true);
    expect(backend.called('favorite_model_set')[0].args).toEqual({ engine: 'opencode', model: 'ollama/gpt-oss', favorite: true });
    expect(favoritesOf('opencode')).toEqual(['ollama/gpt-oss']);
  });
});

describe('the menu of the models in groups (ESC-127)', () => {
  const told = (id: string, label: string, provider?: string) => ({
    id,
    label,
    efforts: [],
    defaultEffort: null,
    images: true,
    default: false,
    ...(provider ? { provider } : {}),
  });
  const shown = (who: Parameters<typeof modelMenu>[0]) => modelMenu(who).map((m) => `${m.group}: ${m.label}`);

  beforeEach(() => {
    resetApp();
    setLang('fr');
    app.engines = [CLAUDE_ENGINE, BARE_ENGINE];
  });

  it('groups Claude Code’s by family, in their order, the newest first in each, those of none in « Autres »', () => {
    const resolved = (value: string, resolvedModel: string) => ({ value, resolvedModel });
    app.accountModels = {
      principal: {
        version: '2.1.302',
        models: [resolved('opus', 'claude-opus-5-5'), resolved('haiku', 'claude-haiku-5-5'), resolved('mythos', 'claude-mythos-1')],
        catalog: [
          told('claude-haiku-4-5-20251001', 'Haiku 4.5'),
          told('opus', 'Opus 5.5'),
          told('claude-sonnet-4-6', 'Sonnet 4.6'),
          told('mythos', 'Mythos'),
          told('claude-opus-5', 'Opus 5'),
          told('labs-preview', 'Aperçu du labo'),
          told('haiku', 'Haiku 5.5'),
          told('claude-nova-2', 'Nova 2'),
          told('fable', 'Fable 5.1'),
          told('sonnet', 'Sonnet 5.5'),
        ],
      },
    };
    const who = { engine: 'claude' as const, account: 'principal' };
    expect(searchesModels(who)).toBe(true);
    expect(shown(who)).toEqual([
      'Fable: Fable 5.1',
      'Opus: Opus 5.5',
      'Opus: Opus 5',
      'Sonnet: Sonnet 5.5',
      'Sonnet: Sonnet 4.6',
      'Haiku: Haiku 5.5',
      'Haiku: Haiku 4.5',
      'Mythos: Mythos',
      // A family the claude lists that Escouade does not know: after the others.
      'Nova: Nova 2',
      'Autres: Aperçu du labo',
    ]);
    // The favorites first, in their order, out of their family.
    app.settings = { ...app.settings, favoriteModels: { claude: ['claude-sonnet-4-6', 'fable'] } };
    expect(shown(who).slice(0, 4)).toEqual(['Favoris: Sonnet 4.6', 'Favoris: Fable 5.1', 'Opus: Opus 5.5', 'Opus: Opus 5']);
    expect(
      modelMenu(who)
        .filter((m) => m.favorite)
        .map((m) => m.id),
    ).toEqual(['claude-sonnet-4-6', 'fable']);
    // In English.
    setLang('en');
    expect(shown(who)[0]).toBe('Favorites: Sonnet 4.6');
    expect(shown(who).at(-1)).toBe('Others: Aperçu du labo');
  });

  it('keeps the order listed of the models of a family that have no version', () => {
    app.accountModels = {
      principal: {
        version: '2.1.302',
        models: [],
        catalog: [
          told('opus-plan', 'Opus Plan'),
          told('opus', 'Opus'),
          told('claude-opus-4-1', 'Opus 4.1'),
          told('opus-fast', 'Opus rapide'),
        ],
      },
    };
    expect(shown({ engine: 'claude', account: 'principal' })).toEqual([
      'Opus: Opus Plan',
      'Opus: Opus',
      'Opus: Opus rapide',
      'Opus: Opus 4.1',
    ]);
  });

  it('groups OpenCode’s by provider, in the order it lists them, those of none last', () => {
    app.engineModels = {
      opencode: [
        told('ollama/qwen', 'Qwen', 'Ollama (local)'),
        told('lmstudio/devstral', 'Devstral', 'LM Studio'),
        told('zen/free', 'Free'),
        told('ollama/gpt-oss', 'gpt-oss', 'Ollama (local)'),
      ],
    };
    expect(shown({ engine: 'opencode' })).toEqual([
      'Ollama (local): Qwen',
      'Ollama (local): gpt-oss',
      'LM Studio: Devstral',
      'Autres: Free',
    ]);
  });

  it('groups those of Codex or Copilot CLI by the vendor of their id', () => {
    app.engineModels = {
      copilot: [
        told('claude-sonnet-4.5', 'Claude Sonnet 4.5'),
        told('gpt-5', 'GPT-5'),
        told('auto', 'Auto'),
        told('gemini-2.5-pro', 'Gemini 2.5 Pro'),
        told('o3', 'o3'),
      ],
    };
    expect(shown({ engine: 'copilot' })).toEqual([
      'GPT: GPT-5',
      'GPT: o3',
      'Claude: Claude Sonnet 4.5',
      'Gemini: Gemini 2.5 Pro',
      'Autres: Auto',
    ]);
  });
});

import { describe, expect, it } from 'vitest';
import { parseReport, splitEscouade } from './escouade';

describe('escouade blocks', () => {
  it('reads the criteria, under either name, and the launch recipe in French', () => {
    expect(parseReport('{"criteres": [{"n": 1, "ok": true, "note": "vu"}, {"n": 0, "ok": true}]}')).toEqual({
      criteria: [{ n: 1, ok: true, note: 'vu' }],
      recipe: null,
      progress: null,
    });
    expect(parseReport('{"criteria": [{"n": 2}]}')?.criteria).toEqual([{ n: 2, ok: false, note: '' }]);
    const r = parseReport(
      '{"lancement": {"preparation": [{"commande": "npm i", "dossier": "web"}], "processus": [{"nom": "web", "commande": "npm run dev", "env": {"PORT": 4101}, "url": "http://localhost:4101"}], "ouvrir": "http://localhost:4101/x"}}',
    );
    expect(r?.recipe).toEqual({
      prepare: [{ command: 'npm i', dir: 'web' }],
      processes: [{ name: 'web', command: 'npm run dev', dir: '', env: { PORT: '4101' }, url: 'http://localhost:4101' }],
      open: 'http://localhost:4101/x',
    });
    expect(parseReport('pas du json')).toBeNull();
    expect(parseReport('{}')).toBeNull();
  });

  it('reads criteria as the backend does: n as a number or a string of digits, the rest tolerated', () => {
    const criteria = (json: string) => parseReport(json)?.criteria;
    // `n` is a positive integer or a string of digits; any other item is skipped.
    expect(
      criteria(
        '{"criteres": [{"n": "2", "ok": true}, {"n": " 3 "}, {"n": "+4", "ok": true}, {"n": "x"}, {"n": "0"}, {"n": -1}, {"n": 1.5}, {"n": null}, {"ok": true}, null, 7, "texte"]}',
      ),
    ).toEqual([
      { n: 2, ok: true, note: '' },
      { n: 3, ok: false, note: '' },
      { n: 4, ok: true, note: '' },
    ]);
    // `ok` is true only for a boolean true; a note is the trimmed text, nothing else.
    expect(
      criteria(
        '{"criteres": [{"n": 1, "ok": "true", "note": 3}, {"n": 2, "ok": null, "note": null}, {"n": 3, "ok": true, "note": "  vu  "}, {"n": 4, "ok": 1}]}',
      ),
    ).toEqual([
      { n: 1, ok: false, note: '' },
      { n: 2, ok: false, note: '' },
      { n: 3, ok: true, note: 'vu' },
      { n: 4, ok: false, note: '' },
    ]);
    // An empty list is a list; one of which nothing is usable is none (it must not read as "none reached").
    expect(criteria('{"criteres": []}')).toEqual([]);
    expect(criteria('{"criteres": [{"n": 0}, {"n": "a"}], "avancement": ["A"]}')).toBeNull();
    expect(criteria('{"criteres": "oui", "avancement": ["A"]}')).toBeNull();
    // Nothing else to show: the block stays code.
    expect(parseReport('{"criteres": [{"n": 0}]}')).toBeNull();
    expect(parseReport('{"criteres": null, "criteria": null}')).toBeNull();
  });

  it('reads the launch recipe under its English names too', () => {
    expect(
      parseReport(
        '{"lancement": {"prepare": [{"command": "npm i", "dir": "web"}], "processes": [{"name": "web", "command": "npm run dev", "dir": "web", "env": {"PORT": "4101"}, "url": "http://localhost:4101"}], "open": "http://localhost:4101/x"}}',
      )?.recipe,
    ).toEqual({
      prepare: [{ command: 'npm i', dir: 'web' }],
      processes: [{ name: 'web', command: 'npm run dev', dir: 'web', env: { PORT: '4101' }, url: 'http://localhost:4101' }],
      open: 'http://localhost:4101/x',
    });
    // Both spellings in one recipe.
    expect(
      parseReport('{"lancement": {"prepare": [{"commande": "npm i"}], "processus": [{"name": "web", "commande": "npm start"}]}}')?.recipe,
    ).toEqual({
      prepare: [{ command: 'npm i', dir: '' }],
      processes: [{ name: 'web', command: 'npm start', dir: '', env: {}, url: '' }],
      open: '',
    });
  });

  it('reads a recipe as the backend does: nulls tolerated, a folder confined, nothing to launch is no recipe', () => {
    const recipe = (lancement: string) => parseReport(`{"criteres": [{"n": 1, "ok": true}], "lancement": ${lancement}}`)?.recipe;
    expect(
      recipe(
        '{"preparation": null, "processus": [{"nom": null, "commande": "npm run dev", "dossier": null, "env": null, "url": null}, {"commande": "npm start", "env": {"PORT": 4112, "DEBUG": null, "OK": true, "NAME": "x"}}], "ouvrir": null}',
      ),
    ).toEqual({
      prepare: [],
      processes: [
        { name: '', command: 'npm run dev', dir: '', env: {}, url: '' },
        { name: '', command: 'npm start', dir: '', env: { PORT: '4112', OK: 'true', NAME: 'x' }, url: '' },
      ],
      open: '',
    });
    // A folder that could leave the worktree, or a step without a command: the recipe is dropped, the criteria stay.
    for (const dir of ['../x', 'web/../../x', '/etc', '\\\\srv', 'C:\\\\x', '..\\\\x']) {
      expect(recipe(`{"processus": [{"nom": "x", "commande": "y", "dossier": "${dir}"}]}`), dir).toBeNull();
      expect(recipe(`{"preparation": [{"commande": "y", "dir": "${dir}"}]}`), dir).toBeNull();
    }
    expect(recipe('{"processus": [{"nom": "x", "commande": "  "}]}')).toBeNull();
    expect(recipe('{"preparation": [{"dossier": "web"}]}')).toBeNull();
    expect(recipe('{}')).toBeNull();
    expect(recipe('{"processus": []}')).toBeNull();
    // Not what a recipe is made of.
    for (const bad of [
      'null',
      '"npm start"',
      '[]',
      '{"processus": "npm start"}',
      '{"processus": [null]}',
      '{"processus": [{"commande": 5}]}',
      '{"processus": [{"commande": "a", "env": "x"}]}',
    ]) {
      expect(recipe(bad), bad).toBeNull();
    }
    expect(
      parseReport('{"criteres": [{"n": 1, "ok": true}], "lancement": {"processus": [{"commande": "../x", "dossier": "../x"}]}}')?.criteria,
    ).toEqual([{ n: 1, ok: true, note: '' }]);
  });

  describe('progress', () => {
    const progress = (json: string) => parseReport(json)?.progress;

    it('is a list of short lines read from avancement, or progress', () => {
      const want = ['A', 'B', 'C suite'];
      expect(progress('{"criteres": [], "avancement": ["A", "  B  ", "", 3, "C\\nsuite"]}')).toEqual(want);
      expect(progress('{"progress": ["A", "  B  ", "", 3, "C\\nsuite"]}')).toEqual(want);
      // A key that is there wins, even null (the backend reads it so).
      expect(progress('{"criteres": [], "avancement": null, "progress": ["A"]}')).toBeNull();
      expect(progress('{"avancement": ["A"], "progress": ["B"]}')).toEqual(['A']);
      // No key, an empty list, or one without a usable item: nothing said.
      expect(progress('{"criteres": [{"n": 1, "ok": true}]}')).toBeNull();
      expect(progress('{"criteres": []}')).toBeNull();
      expect(progress('{"avancement": [], "criteres": []}')).toBeNull();
      for (const junk of ['["", 1, null, " \\t "]', '[3, true, {"a": "b"}, ["x"]]', '["\\u0000\\u001b", "\\n"]']) {
        expect(progress(`{"criteres": [], "avancement": ${junk}}`), junk).toBeNull();
      }
      // One clean line each: control characters dropped, whitespace collapsed.
      expect(progress('{"avancement": ["a\\u0000b\\u0007", "\\u0000\\u001b", " \\t ", "x\\r\\n  y\\tz", "p\\u00a0q\\u2003r"]}')).toEqual([
        'ab',
        'x y z',
        'p q r',
      ]);
    });

    it('keeps at most the first 8 items, each cut at 120 bytes', () => {
      const items = Array.from({ length: 10 }, (_, i) => `fonction ${i + 1}`);
      expect(progress(JSON.stringify({ avancement: items }))).toEqual(items.slice(0, 8));
      // The unusable ones do not take a place.
      expect(progress('{"avancement": ["", 1, null, "a", "b", "c", "d", "e", "f", "g", "h", "i"]}')).toEqual([
        'a',
        'b',
        'c',
        'd',
        'e',
        'f',
        'g',
        'h',
      ]);
      // A long item is cut at 120 bytes with an ellipsis, on a character boundary ("é" takes two bytes).
      expect(progress(JSON.stringify({ avancement: ['x'.repeat(300)] }))).toEqual([`${'x'.repeat(120)}…`]);
      expect(progress(JSON.stringify({ avancement: ['é'.repeat(300), `ab${'é'.repeat(100)}`, '😀'.repeat(50)] }))).toEqual([
        `${'é'.repeat(60)}…`,
        `ab${'é'.repeat(59)}…`,
        `${'😀'.repeat(30)}…`,
      ]);
      // One of exactly 120 bytes is whole.
      expect(progress(JSON.stringify({ avancement: ['y'.repeat(120), 'é'.repeat(60)] }))).toEqual(['y'.repeat(120), 'é'.repeat(60)]);
    });

    it('is none when it is not a list, without costing the criteria or the recipe', () => {
      for (const bad of ['"texte"', 'null', '42', '{"a": "b"}', 'true']) {
        const r = parseReport(
          `{"criteres": [{"n": 1, "ok": true, "note": "vu"}], "avancement": ${bad}, "lancement": {"processus": [{"nom": "web", "commande": "npm run dev"}]}}`,
        );
        expect(r?.progress, bad).toBeNull();
        expect(r?.criteria, bad).toEqual([{ n: 1, ok: true, note: 'vu' }]);
        expect(r?.recipe?.processes[0].command, bad).toBe('npm run dev');
      }
      // And the other way: a list is a report by itself.
      expect(parseReport('{"avancement": ["a"]}')).toEqual({ criteria: null, recipe: null, progress: ['a'] });
      expect(parseReport('{"avancement": "texte"}')).toBeNull();
    });
  });

  it('splits a message around its readable blocks', () => {
    const text = 'Fait.\n\n```escouade\n{"criteres": []}\n```\n\nSuite.';
    expect(splitEscouade(text).map((s) => s.kind)).toEqual(['md', 'report', 'md']);
    expect(splitEscouade(text)[0]).toEqual({ kind: 'md', text: 'Fait.\n\n' });
    // An unreadable block stays a code block; a message without one stays whole.
    const broken = 'A\n```escouade\n{oups\n```';
    expect(splitEscouade(broken)).toEqual([{ kind: 'md', text: broken }]);
    expect(splitEscouade('Bonjour')).toEqual([{ kind: 'md', text: 'Bonjour' }]);
  });

  it('keeps the words around an unreadable block and cuts around each readable one', () => {
    const text =
      'A\n```escouade\n{oups\n```\nB\n```escouade\n{"avancement": ["x"]}\n```\nC\n```escouade\n{"criteres": [{"n": 1, "ok": true}]}\n```';
    const parts = splitEscouade(text);
    expect(parts.map((s) => s.kind)).toEqual(['md', 'report', 'md', 'report']);
    expect(parts[0]).toEqual({ kind: 'md', text: 'A\n```escouade\n{oups\n```\nB\n' });
    expect(parts[1]).toMatchObject({ report: { progress: ['x'] } });
    expect(parts[2]).toEqual({ kind: 'md', text: '\nC\n' });
    // A block still being written (no closing fence) is not one yet.
    expect(splitEscouade('Voilà\n```escouade\n{"criteres": [')).toEqual([{ kind: 'md', text: 'Voilà\n```escouade\n{"criteres": [' }]);
  });

  it('still finds the real block after a mention of the fence in the text', () => {
    const valid = '{"criteres": [{"n": 1, "ok": true}], "avancement": ["Fichier écrit"]}';
    const text = `Voici le bilan, dans un bloc \`\`\`escouade comme demandé.\n\n\`\`\`escouade\n${valid}\n\`\`\``;
    const parts = splitEscouade(text);
    expect(parts.map((s) => s.kind)).toEqual(['md', 'report']);
    expect(parts[0]).toEqual({ kind: 'md', text: 'Voici le bilan, dans un bloc ```escouade comme demandé.\n\n' });
    expect(parts[1]).toMatchObject({ report: { criteria: [{ n: 1, ok: true }], progress: ['Fichier écrit'] } });
  });
});

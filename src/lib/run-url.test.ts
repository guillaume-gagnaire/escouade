import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { expandRunUrl, portVars, runUrlProblem } from './run-url';

describe('the address of a launch command (ESC-560)', () => {
  beforeEach(() => setLang('fr'));

  it('follows the ports of a worktree, or takes its value for the project', () => {
    const url = 'http://localhost:${ESCOUADE_PORT_BASE:-3000}/admin?to=${ESCOUADE_PORT_END}';
    expect(expandRunUrl(url, portVars(4120))).toEqual({ url: 'http://localhost:4120/admin?to=4129' });
    expect(expandRunUrl('http://localhost:${ESCOUADE_PORT_BASE:-3000}/', portVars(null))).toEqual({ url: 'http://localhost:3000/' });
    expect(expandRunUrl(' http://localhost:5173 ', {})).toEqual({ url: 'http://localhost:5173' });
    // In the project, a port without a value: said, with how to give it one.
    expect(expandRunUrl('http://localhost:${ESCOUADE_PORT_BASE}/', {}).error).toBe(
      'L’adresse nomme ESCOUADE_PORT_BASE, qu’un lancement dans le projet n’a pas : donne-lui une valeur pour le projet (${ESCOUADE_PORT_BASE:-3000}).',
    );
    expect(expandRunUrl('http://${HOME}/', portVars(4120)).error).toContain('seulement ESCOUADE_PORT_BASE, ESCOUADE_PORT_END');
    expect(expandRunUrl('http://localhost:${ESCOUADE_PORT_BASE', portVars(4120)).error).toBe('Une variable de l’adresse n’est pas fermée.');
  });

  it('keeps an http or https address with a host, and says what is wrong with any other', () => {
    for (const ok of ['', '  ', 'http://localhost:3000', 'HTTPS://app.test/connexion', 'http://localhost:${ESCOUADE_PORT_BASE}/'])
      expect(runUrlProblem(ok), ok).toBeNull();
    for (const bad of ['file:///C:/Windows/win.ini', 'javascript:alert(1)', 'ftp://localhost/', 'localhost:3000'])
      expect(runUrlProblem(bad), bad).toBe('Seule une adresse http:// ou https:// s’ouvre.');
    expect(runUrlProblem('pas une adresse')).toBe('Ce n’est pas une adresse (http://localhost:3000, par exemple).');
    setLang('en');
    expect(runUrlProblem('ftp://x/')).toBe('Only an http:// or https:// address opens.');
  });
});

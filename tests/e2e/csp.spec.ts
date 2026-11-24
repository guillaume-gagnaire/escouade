// The content security policy the real window has (audit SEC-05, ESC-593): a `<style>` element applies only with the
// nonce Tauri gives the page at each load, `style` attributes still apply, and none of the interface's own styles is
// refused (its sheet, xterm's, the VS Code view's). The policy is read from the window itself (the violation it reports) and
// from what Tauri serves for the main window (the « Pages » windows' bar is gone, ESC-657). A
// launch's page is the developed app's, under its own policy; a plugin's view is drawn in the main window.

import type { Page } from '@playwright/test';
import { addProject, expect, test } from './fixture';
import { editorLines, openInExplorer, workbench } from './vscode';

/** A policy's directives, by name. */
function directives(policy: string): Map<string, string[]> {
  return new Map(
    policy
      .split(';')
      .map((d) => d.trim())
      .filter(Boolean)
      .map((d) => {
        const [name, ...values] = d.split(/\s+/);
        return [name!.toLowerCase(), values] as const;
      }),
  );
}

/** What the window's policy must say of styles, `nonce` being the page's. */
function expectStylePolicy(policy: string, nonce: string) {
  const d = directives(policy);
  const styles = d.get('style-src')!;
  expect(styles, policy).toContain("'self'");
  expect(styles, policy).toContain(`'nonce-${nonce}'`);
  expect(styles, policy).not.toContain("'unsafe-inline'");
  expect(d.get('style-src-attr'), policy).toEqual(["'unsafe-inline'"]);
  expect(d.has('style-src-elem'), policy).toBe(false);
  for (const name of ['object-src', 'base-uri', 'form-action', 'frame-ancestors'])
    expect(d.get(name), `${name}: ${policy}`).toEqual(["'none'"]);
  expect(d.get('script-src'), policy).not.toContain("'unsafe-inline'");
}

/** The `<style>` elements of the page whose sheet the policy refused, and those that carry no nonce. */
const refusedStyles = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll('style')]
      .filter((s) => !s.sheet || !s.nonce)
      .map((s) => `${s.id || s.className || s.parentElement?.className || '?'}: ${s.sheet ? 'no nonce' : 'refused'}`),
  );

test('the window applies a <style> only with its nonce, keeps style attributes, and says so in its real policy', async ({ app }) => {
  const { page } = app;
  await expect(page.locator('#app')).not.toBeEmpty();
  const got = await page.evaluate(async () => {
    const violations: { directive: string; policy: string }[] = [];
    document.addEventListener('securitypolicyviolation', (e) =>
      violations.push({ directive: e.effectiveDirective, policy: e.originalPolicy }),
    );
    const color = (id: string) => getComputedStyle(document.getElementById(id)!).color;
    // Markup as an agent's message would bring it, past the sanitizer: its `<style>` keeps no nonce.
    const box = document.createElement('div');
    box.innerHTML =
      '<style>#csp-a { color: rgb(1, 2, 3) }</style><span id="csp-a">a</span>' +
      '<svg><style>#csp-s { color: rgb(1, 2, 3) }</style></svg><span id="csp-s">s</span>' +
      '<span id="csp-b" style="color: rgb(4, 5, 6)">b</span>';
    document.body.appendChild(box);
    // A `<style>` the interface's code creates: given the nonce (lib/style-nonce.ts).
    const own = document.createElement('style');
    own.textContent = '#csp-c { color: rgb(7, 8, 9) }';
    document.head.appendChild(own);
    const c = document.createElement('span');
    c.id = 'csp-c';
    box.appendChild(c);
    // A `<style>` made by the code but without the nonce: refused all the same.
    const bare = document.createElement('style');
    bare.removeAttribute('nonce');
    bare.nonce = '';
    bare.textContent = '#csp-d { color: rgb(1, 2, 3) }';
    document.head.appendChild(bare);
    const d = document.createElement('span');
    d.id = 'csp-d';
    box.appendChild(d);
    await new Promise((r) => setTimeout(r, 300));
    const out = {
      a: color('csp-a'),
      s: color('csp-s'),
      b: color('csp-b'),
      c: color('csp-c'),
      d: color('csp-d'),
      violations,
      nonce: (document.getElementById('escouade-style-nonce') as HTMLStyleElement).nonce,
      ownNonce: own.nonce,
    };
    box.remove();
    own.remove();
    bare.remove();
    return out;
  });
  expect(got.nonce).toMatch(/^\d+$/);
  expect(got.ownNonce).toBe(got.nonce);
  expect(got.a).not.toBe('rgb(1, 2, 3)');
  expect(got.s).not.toBe('rgb(1, 2, 3)');
  expect(got.d).not.toBe('rgb(1, 2, 3)');
  expect(got.b).toBe('rgb(4, 5, 6)');
  expect(got.c).toBe('rgb(7, 8, 9)');
  // Each refused `<style>` reported, against the policy the window really has.
  const refused = got.violations.filter((v) => v.directive === 'style-src-elem');
  expect(refused.length).toBeGreaterThanOrEqual(3);
  expect(got.violations.filter((v) => v.directive !== 'style-src-elem')).toEqual([]);
  expectStylePolicy(refused[0]!.policy, got.nonce);
});

test('Tauri serves the main window with a nonce of its own, on its <style>', async ({ app }) => {
  const { page } = app;
  await expect(page.locator('#app')).not.toBeEmpty();
  // The « Pages » windows' bar, served with it before, is gone (ESC-657).
  for (const address of ['/index.html']) {
    const served = await page.evaluate(async (address) => {
      const res = await fetch(address);
      return { policy: res.headers.get('content-security-policy'), html: await res.text() };
    }, address);
    // On Windows and macOS the policy comes with the page; under Linux, in it.
    const policy = served.policy ?? served.html.match(/http-equiv="Content-Security-Policy" content="([^"]+)"/)?.[1] ?? '';
    const nonce = served.html.match(/<style id="escouade-style-nonce" nonce="(\d+)"/)?.[1];
    expect(nonce, `${address}: ${served.html.slice(0, 800)}`).toBeTruthy();
    expectStylePolicy(policy, nonce!);
  }
});

test('no style of the interface is refused: the window, a terminal, the VS Code view', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  expect(await refusedStyles(page)).toEqual([]);
  // A terminal: xterm writes its scrollbar's colours in a `<style>` element of its own.
  await page.getByRole('button', { name: 'Nouveau terminal' }).click();
  await page.getByRole('menuitem', { name: /PowerShell/ }).click();
  await expect(page.locator('main.tv .xterm')).toBeVisible();
  await expect
    .poll(() => page.evaluate(() => [...document.querySelectorAll('style')].some((s) => s.textContent?.includes('.xterm'))))
    .toBe(true);
  expect(await refusedStyles(page)).toEqual([]);
  // The editor of the 2.3, the VS Code view (ESC-651): Monaco writes its theme's colours in `<style>` elements of its
  // own, as it goes.
  await page.getByRole('button', { name: 'Parcourir' }).click();
  await openInExplorer(page, 'src/app.ts');
  await expect(editorLines(page)).toContainText('const a = 1;');
  // Its theme applied: the editor's background is a colour its sheet alone gives.
  await expect(workbench(page).locator('.monaco-editor-background').first()).not.toHaveCSS('background-color', 'rgba(0, 0, 0, 0)');
  expect(await refusedStyles(page)).toEqual([]);
});

// Ce que le catalogue des textes d'Escouade attend d'un navigateur, et que React Native n'a pas. Importé en premier
// par index.ts, avant tout module qui lit le catalogue.
// - `$state`, une rune de Svelte : hors de Svelte, une valeur simple suffit ;
// - `navigator.language` (locale.svelte.ts en tire la langue de départ) : React Native a un `navigator` sans langue.
(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v;
const nav = (globalThis as unknown as { navigator?: { language?: string } }).navigator;
if (nav && !nav.language) {
  try {
    Object.defineProperty(nav, 'language', { value: 'fr-FR', configurable: true });
  } catch {
    /* le catalogue retombera sur sa langue par défaut */
  }
}

// Le catalogue des textes d'Escouade lit `$state`, une rune de Svelte : hors de Svelte, une valeur simple suffit.
// Importé en premier par index.ts, avant tout module qui lit le catalogue.
(globalThis as unknown as { $state: <T>(v: T) => T }).$state = (v) => v;

# ESC-66 — la relecture d'une PR par un agent

`apercu.html` (rendu par `node design/esc-66/shot.mjs` : `apercu.png`, `apercu-clair.png`) est l'aperçu fait avant le code, avec les jetons de `src/app.css`. `captures/` vient de la vraie app, sur le faux GitHub et le faux claude : `ESCOUADE_E2E_SHOTS=$PWD/design/esc-66/captures npx playwright test tests/e2e/prs-review.spec.ts`.

- **Une vue, pas une fenêtre** : trancher vingt commentaires prend du temps, et les agents continuent d'appeler pendant ce temps. La relecture prend la place de la liste, avec le même en-tête.
- **Une colonne de 860 px** : on lit de haut en bas, le texte puis chaque fichier ; chaque commentaire porte ses lignes du diff, la sienne encadrée.
- **Le pied reste à l'écran** : il dit ce qui reste à trancher, le verdict et « Publier sur GitHub », éteint tant qu'un commentaire n'est pas tranché.

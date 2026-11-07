# Les captures du site

Les 15 images du site (`website/public/images/*.jpg`, et `images/en/` pour l'anglais) sont prises dans la vraie app 2.0, en une commande :

```sh
npx tauri build --debug --no-bundle   # l'app de debug, comme pour les e2e
npm run site-shots                    # les 30 images, en français puis en anglais
```

À relancer après un changement d'interface : les images changent, `git status` les montre.

## Ce que fait le script

- **L'outillage des e2e** (`tests/e2e/fixture.ts`) : l'app de debug, un dossier de données jetable, les faux harness (Claude Code, Codex, Cursor CLI), la fenêtre cachée, la capture par CDP. Rien ne s'ouvre sur la machine : les ouvertures de navigateur et de dossiers sont interceptées.
- **Des données de démo fictives** (`demo.ts`, `repos.ts`) : les projets `demo-api`, `studio-web` (deux dépôts), `mobile-app` et `infra` avec leurs tuiles (lettres, logos, libellés), des agents sur trois harness, des tickets dans toutes les colonnes, deux semaines de statistiques, un Jira de démo (`tests/fixtures/fake-trackers.mjs`). Les agents au travail jouent des tours écrits d'avance (`$FAKE_CLAUDE_DEMO`, lus par `tests/fixtures/fake-claude.mjs`).
- **Rien de réel** : tout est sous `C:\escouade-demo` (`ESCOUADE_E2E_ROOT`, vidé au départ, gardé à la fin pour y regarder), le dossier temporaire sous `C:\escouade-demo-tmp` ; aucun compte, e-mail ni jeton réel ; la fenêtre de l'app est capturée seule, jamais l'écran.
- **Le format du site** (`website/app/data/shots.ts`) : la fenêtre à 1500 × 844, posée à (210, 70) sur un fond de 1920 × 960, en JPEG qualité 85, aux noms de `FEATURE_SHOTS` (`website/app/data/site.ts`). Le site recadre la fenêtre pour le haut de page et zoome sur chaque fonctionnalité (`FOCUS`).

## Options

- `npm run site-shots -- --project=fr` (ou `en`) : une langue seulement.
- `npm run site-shots -- --grep "code hosts"` : les 4 images des hébergeurs seulement (`prs`, `pr-review`, `board-review`, `environments`), contre le faux GitHub de `tests/fixtures/fake-trackers.mjs`, sur une démo à part : un ticket « En revue » et deux tickets mergés aux commits des environnements.
- `ESCOUADE_SHOTS_OUT=C:\tmp\shots` : les images ailleurs que dans le site, pour les regarder d'abord ; en cas d'échec, `test-results/…/failed-<langue>.png` montre la fenêtre.
- `ESCOUADE_E2E_ROOT` : un autre dossier de démo, à un chemin qui ne dit rien de la machine.

L'affiche et les sous-titres de la vidéo viennent toujours de `video/` (`npm run site-images`).

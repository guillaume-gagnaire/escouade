# La documentation d’Escouade

Les pages de `/docs` (français) et `/en/docs` (anglais) du site. Une page est un fichier Markdown par langue, de même nom : `fr/<slug>.md` et `en/<slug>.md` ; les pages d’une section sont dans son dossier (`fr/plugins/manifest.md`, de slug `plugins/manifest`). Leur ordre, leurs groupes et leurs adresses sont dans `app/data/docs.ts` ; une page qui n’y est pas n’est pas publiée, et une page qui y est doit exister dans les deux langues (les tests le vérifient).

## Forme d’une page

```markdown
# Titre de la page

Un seul paragraphe d’introduction, une ou deux phrases, sans lien ni mise en forme : ce que la page couvre. Il s’affiche en chapeau sous le titre et sert de description aux moteurs de recherche.

## Une section

Du texte, des listes, des tableaux.

### Une sous-section
```

- Un seul titre `#`, en première ligne ; puis des `##` et des `###`, jamais plus bas. Le sommaire de droite (« Sur cette page ») liste les `##` et les `###`.
- Pas de HTML : le Markdown suffit (GFM : tableaux, listes, blocs de code).
- Les deux langues ont les mêmes sections dans le même ordre, les mêmes liens et les mêmes images.

## Liens

- Vers une autre page de la doc : son slug seul, `[le pilote auto](autopilot)` ou `[le manifeste](plugins/manifest)`, ou avec une section, `[les modes de permission](managing-agents#modes-de-permission)`. L’ancre d’une section est son titre en minuscules, sans accents, chaque suite de caractères qui ne sont ni lettres ni chiffres remplacée par un `-` (« Modes de permission » → `modes-de-permission`, « Données locales » → `donnees-locales`). Le site en fait de vraies adresses ; un lien vers une page ou une section qui n’existe pas fait échouer les tests.
- Vers l’extérieur : l’adresse complète (`https://…`).
- Vers la page « Offrir un café » : `[offrir un café](support:)`, que le site remplace par celle de la langue de la page (`SUPPORT` de `app/data/site.ts`).

## Images

Les captures du site, prises dans la vidéo de présentation, dans la langue de la page : `![Ce que montre la capture](/images/board.jpg)` en français, `![What it shows](/images/en/board.jpg)` en anglais. Il y a `agents`, `board`, `board-review`, `chat`, `editor`, `environments`, `git`, `integrations`, `launch`, `notifications`, `pr-review`, `prs`, `remote`, `stats` et `test` (`.jpg`). Le texte entre crochets est lu par les lecteurs d’écran : il dit ce que montre l’image.

## Encadrés

Un bloc de citation dont la première ligne est `[!NOTE]`, `[!TIP]` ou `[!WARNING]` devient un encadré (« À noter », « Astuce », « Attention ») :

```markdown
> [!TIP]
> `Ctrl+J` ouvre le prochain agent qui attend ta réponse.
```

## Style

- **Français :** tutoiement, phrases simples et concrètes, sans emphase ni superlatifs. Apostrophe typographique `’`, guillemets `« »` avec une espace insécable (U+00A0) de chaque côté du texte, points de suspension `…`. Les libellés de l’interface s’écrivent tels qu’ils sont, entre guillemets : « Pilote auto ».
- **Anglais :** même ton, « you », impératif pour les actions, orthographe américaine. Apostrophe `’`, guillemets “ ”. Les libellés de l’interface entre guillemets : “Autopilot”.
- Les mots de l’app et leur traduction sont fixés par `src/lib/i18n/GLOSSARY.md` (à la racine du dépôt) : « À tester » est “To review”, un ticket « validé » est “approved”, “autopilot” en un mot.
- Raccourcis, commandes, chemins et noms de fichiers en code : `Ctrl+K`, `~/.escouade/settings.json`. Les touches s’écrivent comme l’app les affiche dans la langue de la page : `Ctrl+Maj+F`, `Entrée`, `Échap` en français, `Ctrl+Shift+F`, `Enter`, `Esc` en anglais. Sous macOS, `Ctrl` devient `⌘` (sauf `Ctrl+Tab`) : le dire une fois là où c’est utile, la page des raccourcis détaille.
- Les apostrophes et guillemets droits restent dans le code (blocs et `code`), jamais dans le texte.

## Exactitude

La doc dit ce que fait l’app, rien d’autre. Chaque libellé vient des catalogues de l’app (`src/lib/i18n/fr/*.ts`, `src/lib/i18n/en/*.ts`), chaque comportement et chaque valeur par défaut du code (`src/`, `src-tauri/src/`). `README.md` et `CHANGELOG.md` aident à s’y retrouver, mais le code a raison. Ce qui sert à développer Escouade (build, tests, vidéo, site) reste dans le `README.md`.

Dans `plugins/`, un bloc de TypeScript ou de Rust vient toujours d’un fichier compilé, cité sous son chemin : les deux plugins de `sdk/doc-examples/`, un modèle de `sdk/create-escouade-plugin/templates/` ou un plugin de `plugins/`. Le morceau d’un manifeste doit passer `validate`. Les tests le tiennent (`sdk/doc-examples/doc-examples.test.ts`, `tests/docs.test.ts`).

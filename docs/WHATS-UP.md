# What's up ? : les nouveautés d'une version

Une fenêtre qui s'ouvre **une fois** chez chaque utilisateur après une mise à jour d'Escouade : 1 à 5 écrans, chacun un geste nouveau montré en captures de la vraie interface. Le composant vient du plugin Claude Code `osho-whats-up` d'OshOEz (PR 51) ; ce dépôt garde le composant, les fiches, l'atelier et un script de capture.

**La fiche est facultative.** Elle n'est due que pour une version qui apporte des nouveautés visibles ; une version de correctifs part sans fiche. Aucun hook n'est exigé.

## Ce qu'il y a dans le dépôt

| Chemin | Rôle |
| --- | --- |
| `public/whats-up/whats-up.js` | Le composant `<whats-up>` (un fichier, sans dépendance, Shadow DOM). Copié tel quel du plugin : sa première ligne dit sa version ; hors Prettier. On ne le modifie pas. |
| `public/whats-up/index.json` | La liste des fiches, de la plus ancienne à la plus récente. **Trois au plus** : leurs captures entrent dans le binaire de l'app (un test de `src/lib/whats-up.test.ts` le tient). |
| `public/whats-up/<id>.json`, `<id>.css`, `<id>-<état>[-sombre].webp` | Une fiche en français : ses écrans (texte et scène), leurs styles, ses captures en clair et en sombre. `<id>` est la date de la fiche (`AAAA-MM-JJ`, puis `AAAA-MM-JJ-2` pour une deuxième le même jour). |
| `public/whats-up/en/` | Les mêmes fiches en anglais : son propre `index.json` (les mêmes ids), les textes traduits et les captures de l'interface en anglais. |
| `index.html` | Le `<whats-up>` dans un `<template>`, après `main.ts` (son `<style>` reçoit le nonce de la page). Le thème vient des variables de `src/app.css` ; `theme-sombre` choisit les captures sombres. |
| `src/lib/whats-up.ts` | Ce qui compte comme vu au démarrage, et la fenêtre mise dans la page dans la langue de l'interface (voir plus bas). |
| `.whatsup/atelier/` | L'atelier : la vraie `App.svelte` dans un navigateur, sur le faux backend des tests (`src/test/ipc.ts`), avec un projet et des agents fictifs. Hors du build. |
| `.whatsup/capturer.mjs` | Les captures d'une fiche, depuis l'atelier, avec Playwright. |
| `.whatsup/config.json`, `garde.py`, `pre-push.sh`, `installer-hook.sh` | La configuration du plugin et son garde `pre-push`, facultatifs. |

## Ce que voit l'utilisateur

Ce qu'il a vu est gardé dans le `localStorage` de la fenêtre (`whats-up:Escouade`). `src/lib/whats-up.ts`, au démarrage, sans rien lire sur le réseau (les ids des fiches sont connus dès le build) :

- **Premier lancement** (ni thème ni fiche vue gardés) : les fiches passées ne sont pas des nouveautés. La plus récente est enregistrée comme vue tout de suite ; un rechargement aussitôt après reste fermé aussi (chaque e2e, profil neuf, reste sans fenêtre).
- **Versions sautées** (plusieurs fiches plus récentes que la dernière vue) : seule la plus récente s'ouvre. L'avant-dernière est donnée au composant comme vue (`vu`) ; en fermant, il enregistre la dernière.
- **La langue** : la fenêtre n'entre dans la page qu'une fois la langue des réglages connue. En français, `src="/whats-up/"` ; en anglais, `src="/whats-up/en/"` et `lang="en"` (les mots du composant : « What's new », « Next »…). Le composant lit ses attributs en se connectant, d'où le `<template>`.

Dans l'app, `document.querySelector('whats-up').ouvrir()` rouvre la dernière fiche ; dans un navigateur (`npx vite`), `?whats-up=apercu` la montre sans rien retenir, dans la langue du navigateur.

## Faire une fiche (atelier et Playwright)

1. **Choisir** au plus 5 nouveautés visibles de la version, dans le CHANGELOG sous « Non publié ». Les navigateurs d'abord quand la version en apporte.
2. **Un état par capture** dans `.whatsup/atelier/etats.ts` : les données fictives (projet « Atelier », agents `front-1`, `testeur`, `rédacteur`) et les gestes faits une fois la fenêtre dessinée (`apres` : un clic, un événement du backend, une page de `.whatsup/atelier/pages/` dessinée là où serait la webview). Les mots fictifs passent par `L('français', 'english')`, les sélecteurs par `t(...)`, pour que le même état serve aux deux langues. Jamais un vrai compte, un vrai site ni un vrai nom.
3. **Lancer l'atelier**, puis le regarder à la main si besoin :

   ```sh
   npx vite --config .whatsup/atelier/vite.config.ts --port <port>
   ```

   `http://127.0.0.1:<port>/.whatsup/atelier/atelier.html?etat=<nom>&theme=clair|sombre&lang=fr|en`. La page met `window.__whatsupPret = true` une fois l'écran dessiné.

4. **Capturer** chaque état en clair et en sombre, à 1440 × 900, dans les deux langues :

   ```sh
   node .whatsup/capturer.mjs --url http://127.0.0.1:<port> --id <id> --lang fr <état> <état>…
   node .whatsup/capturer.mjs --url http://127.0.0.1:<port> --id <id> --lang en <état> <état>…
   ```

   Il écrit `public/whats-up/<id>-<état>.webp` et `<id>-<état>-sombre.webp` (`public/whats-up/en/` pour l'anglais). Relis chaque image.

5. **Écrire la fiche** : `<id>.json` (chaque écran : `etiquette`, `titre`, `texte`, `scene`) et `<id>.css` (la caméra de chaque scène : le cadre de 1440 × 900 réduit, puis zoomé sur le geste ; ses classes préfixées `wu-<id sans tirets>-`), en français à la racine et en anglais dans `en/`. Le plus simple : partir de la fiche précédente.
6. **Ajouter l'id** à la fin des deux `index.json`, et **retirer la plus ancienne** fiche (son `.json`, son `.css`, ses captures, dans les deux langues) s'il y en a plus de trois.
7. **Regarder l'aperçu** : `npx vite --port <port>` puis `http://localhost:<port>/?whats-up=apercu`, à 1440 et à 390 px de large, en clair et en sombre (thème `escouade-theme` du `localStorage`), en français et en anglais (la langue du navigateur).
8. `npx vitest run src/lib/whats-up.test.ts` : les deux langues, les captures nommées par les scènes, trois fiches au plus.
9. La fiche part sur `main` avec la version, avant son tag. Une fiche déjà publiée ne se réécrit pas (les utilisateurs l'ont vue) : on en fait une nouvelle.

## Avec le plugin de l'auteur (possible, pas exigé)

Le plugin `osho-whats-up` fait les étapes 1 à 7 en conversation (« what's up » ou `/whats-up` dans Claude Code) : il lit les commits depuis le dernier tag `v*`, fait cocher les nouveautés, capture dans l'atelier et rédige la fiche. C'est un plugin tiers : l'installer reste le choix de chacun.

```sh
claude plugin marketplace add OshOEz/OshO-Skillz
claude plugin install osho-whats-up@osho-skillz
```

Il rédige en une langue (`langue` de `.whatsup/config.json`) : la version anglaise se fait ensuite comme plus haut (états, `--lang en`, `en/<id>.json`).

Son garde `pre-push` (`sh .whatsup/installer-hook.sh`, une fois par clone) refuse le push d'un tag `v*` sans fiche plus récente qu'au tag précédent. Il est facultatif ; avec lui, une version sans nouveauté visible part avec sa raison :

```sh
WHATSUP_SANS="correctifs seulement" git push origin vX.Y.Z
```

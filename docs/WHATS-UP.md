# What's up ? : les nouveautés d'une version

Une fenêtre qui s'ouvre **une fois** chez chaque utilisateur après une mise à jour d'Escouade : 1 à 5 écrans, chacun un geste nouveau montré en captures de la vraie interface. Elle vient du plugin Claude Code `osho-whats-up` ; ce dépôt n'en garde que le composant, les fiches et le garde.

## Ce qu'il y a dans le dépôt

| Chemin | Rôle |
| --- | --- |
| `public/whats-up/whats-up.js` | Le composant `<whats-up>` (un fichier, sans dépendance, Shadow DOM). Copié tel quel du plugin : sa première ligne dit sa version ; hors Prettier. |
| `public/whats-up/index.json` | La liste des fiches, de la plus ancienne à la plus récente. |
| `public/whats-up/<id>.json`, `<id>.css`, `<id>-*.webp` | Une fiche : ses écrans (texte et scène), leurs styles, ses captures en clair et en sombre. `<id>` est la date de la version (`AAAA-MM-JJ`). |
| `index.html` | Le montage, après `main.ts` (son `<style>` reçoit le nonce de la page). Le thème vient des variables de `src/app.css` ; `theme-sombre` choisit les captures sombres. |
| `src/lib/whats-up.ts` | Au tout premier lancement de la fenêtre, rien n'est montré : les fiches passées ne sont pas des nouveautés (et chaque e2e, profil neuf, reste sans fenêtre). |
| `.whatsup/config.json` | `prod` = `tag:v*` (une version part d'un tag), `base` = `main`, le thème, l'atelier. |
| `.whatsup/garde.py`, `pre-push.sh`, `installer-hook.sh` | Le garde du hook `pre-push` et son installeur. |
| `.whatsup/atelier/` | L'atelier : la vraie `App.svelte` dans un navigateur, sur le faux backend des tests (`src/test/ipc.ts`), avec un projet et des agents fictifs. Hors du build. |

Ce que l'utilisateur a vu est gardé dans le `localStorage` de la fenêtre (`whats-up:Escouade`) : une fiche s'ouvre si elle est plus récente que la dernière vue. Dans l'app, `document.querySelector('whats-up').ouvrir()` la rouvre ; dans un navigateur, `?whats-up=apercu` la montre sans rien retenir.

## Installer (une fois par développeur)

Le plugin, qui prépare les fiches :

```sh
claude plugin marketplace add OshOEz/OshO-Skillz
claude plugin install osho-whats-up@osho-skillz
```

Le garde, dans le hook `pre-push` de ton clone (sous Windows, depuis Git Bash) :

```sh
sh .whatsup/installer-hook.sh
```

Il ne remplace jamais un hook : son bloc se place juste après le shebang du `pre-push` existant, et le relancer remplace ce bloc. Avec `core.hooksPath` réglé (husky, lefthook…), il le dit et ne touche à rien : place à la main le bloc de `.whatsup/pre-push.sh` après le shebang de ce hook-là. Sans `python3`, le garde laisse passer.

## Préparer une version

1. Dans Claude Code, sur ce dépôt : « what's up » (ou `/whats-up`). Il lit les commits depuis le dernier tag `v*`, fait cocher au plus 5 nouveautés sur une page, lance l'atelier, y capture chaque état en clair et en sombre, rédige la fiche dans un worktree et montre l'aperçu, jusqu'à ton accord.
2. La fiche part sur `main` par une PR, avant le tag.
3. Le tag `vX.Y.Z` posé et poussé (README, « Publier une version ») : le garde laisse passer.

Le garde ne juge que le push d'un tag `v*` : il le refuse (« Pas de fiche What's up ? ») si `public/whats-up/index.json` n'a pas de fiche plus récente qu'au tag précédent. Les pushs de branches passent toujours. Une version sans nouveauté visible part avec sa raison :

```sh
WHATSUP_SANS="correctifs seulement" git push origin vX.Y.Z
```

Une fiche déjà publiée ne se réécrit pas (les utilisateurs l'ont vue) : on en fait une nouvelle.

## L'atelier à la main

```sh
npx vite --config .whatsup/atelier/vite.config.ts --port 5199
```

Puis `http://127.0.0.1:5199/.whatsup/atelier/atelier.html?etat=<nom>&theme=clair|sombre` ; les états sont dans `.whatsup/atelier/etats.ts`. La page met `window.__whatsupPret = true` une fois l'écran dessiné : c'est ce qu'attend la capture du plugin.

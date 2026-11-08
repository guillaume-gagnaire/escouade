# Les thèmes

Un plugin **déclaratif** peut donner à Escouade d’autres couleurs : des valeurs pour les jetons de l’interface, en sombre et en clair, et une palette de couleurs de projet. Il se choisit dans **Réglages › Général › Couleurs** et s’applique tout de suite, terminaux et éditeur compris.

## Un thème dans le manifeste

Le plugin « Contraste élevé », en entier : un thème, sombre et clair, et sa palette de projets.

`plugins/high-contrast/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.high-contrast",
  "name": { "fr": "Contraste élevé", "en": "High contrast" },
  "version": "1.1.2",
  "description": {
    "fr": "Un thème à fort contraste, clair et sombre : texte pur, bordures nettes, couleurs d'état franches.",
    "en": "A high-contrast theme, light and dark: pure text, sharp borders, plain status colors."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/high-contrast",
  "escouade": { "api": "1", "min": "2.1.0" },
  "contributes": {
    "themes": [
      {
        "id": "high-contrast",
        "name": { "fr": "Contraste élevé", "en": "High contrast" },
        "dark": {
          "--frame": "#000000",
          "--panel": "color-mix(in oklch, var(--p) 4%, #050505)",
          "--raise": "#141414",
          "--raise-2": "#1f1f1f",
          "--well": "#000000",
          "--line": "#8a8a8a",
          "--line-2": "#b0b0b0",
          "--line-3": "#d8d8d8",
          "--text": "#ffffff",
          "--soft": "#f0f0f0",
          "--muted": "#d0d0d0",
          "--dim": "#b8b8b8",
          "--p-ink": "#ffd400",
          "--ring": "#ffd400",
          "--run": "#4dff88",
          "--wait": "#ffb000",
          "--done": "#7ab8ff",
          "--add": "#4dff88",
          "--del": "#ff6b6b"
        },
        "light": {
          "--frame": "#ffffff",
          "--panel": "#ffffff",
          "--raise": "#f2f2f2",
          "--raise-2": "#e6e6e6",
          "--well": "#ffffff",
          "--line": "#5c5c5c",
          "--line-2": "#3d3d3d",
          "--line-3": "#1f1f1f",
          "--text": "#000000",
          "--soft": "#0a0a0a",
          "--muted": "#262626",
          "--dim": "#3d3d3d",
          "--p-ink": "#0040c0",
          "--ring": "#0040c0",
          "--run": "#006b2e",
          "--wait": "#8a4b00",
          "--done": "#0040c0",
          "--add": "#006b2e",
          "--del": "#b00020"
        },
        "projectColors": ["#ffd400", "#00e0ff", "#ff5cf0", "#7cff4f", "#ff8a00", "#b48cff"]
      }
    ]
  },
  "store": { "docs": "docs" }
}
```

Chaque entrée de `contributes.themes` est un thème :

- `id` : son identifiant dans le plugin (lettres, chiffres, `.`, `_`, `-`) ;
- `name` : son nom, en une langue ou par langue ; sans lui, son `id` ;
- `dark` et `light` : les valeurs des jetons, l’un, l’autre ou les deux ;
- `projectColors` : la palette proposée pour les projets (24 couleurs au plus), à la place de celle d’Escouade.

Un plugin peut donner plusieurs thèmes. Dans le menu « Couleurs », chacun s’appelle par son nom, suivi de celui du plugin quand ils diffèrent.

## Ce qu’un thème ne donne pas

**Un jeton manquant garde la valeur d’Escouade.** Un thème qui ne donne que `--frame` change le cadre de la fenêtre et rien d’autre ; un thème qui n’a que `dark` laisse le mode clair à Escouade. Le choix Système, Clair ou Sombre reste celui de l’utilisateur, dans la rangée « Thème » au-dessus.

`--p`, la couleur du projet ouvert, n’est pas un jeton de thème : le thème donne la palette dans laquelle l’utilisateur la choisit. Les surfaces d’Escouade en sont teintées (`color-mix(in oklch, var(--p) 8%, …)`) ; un thème peut garder cette teinte en écrivant ses surfaces de la même façon.

## Les jetons

Ce sont ceux de `src/app.css`, listés dans `DESIGN.md` (« Les jetons ») :

| Famille | Jetons |
| --- | --- |
| Projet | `--p-ink`, `--p-soft`, `--sel`, `--glow`, `--brand` |
| Surfaces | `--frame`, `--panel`, `--raise`, `--raise-2`, `--well`, `--scrim`, `--shade`, `--drop-base`, `--vo-base`, `--orb-base` |
| Filets et survol | `--fg-rgb`, `--line`, `--line-2`, `--line-3`, `--hover`, `--sep`, `--ring` |
| Textes | `--text`, `--soft`, `--muted`, `--dim`, `--text-ai`, `--idle` |
| États | `--run`, `--wait`, `--wait-bg`, `--wait-line`, `--done`, `--add`, `--del`, `--git-m`, `--warn-ink`, `--info`, `--teal-ink`, `--teal-ink-2`, `--disc`, `--brain` |
| Bouton plein et encres | `--primary`, `--primary-hover`, `--on-primary`, `--on-primary-line`, `--on-primary-kbd`, `--on-primary-dim`, `--on-wait`, `--on-hit`, `--on-tick`, `--on-tile`, `--win-close`, `--on-win-close` |
| Historique de git | `--lane-main`, `--lane-1` à `--lane-8` |
| Code et terminal | `--code-fg`, `--code-ln`, `--code-nb`, `--code-dim`, `--syn-k`, `--syn-f`, `--syn-s`, `--syn-t`, `--syn-n`, `--ansi-k`, `--ansi-w`, `--ansi-w-2`, `--ansi-y`, `--ansi-cy`, `--ansi-cy-2`, `--term-sel-bg`, `--term-sel-fg` |

La forme n’est pas du ressort d’un thème : polices, rayons, largeurs de la barre des projets et de la barre latérale (`--font`, `--mono`, `--r`, `--rail`, `--side`…) restent ceux d’Escouade.

Les terminaux tirent leurs couleurs de `--well`, `--code-fg`, `--text`, des couleurs ANSI (`--ansi-*`, `--del`, `--add`, `--syn-f`, `--syn-k`, `--code-dim`) et de `--p` pour la sélection ; l’éditeur, de `--code-*` et `--syn-*`.

## Les valeurs permises

Escouade filtre chaque valeur avant de la poser sur la fenêtre. Ce qui ne passe pas est écarté, et le jeton garde la valeur d’Escouade ; Réglages › Général dit combien de valeurs du thème choisi ont été écartées.

- **Les couleurs** (presque tous les jetons) : `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, les fonctions `rgb()`, `rgba()`, `hsl()`, `hsla()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`, `color()` et `color-mix()`, les couleurs relatives (`oklch(from var(--wait) l c h / 0.1)`), `transparent`, `white`, `black`, et `var()` d’un autre jeton de la liste (ou de `--p`). Une seule couleur par jeton, 160 caractères au plus.
- **Les proportions** `--sel` et `--glow` : un pourcentage de 0 à 100 % (`26%`).
- **Le nombre** `--shade`, qui adoucit les ombres : de 0 à 1 (`0.3`).
- **Les trois canaux** `--fg-rgb`, l’encre des filets : `15, 17, 22`.
- **La palette** `projectColors` : des couleurs, sans `var()`.

Jamais de `url()`, d’autre fonction (`image-set()`, `attr()`, `env()`…), de guillemets, d’antislash, de `;`, d’accolade ni de `!important` : un thème ne charge rien, n’écrit pas de CSS à lui, et ne peut pas masquer une demande d’Escouade (une autorisation, une question). Un jeton qui n’est pas dans la liste est ignoré.

## Ce que voit l’utilisateur

- **Le choix.** La rangée « Couleurs », sous « Thème » : « Escouade », puis les thèmes des plugins installés et activés pour l’app. Sans thème installé, un bouton « Voir les thèmes » ouvre la vue Plugins sur sa catégorie « Thèmes ».
- **Tout de suite.** La fenêtre, les terminaux ouverts (sans les rouvrir) et l’éditeur prennent le thème dès qu’il est choisi ; la palette remplace celle d’Escouade dans l’assistant « Nouveau projet », dans Réglages › Projets › Projet et dans le menu d’une tuile. Un projet garde sa couleur : si elle n’est pas dans la palette, elle y est ajoutée à la fin.
- **Au démarrage.** Le thème choisi est gardé avec les réglages, et posé avant le premier affichage.
- **Le plugin retiré.** Désactivé, désinstallé ou devenu invalide, son thème quitte la fenêtre et Escouade reprend ses couleurs ; Réglages › Général le dit. Le choix reste : le plugin réactivé, son thème revient.

## Tester un thème

`escouade-plugin validate` vérifie le manifeste. Pour voir le thème, installe le dossier du plugin en développement (vue Plugins › « En développement »), choisis-le dans Réglages › Général et passe de Sombre à Clair. Le plugin d’exemple « Contraste élevé », celui du manifeste plus haut (`plugins/high-contrast` dans le dépôt d’Escouade), est un point de départ : voir [Les plugins d’exemple](plugins/examples#contraste-eleve).

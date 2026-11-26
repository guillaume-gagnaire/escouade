# ESC-661 — les pages des agents (2.3), trois directions

Rien du produit ne change dans ce ticket. Ce sont des maquettes statiques, faites pour choisir une direction. L'outil MCP, le retour des choix par `ask_user`, les modèles, la page à soi, la sécurité, la place dans l'app, les choix à trancher et les tickets sont dans [`docs/AGENT-PAGES.md`](../../docs/AGENT-PAGES.md).

- `maquette.html` : les trois directions. On choisit par `?d=a|b|c`, `&theme=light`, puis `#revue` (l'agent ouvre la revue de POR-41 à côté de sa conversation et met en évidence le troisième critère) ou `#choisi` (l'utilisateur a repris la main et choisi A dans la page ; la bande d'Escouade attend son geste). La page prend la taille de la fenêtre du navigateur : 1440 × 900 ou 1024 × 728. Elle utilise les jetons de `src/app.css`, les polices de l'app et les icônes de `src/lib/icons.ts` (copiées dans `icons.js`). Les données sont fictives : le projet « portail-adherents », l'agent « pagination », le ticket POR-41.
- `captures/` : `node design/agent-pages/render.mjs` produit 24 PNG, soit 3 directions × 2 écrans, en sombre et en clair (`-clair`), à 1440 × 900 et à 1024 × 728 (`-1024`). Le script signale tout texte qui déborde et tout ce qui sort de la fenêtre. La dernière passe ne signale rien.

## Comment les directions ont été choisies

Avec le skill `impeccable`, en mode Operate : le monde visuel est établi (`DESIGN.md`), donc seule la composition est ouverte. Sept structures ont été tirées du cas phare (la liste est dans `docs/AGENT-PAGES.md` § 12). Le tirage (`concept-seed --scope surface --mode operate`, clé `32b8d056`) a distribué les n° 7 (en tête), 5 et 2.

## Les trois directions

Ce qui est commun aux trois directions :

- la **bande de confiance** au-dessus de la page : « Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers » ;
- la **bande du choix**, qui appartient à Escouade et se trouve hors de la page : rien ne part sans son bouton, et elle montre le libellé de la demande lu dans le cœur ;
- l'indication de qui a la main : « pagination pilote · Reprendre la main », puis « Tu as la main · La rendre » ;
- la carte dans la conversation ;
- la ligne de « Navigateurs », avec son étiquette « choix ».

| Direction | Captures | En une phrase |
| --- | --- | --- |
| **A — Les repères** (choisie le 2026-11-25) | `a-revue*.png`, `a-choisi*.png` | La conversation et la page côte à côte. Chaque geste de l'agent est un repère numéroté, à la fois dans la conversation et sur un rail le long de la page. |
| B — La visite | `b-*.png` | La page domine ; la conversation se réduit à la narration de l'agent, étape par étape. |
| C — L'onglet | `c-*.png` | La page est un onglet du panneau de droite ; les choix sont natifs, au pied du panneau. |

## Ce qui n'est pas montré

Les états d'erreur (page trop lourde, modèle aux données refusées), la page à soi, le schéma et la comparaison. Ils suivent la direction choisie et viendront avec AP-3 et AP-5. La vérification par le relecteur de finition d'impeccable n'a pas tourné : le ticket demande de travailler sans sous-agents. Les deux passes de captures ont été relues ici, à 1440 et à 1024, en clair et en sombre.

## Les modèles (AP-5, ESC-674)

`captures/modeles/` : les modèles `review`, `diagram` et `compare` tels que la cloison les sert, en sombre (fr, page à côté de la conversation à 1440) et en clair (en, à 1024), avec des données dont chaque texte est piégé (il reste du texte). Elles viennent de l'e2e `tests/e2e/agentpage-templates.spec.ts`, qui en fait 24 (fr et en, clair et sombre, 1440 et 1024) dans `ESCOUADE_CAPTURES` quand il est donné.

# ESC-65 — la vue « Pull requests » du projet, trois directions

Captures prises dans la vraie app de debug (outillage des e2e), sur le faux GitHub des e2e : 30 PR ouvertes sur `acme/demo`, lues par l'app elle-même (`project_prs`, via le plugin GitHub). Le compte connecté est `@ada`. Seuls deux faits sont posés à la main, faute de champ dans l'app aujourd'hui (ils viendront en phase 2) : à qui une relecture est demandée, et l'agent du projet déjà sur une PR (« rate-limit »).

Relancer : `npx tauri build --debug --no-bundle`, puis `npx playwright test -c design/esc-65/playwright.config.ts` (`directions.js` dessine chaque direction dans la fenêtre avec les jetons, les classes et les icônes de l'app ; rien n'est branché).

## Où vit la vue

Dans la barre latérale du projet, un quatrième bouton « PR » (« MR » sur GitLab) entre Kanban et Docs, avec le nombre de PR ouvertes ; la vue prend la place de travail comme le Kanban, avec le même en-tête (titre, ligne du projet, contrôles à droite). Les trois filtres sont un `segmented` de l'en-tête, avec leur compte : « Les miennes », « Relecture demandée », « Toutes ». « Rafraîchir » et « Lu il y a 12 s » à droite. À 276 px, les quatre boutons de la barre latérale tiennent sur une ligne mais serrés : l'implémentation leur donnera leur place (le « PR » sans compte, comme Docs ; le nombre est dans l'en-tête de la vue).

## Les états (communs aux trois)

- `etat-chargement.png` : « Lecture des PR ouvertes de acme/demo… ».
- `etat-erreur.png` : le cadre rouge de l'onglet « PR et CI », ses mots (« Hébergeur injoignable. », « Jeton refusé… », « Limite d'API… ») et « Réessayer ».
- `etat-vide.png` / `etat-vide-relecture.png` : un message par filtre, et « Toutes les PR » pour en sortir.
- `*-une.png` : une seule PR, dans chaque direction.
- PR de fork : marque « fork marta » / « fork eve » (sa branche se lit `marta:i18n-es`) ; « Lancer un agent » la récupère par `refs/pull/<n>/head` (`refs/merge-requests/<n>/head` sur GitLab).

## A — Liste dense (façon Linear)

`a-toutes.png`, `a-toutes-clair.png`, `a-miennes.png`, `a-relecture.png`

Une ligne de 34 px par PR : signe de la CI, numéro, titre et marques (brouillon, fork, agent), puis relectures et conflit, la CI en mots, l'auteur et l'âge en colonnes alignées. La ligne survolée ou choisie montre « Lancer un agent » et l'ouverture dans le navigateur.

- Forces : les 30 PR tiennent dans la fenêtre ; se balaie de haut en bas, se lit au clavier (↑ ↓, Entrée) ; les colonnes alignées se comparent d'un coup d'œil ; la plus proche de la liste de l'onglet « PR et CI ».
- Limites : rien ne dit ce qui attend quoi sans lire chaque ligne ; les actions n'apparaissent qu'au survol.

## B — Colonnes par ce qu'attend la PR (façon Kanban)

`b-toutes.png`, `b-toutes-clair.png`, `b-miennes.png`, `b-relecture.png`

Les colonnes et les cartes du Kanban d'ESC-78 : « À corriger » (CI en échec ou conflit), « Changements demandés », « À relire », « Prêtes à merger », « Brouillons ». Une carte dit son numéro, son auteur, son âge, sa CI, son titre sur deux lignes, sa branche → sa cible et ses relectures ; une relecture qui t'est demandée la colore en ambre, comme une carte à tester.

- Forces : on voit d'abord ce qui bloque ; même geste que le Kanban, qu'on connaît déjà.
- Limites : moins de PR à l'écran (cinq colonnes étroites, il faut défiler) ; la colonne d'une PR est déduite par Escouade et peut surprendre ; fait doublon avec la colonne « En revue » du Kanban (ESC-59) pour les PR des tickets.

## C — Liste et détail côte à côte

`c-toutes.png`, `c-toutes-clair.png`, `c-miennes.png`, `c-relecture.png`

Une liste de deux lignes par PR (titre, puis numéro, auteur, âge et pastilles) et, à droite, la PR choisie : son état, sa branche → sa cible, sa CI, ses relectures, le conflit, les trois actions en boutons (« Lancer un agent sur cette PR », « Ouvrir dans le navigateur », « Faire relire par un agent », ticket suivant) et ce que fera le lancement (le worktree, la référence récupérée, rien de publié).

- Forces : l'action principale est un vrai bouton, toujours visible, et le lancement s'explique avant le clic (fork compris) ; la place à droite accueillera la relecture par un agent (ticket suivant).
- Limites : la liste n'a que la moitié de la largeur ; la vue détaillée redit en partie l'onglet « PR et CI » du panneau latéral.

## Recommandation

**A**, la liste dense. La vue sert à trouver vite la PR sur laquelle agir parmi des dizaines, et à filtrer ; A en montre le plus, se lit au clavier et reste cohérente avec la liste « PR ouvertes du projet » de l'onglet « PR et CI » (qu'elle prolonge sans la répéter). Pour que l'action ne dépende pas du survol, je garderais de C la ligne choisie qui montre ses boutons, et Entrée qui lance l'agent ; le détail d'une PR reste l'onglet « PR et CI ». B est séduisante, mais fait double emploi avec le Kanban et sa future colonne « En revue ».

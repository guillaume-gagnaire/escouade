# ESC-508 — la boîte « À toi » (2.3), quatre directions

Demande de l'utilisateur (2026-11-16) : « une solution pour voir rapidement les inputs que mes agents attendent de moi […] Limite faudrait un onglet dédié pour voir tout ce que les agents veulent que je fasse / choisisse en asynchrone », « utilise Impeccable pour placer et designer cet écran », « il faudra que dans cet écran, on puisse avoir les pièces jointes fournies par les agents (images / pdf, etc) ». Règle du même jour : la boîte repose sur le serveur MCP, son onglet ne s'affiche que s'il est activé.

Rien du produit ne change dans ce ticket : ce sont des maquettes statiques pour choisir une direction. Le modèle d'une demande, ses pièces jointes, ses états, le contrat des outils MCP, les textes en français et en anglais, les notifications, la sécurité, les choix à trancher et le découpage sont dans [`docs/INBOX.md`](../../docs/INBOX.md).

- `maquette.html` : les quatre directions. `?d=a|b|c|d`, `&theme=light`, puis `#pleine` (la boîte pleine), `#choix` (un choix entre trois rendus), `#pdf` (une demande avec un PDF, deux images et un tableur joints), `#validation` (un feu vert), `#tache` (une tâche à faire), `#vide` (la boîte vide) ou `#compteur` (le compteur dans la fenêtre, boîte fermée). `?d=x#eteint` et `?d=x#eteindre` : le serveur MCP éteint, commun aux quatre. La page prend la taille de la fenêtre du navigateur : ouvre-la en 1440 × 900 ou en 1024 × 728. Jetons de `src/app.css`, polices de l'app, icônes de `src/lib/icons.ts` (copiées dans `icons.js`, avec huit de plus). Données fictives : les projets « portail-adherents » (ouvert) et « atlas-web », leurs agents et leurs tickets ; les rendus, le PDF et les graphiques sont dessinés par la page.
- `captures/` : `node design/inbox-2-3/render.mjs` (120 PNG : 4 directions × 7 écrans, plus les 2 écrans du serveur éteint, en sombre et en clair `-clair`, à 1440 × 900 et à 1024 × 728 `-1024`). Le script signale tout texte qui déborde, tout ce qui sort de la fenêtre et tout ce qu'une boîte de hauteur fixe coupe ; les listes qui défilent dans l'app (barre latérale, file, colonnes, page d'un PDF) en sont exclues. La dernière passe ne signale rien.

## Comment les directions ont été choisies

Avec le skill `impeccable` : la surface est neuve dans un monde visuel établi (`DESIGN.md`, la refonte 2.0), donc seule la composition est ouverte, en mode Operate. Sept structures, de la plus à la moins évidente :

1. la liste dans un onglet de la barre latérale, la demande dans le panneau (comme une boîte mail) ;
2. une vue de l'app en trois colonnes (liste, demande, pièces jointes), à côté de la vue d'ensemble ;
3. le tiroir de la barre d'état : le compteur « en attente » devient « À toi » et ouvre la boîte par-dessus le travail ;
4. un premier groupe « À toi » en tête de la vue d'ensemble ;
5. la file, une demande à la fois, en grand, dans une vue de tous les projets ;
6. le tableau : un Kanban de ce qui attend, une colonne par geste (Choisir, Valider, Répondre, Faire) ;
7. dans la conversation de chaque agent, une carte épinglée, et seulement un compteur global.

Le tirage d'impeccable (`concept-seed --scope surface --mode operate`, clé `28c617af`) a distribué les n° 6, 5 et 3, et non les deux premières, les plus attendues. L'utilisateur a demandé le 2026-11-17 de rendre ces trois **et la n° 1**, « la forme la plus attendue » : il avait parlé d'« un onglet dédié ».

Les challengers du tirage, jugés sur deux axes, l'identification du public et la clarté du produit :

| Challenger                                    | Verdict                               | Ce que les directions en gardent                                                                                                                                                                                                                                                                                                                  |
| --------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| La section des planches d'un annuel de design | **Compétitif**, gardé dans les quatre | La comparaison des rendus en **planches** : chaque option sur son fond, une ligne de crédits sous l'image (lettre, nom, description), et la recommandation de l'agent en **sceau** rond posé à demi hors de la planche. C'est le geste signature de la boîte, et il ne prend rien d'autre au monde de l'annuel (ni papier, ni croix de repérage). |
| Le mur de vignettes d'un service de streaming | Écarté                                | Le rendu sert d'affiche : un choix montre ses trois vignettes dans la liste même (D, C) et sur sa carte (A).                                                                                                                                                                                                                                      |
| Le manuel à intercalaires                     | Écarté                                | La règle de l'onglet proportionnel à son contenu : la jauge de la file (B) montre ce qui est fait et ce qui reste, une case par demande.                                                                                                                                                                                                          |
| Le spécimen de caractère variable             | Écarté                                | Le curseur qui balaie entre deux états : « Sombre · Clair · Les deux » de la comparaison en grand.                                                                                                                                                                                                                                                |
| La notation de danse, la carte d'un zoo       | Écartés                               | Rien : costumes, contraires au monde de l'app.                                                                                                                                                                                                                                                                                                    |

## Les quatre directions

Chacune montre les mêmes sept écrans, avec les mêmes demandes et les mêmes textes (ceux de `docs/INBOX.md`) ; seules la place et la forme changent.

### A — « Le tableau »

Un onglet **« À toi »** dans la piste de la barre latérale (après Docs), avec le compte du projet ; la barre latérale porte alors les filtres (projets, sortes), « Plus tard » et « Répondu récemment ». Le panneau devient un **tableau** : en haut, la bande ambre des agents arrêtés, réglés sur place ; dessous, quatre colonnes par geste, **Choisir, Valider, Répondre, Faire**, des cartes avec leurs vignettes et ce que la réponse débloque. Une carte s'ouvre en **volet** à droite (plein panneau à 1024) ; un choix s'ouvre droit sur **la comparaison en grand**, plein fenêtre, clair et sombre l'un sur l'autre.

|               | Sombre 1440                    | Clair 1024                                |
| ------------- | ------------------------------ | ----------------------------------------- |
| Pleine        | ![](captures/a-pleine.png)     | ![](captures/a-pleine-clair-1024.png)     |
| Choix         | ![](captures/a-choix.png)      | ![](captures/a-choix-clair-1024.png)      |
| PDF et images | ![](captures/a-pdf.png)        | ![](captures/a-pdf-clair-1024.png)        |
| Feu vert      | ![](captures/a-validation.png) | ![](captures/a-validation-clair-1024.png) |
| Tâche         | ![](captures/a-tache.png)      | ![](captures/a-tache-clair-1024.png)      |
| Vide          | ![](captures/a-vide.png)       | ![](captures/a-vide-clair-1024.png)       |
| Compteur      | ![](captures/a-compteur.png)   | ![](captures/a-compteur-clair-1024.png)   |

- **Pour** : tout se voit d'un coup, rangé par ce qu'il demande ; on traite ensemble les choses du même geste (tous les oui ou non, puis les choix). La bande des bloquantes est la plus visible des quatre. Les rendus sont déjà sur les cartes.
- **Contre** : ses colonnes ne sont pas des étapes (une carte n'y avance jamais) : c'est le vocabulaire du Kanban détourné, à côté du vrai Kanban. Quatre colonnes de 170 px à 1024. Le volet cache le tableau pour lire une demande longue. Cinq onglets serrés à 1024 (le compte des agents en sort).

### B — « La file »

Une **vue de l'app**, tous projets, ouverte par l'icône de la boîte en haut de la barre des projets, sous « Vue d'ensemble », avec le compte de tous les projets. À gauche, **la file** : la jauge (une case par demande, verte faite, ambre bloquante), « 3 traitées aujourd'hui · il en reste 9 », les groupes. À droite, **une demande à la fois, en grand** : les trois rendus en planches pleine largeur, le PDF à côté du texte, la réponse en bas ; `J` `K` `L` (suivante, précédente, plus tard), et une réponse envoyée ouvre la suivante.

|               | Sombre 1440                    | Clair 1024                                |
| ------------- | ------------------------------ | ----------------------------------------- |
| Pleine        | ![](captures/b-pleine.png)     | ![](captures/b-pleine-clair-1024.png)     |
| Choix         | ![](captures/b-choix.png)      | ![](captures/b-choix-clair-1024.png)      |
| PDF et images | ![](captures/b-pdf.png)        | ![](captures/b-pdf-clair-1024.png)        |
| Feu vert      | ![](captures/b-validation.png) | ![](captures/b-validation-clair-1024.png) |
| Tâche         | ![](captures/b-tache.png)      | ![](captures/b-tache-clair-1024.png)      |
| Vide          | ![](captures/b-vide.png)       | ![](captures/b-vide-clair-1024.png)       |
| Compteur      | ![](captures/b-compteur.png)   | ![](captures/b-compteur-clair-1024.png)   |

- **Pour** : la plus rapide pour vider la boîte (dix demandes à la suite, au clavier). Les rendus et le PDF ont toute la place. Tous projets d'emblée, comme la vue d'ensemble. C'est la forme du téléphone : une demande par écran.
- **Contre** : elle prend toute la fenêtre et fait quitter le projet et la conversation en cours. Ce n'est pas l'« onglet dédié » demandé, mais un deuxième endroit du rail à côté de la vue d'ensemble, qui montre déjà les agents arrêtés. Le travail par projet y est un filtre.

### C — « Le tiroir »

Le compteur « en attente » de la barre d'état devient **« À toi 9 · 2 bloquantes »** : un seul chiffre pour tout ce qui attend. Il ouvre **un tiroir** qui monte de lui, par-dessus la barre latérale, la conversation toujours visible à droite. On y trouve les filtres par sorte, la liste avec ses vignettes, et la demande ouverte à côté de la liste (seule à 1024, avec « ‹ Toutes »). Un choix montre une pellicule : un rendu en grand, les trois en bande.

|               | Sombre 1440                    | Clair 1024                                |
| ------------- | ------------------------------ | ----------------------------------------- |
| Pleine        | ![](captures/c-pleine.png)     | ![](captures/c-pleine-clair-1024.png)     |
| Choix         | ![](captures/c-choix.png)      | ![](captures/c-choix-clair-1024.png)      |
| PDF et images | ![](captures/c-pdf.png)        | ![](captures/c-pdf-clair-1024.png)        |
| Feu vert      | ![](captures/c-validation.png) | ![](captures/c-validation-clair-1024.png) |
| Tâche         | ![](captures/c-tache.png)      | ![](captures/c-tache-clair-1024.png)      |
| Vide          | ![](captures/c-vide.png)       | ![](captures/c-vide-clair-1024.png)       |
| Compteur      | ![](captures/c-compteur.png)   | ![](captures/c-compteur-clair-1024.png)   |

- **Pour** : à portée de main partout, sans quitter ce qu'on lisait ; le compteur de la barre d'état réunit enfin les bloquantes et le reste ; rien n'ajoute d'onglet. Le moins cher à placer.
- **Contre** : un tiroir est fait pour un coup d'œil, pas pour vingt minutes de choix ; il cache la barre latérale (les agents qui attendent) ; sa hauteur est comptée : à 1024, le PDF ne montre que le haut de sa page, et la pellicule réduit les rendus. La comparaison en grand y reste un plein écran à part.

### D — « La liste »

L'**onglet « À toi »** de la piste (après Docs), avec le compte du projet, comme une boîte mail. La **liste** prend la barre latérale : portée « Ce projet · Tous », les bloquantes en tête (réglables sur place), « Aujourd'hui », « Plus tôt », « Plus tard », « Répondu récemment » ; un choix montre ses trois vignettes dans sa ligne. La **demande** prend le panneau : son en-tête (qui, quoi, depuis quand), ses planches avec le sceau, ses pièces jointes et le PDF lu sur place à côté du texte, ce que la réponse débloque, et la **réponse en bas**, comme le champ de la conversation : l'option, un mot, la voix, « Plus tard », « Choisir B ».

|               | Sombre 1440                    | Clair 1024                                |
| ------------- | ------------------------------ | ----------------------------------------- |
| Pleine        | ![](captures/d-pleine.png)     | ![](captures/d-pleine-clair-1024.png)     |
| Choix         | ![](captures/d-choix.png)      | ![](captures/d-choix-clair-1024.png)      |
| PDF et images | ![](captures/d-pdf.png)        | ![](captures/d-pdf-clair-1024.png)        |
| Feu vert      | ![](captures/d-validation.png) | ![](captures/d-validation-clair-1024.png) |
| Tâche         | ![](captures/d-tache.png)      | ![](captures/d-tache-clair-1024.png)      |
| Vide          | ![](captures/d-vide.png)       | ![](captures/d-vide-clair-1024.png)       |
| Compteur      | ![](captures/d-compteur.png)   | ![](captures/d-compteur-clair-1024.png)   |

- **Pour** : la forme attendue, « l'onglet dédié » de la demande. Elle suit Agents, Kanban, PR et Docs (une liste dans la barre, le travail dans le panneau), garde le projet et ses agents à un onglet, et laisse la place de lire un PDF ou trois rendus. La réponse en bas, comme dans une conversation : répondre à une demande, c'est répondre à l'agent.
- **Contre** : la plus attendue, aussi pour ce qu'elle a de banal. Cinq onglets serrés à 1024 (le compte des agents en sort). « Ce projet » ou « Tous » est un choix de plus. Sans le traitement à la suite de B, on clique chaque ligne.

### Commun : le serveur MCP éteint

|                                                             | Sombre 1440                  | Clair 1024                              |
| ----------------------------------------------------------- | ---------------------------- | --------------------------------------- |
| Réglages, serveur éteint : pas d'onglet, la boîte expliquée | ![](captures/x-eteint.png)   | ![](captures/x-eteint-clair-1024.png)   |
| Éteindre avec des demandes ouvertes                         | ![](captures/x-eteindre.png) | ![](captures/x-eteindre-clair-1024.png) |

## Comparaison

|                           | A · Le tableau                            | B · La file                    | C · Le tiroir                            | D · La liste                               |
| ------------------------- | ----------------------------------------- | ------------------------------ | ---------------------------------------- | ------------------------------------------ |
| Où                        | onglet du projet, tableau dans le panneau | vue tous projets, dans le rail | barre d'état, tiroir par-dessus          | onglet du projet, liste et panneau         |
| Ce qu'on garde à l'écran  | la barre latérale (filtres)               | rien du projet                 | la conversation                          | le projet (rail, onglets)                  |
| Comparer trois rendus     | plein écran d'emblée                      | **en grand dans la file**      | pellicule, puis plein écran              | planches dans le panneau, puis plein écran |
| Lire un PDF               | volet (plein panneau à 1024)              | **à côté du texte, en grand**  | serré à 1024                             | à côté du texte                            |
| Vider la boîte à la suite | par colonne                               | **le plus rapide**             | moyen                                    | avec `J` `K` (emprunté à B)                |
| Le compteur               | onglet                                    | rail                           | **un seul chiffre dans la barre d'état** | onglet (+ barre d'état, emprunté à C)      |
| 1024 × 728                | serré (colonnes, onglets)                 | à l'aise                       | serré (hauteur)                          | à l'aise (onglets serrés)                  |
| Le téléphone              | non                                       | **c'est sa forme**             | non                                      | la liste, puis B pour une demande          |
| Coût                      | moyen                                     | moyen                          | **le plus bas**                          | moyen                                      |

**Recommandation : D**, avec trois emprunts :

- **à B**, le traitement à la suite (`J` `K` `L`, « 3 sur 9 », la réponse qui ouvre la suivante) et sa forme pour le téléphone ;
- **à C**, le compteur « 2 en attente · 9 à toi » dans la barre d'état, qui ouvre l'onglet sur « Tous » ;
- **à A**, la comparaison en grand (planches, sceau, « Sombre · Clair · Les deux »).

D répond à la demande telle qu'elle est faite (un onglet dédié) dans la grammaire que l'app a déjà. Elle a la place des rendus et des PDF aux deux tailles. Les emprunts lui retirent ses deux défauts : cliquer chaque ligne, et un compteur caché dans un onglet. Voir le choix I1 de `docs/INBOX.md`.

## Vérifications

- `node design/inbox-2-3/render.mjs` : 120 captures, aucun débordement, rien hors de la fenêtre, rien de coupé (hors listes qui défilent).
- `impeccable detect` sur `maquette.html` : aucun motif bloquant ; des avis sur des valeurs hors de `DESIGN.md` (rayons de 6 et 2,5 px, déjà ceux des pastilles et des points de projet de l'app ; `#1a1405`, l'encre `--on-wait` ; les couleurs des rendus fictifs, qui ne sont pas l'interface).

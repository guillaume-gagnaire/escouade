# ESC-67 · la vue « Environnements »

Ce qui tourne où (commit, date, auteur, adresse), les tickets livrés dans chaque environnement, ceux mergés qui attendent la production, et un déploiement en échec avec le lien de son job. La vue s'ouvre depuis l'en-tête du Kanban (« Tickets · Environnements »). Données fictives. Captures : `node design/esc-67/render.mjs`.

| | Direction | Forces | Limites |
|---|---|---|---|
| **A** (`a.png`) | **Couloirs** : une colonne par environnement, dans l'ordre où le code avance (mergés → staging → production), comme le Kanban | Se lit d'un coup comme un flux ; même grammaire que le Kanban (cartes) ; l'échec est sur la colonne concernée | Un ticket n'apparaît que dans le dernier environnement atteint ; au-delà de 3 ou 4 environnements, les colonnes se serrent |
| **B** (`b.png`) | **Cartes + tableau** : une carte par environnement en haut, puis un tableau ticket × environnement (✓ / ⏳) | Répond exactement à « ce ticket est-il en prod ? » ; tient beaucoup de tickets et d'environnements ; le bandeau donne le compte en attente | Plus « tableau de bord », moins chaleureux ; la colonne « Mergé » répète la carte |
| **C** (`c.png`) | **Sections** : un bloc par environnement (ce qui tourne, puis ses tickets en pastilles), le bandeau des tickets en attente en tête | La plus compacte ; marche dans une fenêtre étroite ; rien à faire défiler de côté | Les tickets d'un environnement se répètent dans le suivant ; moins visuel |

**Recommandation : A.** Elle prolonge le Kanban (après « Terminé », le code continue d'avancer de gauche à droite), se comprend sans légende, et l'échec saute aux yeux sur l'environnement touché. Le compte « N tickets pas encore en production » se met en tête de la colonne des mergés.

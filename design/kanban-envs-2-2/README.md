# ESC-390 · les onglets de la barre latérale, l'en-tête du Kanban et la vue Environnements

Retour sur la 2.2.0-beta.2 : `avant-onglets.png` et `avant-environnements.png`, les captures de l'utilisateur. Trois directions, chacune rendue pour les trois endroits, en sombre et en clair (`captures/`, `node design/kanban-envs-2-2/render.mjs`) :

- `<d>-fenetre.png` : la fenêtre entière (1440 × 900), le projet Escouade tel qu'il est le 2026-11-08, vue Environnements ouverte ;
- `<d>-etats.png` : la planche des états : onglets avec et sans hébergeur, GitLab (« MR »), barre la plus étroite (220 px) ; l'en-tête du Kanban à trois largeurs de fenêtre (1440, 1250, 1110 px), CI rouge, en cours, verte ; la vue Environnements d'un projet à staging et production (fictif) : colonne vide, déploiement en cours, échec, et le choix des environnements montrés.

## Ce qui est commun aux trois

- **L'onglet PR** (et l'onglet Environnements du Kanban) n'apparaît que si le projet a un hébergeur de code : une remote reconnue par un plugin d'hébergeur actif. Sans hébergeur, trois onglets. Si l'hébergeur part pendant que la vue PR est ouverte, on revient à la vue Agents ; s'il arrive, l'onglet apparaît.
- **Les environnements montrés** : d'office, tous ceux qui ont déjà eu un déploiement réussi (`github-pages`, qui n'en a aucun, n'est pas montré, et la vue le dit). L'utilisateur en masque d'autres (`npm`, la publication du SDK) et change leur ordre ; le dernier montré compte comme la production. Gardé par projet, sur la machine. Un déploiement en cours ou en échec se lit sur l'environnement lui-même, jamais dans une boîte vide ; une colonne vide dit pourquoi.

## Les trois directions

| | Direction | Onglets | En-tête du Kanban | Environnements |
|---|---|---|---|---|
| **A** | **Deux étages** | Texte seul, chaque onglet à sa largeur, la pastille active en relief ; compteur gris, « à relire » en pastille ambre | Étage 1 : titre, projet, « Importer », « Pilote auto ». Étage 2 : « Tickets · Environnements » soulignés à gauche ; CI, places, « Après validation » à droite. Rien ne se chevauche : chaque étage a sa place | Colonnes sans boîte, l'état de l'environnement dans sa tête (commit, adresse, en cours, échec souligné de rouge) ; un menu « 1 environnement sur 2 » coche et range les environnements |
| **B** | **Une barre, des groupes** | Icône au-dessus du mot, soulignés de la couleur du projet ; tient à 220 px sans rien couper | Une seule rangée : « Tickets · Environnements » devient le titre ; CI et places réunies en un groupe ; réglages et import en boutons | Un fil « Mergés → npm » en tête ; des colonnes en caissons, l'état en bandeau (ambre en cours, rouge en échec) ; une colonne « Pas montrés » au bout, « Montrer » d'un clic, « Masquer » dans le ⋯ de chaque colonne |
| **C** | **Lignes** | Icônes seules, l'onglet actif s'élargit avec son mot | Les deux vues comme deux titres ; la CI dans la ligne du dessous ; les places en jauge | Une ligne par environnement : son état à gauche, ses tickets en puces à droite ; les environnements pas montrés au pied, « Choisir les environnements… » |

### Forces et limites

- **A** : la plus proche de l'app (Linear, réglages de la 2.2) ; l'en-tête a de la place pour tout, à toutes les largeurs ; la vue reste le Kanban qui continue. Limite : l'en-tête prend 40 px de plus.
- **B** : la plus compacte en hauteur ; le fil montre le chemin du code. Limites : une seule rangée doit encore tout tenir à 1000 px (le projet et ses compteurs disparaissent tôt) ; les onglets à icône au-dessus sont plus hauts que l'app n'en a ailleurs.
- **C** : la plus dense pour beaucoup de tickets et beaucoup d'environnements ; rien ne défile de côté. Limites : les onglets en icônes se devinent moins ; les puces coupent les titres ; elle s'éloigne du Kanban.

**Recommandation : A.** Elle règle le chevauchement par la structure plutôt que par des coupes, garde la grammaire du Kanban, et ses onglets sont les plus lisibles à la largeur habituelle.

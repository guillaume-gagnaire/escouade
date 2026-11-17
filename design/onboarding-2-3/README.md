# ESC-503 — l'onboarding d'Escouade (2.3), trois directions

Demande de l'utilisateur (2026-11-16) : « la phase d'onboarding sur Escouade : poser les questions de harness, etc, très didactique, pour setup rapidement Escouade et bien découvrir les différentes fonctionnalités (voix, etc). On fera le design et l'ux avec impeccable. »

Rien du produit ne change dans ce ticket : ce sont des maquettes statiques pour choisir une direction. Le parcours, ses textes en français et en anglais, ses états, la mesure et les choix à trancher sont dans [`docs/ONBOARDING.md`](../../docs/ONBOARDING.md).

- `maquette.html` : les trois directions. `?d=a|b|c`, `&theme=light`, puis `#harness`, `#modes`, `#projet`, `#agent` ou `#decouverte`. La page prend la taille de la fenêtre du navigateur : ouvre-la en 1440 × 900 ou en 1024 × 728. Jetons de `src/app.css`, polices de l'app, icônes de `src/lib/icons.ts` (copiées dans `icons.js`). Données fictives : le compte « toi@exemple.dev », le projet « atlas-web », le dépôt de démonstration « bac-a-sable ».
- `captures/` : `node design/onboarding-2-3/render.mjs` (60 PNG : 3 directions × 5 écrans × sombre et clair `-clair` × 1440 × 900 et 1024 × 728 `-1024`). Le script dit aussi tout texte qui déborde, tout ce qui sort de la fenêtre et tout ce qu'une boîte coupe : la dernière passe n'en signale aucun.

## Comment les trois directions ont été choisies

Avec le skill `impeccable` : la surface est neuve, mais dans un monde visuel établi (`DESIGN.md`, la refonte 2.0). Seule la composition est ouverte. Sept structures, de la plus à la moins évidente :

1. une checklist dans l'écran sans projet ;
2. un plein écran par étapes ;
3. une conversation d'accueil ;
4. un Kanban de départ ;
5. le projet de démonstration d'abord ;
6. des bulles au bon moment ;
7. le cycle d'Escouade en ligne de métro.

Le tirage d'impeccable (`concept-seed --scope surface --mode operate`) en a distribué trois, les n° 5, 6 et 7. Il évite de rendre seulement les trois premières, les plus attendues. L'utilisateur a confirmé ces trois le 2026-11-17.

Les challengers du tirage, jugés sur deux axes, l'identification du public et la clarté du produit :

| Challenger | Verdict | Ce que la direction en garde |
| --- | --- | --- |
| Le plan de métro émaillé | **Gagne**, fondu dans C | La ligne et ses stations, tracées comme les voies du git graph de l'app (`--lane-*`) : l'onboarding parle la langue d'un outil git. |
| Le mur de vignettes d'un service de streaming | Écarté | Une rangée par famille, une seule mise en avant à la fois : la carte de C (une voie par moment du cycle, une station choisie). |
| Le manuel à intercalaires | Écarté | Rien d'autre que la règle : la progression montre toujours où l'on en est et ce qui reste (les compteurs « 2 sur 5 »). |
| Le tableau des départs, les tuiles Metro, l'indicateur aérien | Écartés | Rien : ce sont des costumes d'interface, contraires au monde de l'app. |

## Les trois directions

Chacune montre les mêmes écrans : le harness, les réglages de départ (les modes expliqués), le premier projet, le premier agent avec sa première permission, et la découverte. Elles disent aussi les mêmes choses (les textes sont ceux de `docs/ONBOARDING.md`) ; seule la forme change.

### A — « Essaie d'abord »

Escouade s'ouvre directement sur un **bac à sable** : un petit dépôt de démonstration, créé sur la machine, sans réseau. Un guide **« Premiers pas »** occupe une colonne à droite de la vraie fenêtre. Chaque réglage est demandé au moment où il sert : le harness avant le premier message, le mode juste avant l'envoi, le vrai projet une fois que l'agent a fini. **Le fil**, un trait pointillé teal, court dans les gouttières du guide jusqu'à la vraie chose (le mode sous le message, la carte de permission, le micro) et l'entoure.

| | Sombre 1440 | Clair 1024 |
| --- | --- | --- |
| Harness | ![](captures/a-harness.png) | ![](captures/a-harness-clair-1024.png) |
| Modes | ![](captures/a-modes.png) | ![](captures/a-modes-clair-1024.png) |
| Premier agent | ![](captures/a-agent.png) | ![](captures/a-agent-clair-1024.png) |
| Projet | ![](captures/a-projet.png) | ![](captures/a-projet-clair-1024.png) |
| Découverte | ![](captures/a-decouverte.png) | ![](captures/a-decouverte-clair-1024.png) |

- **Pour** : le temps jusqu'à la valeur est le plus court, puisqu'on est dans l'app dès la première seconde. Tout se fait par le geste réel, dans la vraie fenêtre. Le bac à sable permet d'essayer une permission, un diff et une branche sans risque pour un vrai dépôt. Le même guide sert ensuite à la découverte.
- **Contre** : le bac à sable est un projet de plus à créer, à expliquer et à retirer. La colonne du guide prend 292 px en 1024 : les étapes faites et à venir s'y replient en une ligne. On découvre son harness après avoir vu l'app, ce qui surprend qui n'en a aucun. C'est la direction la plus chère à construire, avec le dépôt de démonstration, le fil et le guide.

### B — « Au bon endroit »

Un **accueil court en trois questions** remplace l'écran sans projet (`Welcome.svelte`) : le harness, le mode, le projet, une question par écran. Ensuite, des **bulles** se posent sur les vrais contrôles, une à la fois, avec un halo teal : la carte de permission, le micro, le pilote auto. Une liste **« À découvrir »** se coche dans le pied de la barre latérale.

| | Sombre 1440 | Clair 1024 |
| --- | --- | --- |
| Harness | ![](captures/b-harness.png) | ![](captures/b-harness-clair-1024.png) |
| Modes | ![](captures/b-modes.png) | ![](captures/b-modes-clair-1024.png) |
| Projet | ![](captures/b-projet.png) | ![](captures/b-projet-clair-1024.png) |
| Premier agent | ![](captures/b-agent.png) | ![](captures/b-agent-clair-1024.png) |
| Découverte | ![](captures/b-decouverte.png) | ![](captures/b-decouverte-clair-1024.png) |

- **Pour** : c'est le plus proche de l'existant (`Welcome.svelte`, `FirstModeModal`), donc le moins cher. Le parcours n'est jamais en travers : trois questions, puis l'app. La liste reste visible sans prendre de place.
- **Contre** : c'est le plus attendu, la « visite par bulles » de beaucoup d'outils. Une bulle cache ce qu'elle désigne en 1024 et doit éviter la frappe et les demandes de permission. La liste dans la barre réduit la place des agents. Le tout est moins didactique : le « pourquoi » tient en une phrase par bulle.

### C — « La ligne »

Le premier lancement prend **toute la fenêtre**. À gauche, une ligne : six stations (harness, compte, réglages de départ, premier projet, premier agent, statistiques). Elles sont tracées comme les commits du git graph de l'app : une station faite est pleine, celle en cours est teal, une station passée reste creuse, « à reprendre ». À droite, **une seule grande question** par station, avec son pourquoi, le choix recommandé, `↑ ↓` et `Entrée`. À la station du premier agent, la **vraie conversation** est encadrée à côté de l'explication. La ligne se prolonge en cinq voies, **Planifier, Coder, Relire, Livrer, Piloter** : c'est la carte **« À découvrir »**, une page de l'app (Aide › À découvrir), où chaque station mène au vrai geste et dit ce qu'il demande (148 Mo, un jeton, une PR ouverte…).

| | Sombre 1440 | Clair 1024 |
| --- | --- | --- |
| Harness | ![](captures/c-harness.png) | ![](captures/c-harness-clair-1024.png) |
| Modes | ![](captures/c-modes.png) | ![](captures/c-modes-clair-1024.png) |
| Projet | ![](captures/c-projet.png) | ![](captures/c-projet-clair-1024.png) |
| Premier agent | ![](captures/c-agent.png) | ![](captures/c-agent-clair-1024.png) |
| Découverte | ![](captures/c-decouverte.png) | ![](captures/c-decouverte-clair-1024.png) |

- **Pour** : la plus didactique, comme le demande l'utilisateur. Une question par écran, avec la place de dire pourquoi, et un schéma de branches pour expliquer le worktree. On sait toujours où on en est et ce qui reste. Elle se fait entièrement au clavier. La carte de la découverte donne d'un coup d'œil tout ce qu'Escouade sait faire, rangé dans le cycle du site (Planifier → Livrer). Le récit est propre à Escouade : la ligne est son git graph, qu'aucun autre outil ne peut reprendre.
- **Contre** : le plein écran sépare le parcours de l'app jusqu'au premier agent. La conversation encadrée le compense, mais on ne voit la vraie fenêtre qu'après. Il faut un composant de ligne et une page de carte. Les couleurs de voies du thème clair sont à foncer pour tenir 3:1 (fait dans la maquette, à reporter dans `src/app.css`).

## Comparaison

| | A · Essaie d'abord | B · Au bon endroit | C · La ligne |
| --- | --- | --- | --- |
| Où vit le parcours | une colonne dans la vraie fenêtre | l'écran sans projet, puis des bulles | plein écran, puis une page de l'app |
| Une question à la fois | oui, dans le guide | oui, trois écrans | oui, une station par écran |
| Didactique (le pourquoi) | moyen : la colonne est étroite | faible : une phrase par bulle | **fort** : la place d'expliquer, des schémas |
| Temps jusqu'au premier agent | **le plus court** | court | court (6 stations, `Entrée` partout) |
| Geste réel | **partout, dans la vraie fenêtre** | oui, par les bulles | la conversation encadrée, puis la carte |
| La découverte | le même guide, une liste | une liste dans la barre + bulles | **une carte du cycle**, page Aide › À découvrir |
| 1024 × 728 | serré : le guide se replie | à l'aise | à l'aise (la carte passe sur une colonne) |
| Coût de réalisation | le plus haut (bac à sable, fil, guide) | **le plus bas** | moyen (ligne, carte) |
| Risque | un projet de démo à tenir propre | l'air d'une visite par bulles comme ailleurs | le plein écran éloigne de l'app au début |

**Recommandation : C**, en lui prenant de B une chose : les trois bulles au bon moment, pour la découverte dans l'app (le micro survolé la première fois, le premier ticket créé, la première PR ouverte). La demande est « très didactique » : C est la seule direction qui a la place d'expliquer, et sa carte rend la découverte lisible sans noyer. Le bac à sable de A reste une des trois réponses de la station « Premier projet » (le projet de démonstration), sans en faire le point de départ. Voir les choix O1 et O2 de `docs/ONBOARDING.md`.

# ESC-516 — le mode serveur (MS-2), trois directions

Demande de l'utilisateur (2026-10-25, relayée) : « lancer Escouade sur un serveur VPS et utiliser le client ici pour piloter à distance (et laisser tourner H24) » ; puis le 2026-11-14 : « avoir plusieurs serveurs Escouade pilotés par un seul Escouade (comme ça j'ai Escouade pour le taf, et en perso) ». Le fond est dans [`docs/REMOTE.md`](../../docs/REMOTE.md) ; le parcours, les choix à trancher (S1 à S16) et le découpage en tickets sont dans [`docs/SERVER-UX.md`](../../docs/SERVER-UX.md).

Rien du produit ne change dans ce ticket : ce sont des maquettes statiques pour choisir une direction.

- `maquette.html` : les trois directions. `?d=a|b|c`, `&theme=light`, puis `#un` (un seul espace, et où l'on trouve « Se connecter à un serveur »), `#deux` (deux espaces, sur ce poste), `#trois` (trois espaces, sur Travail), `#serveur` (la barre de titre et un terminal sur un serveur), `#coupure` (le tunnel tombé), `#connexion` (le premier appairage), `#erreur` (le serveur injoignable), `#projet` (« Nouveau projet » sur un serveur) ou `#appareils` (les réglages des appareils). `?d=x#hote`, `#etats`, `#version`, `#jeton`, `#fichier`, `#compte`, `#notifs` : les écrans communs aux trois, où la navigation entre espaces est masquée (seule la machine de la barre de titre, commune, reste). La page prend la taille de la fenêtre du navigateur : ouvre-la en 1440 × 900 ou en 1024 × 728. Jetons de `src/app.css`, polices de l'app (`node_modules`), icônes de `src/lib/icons.ts` (copiées dans `icons.js`, avec sept de plus : `server`, `warn`, `refresh`, `key`, `unlink`, `window`, `tunnel`). Rien n'est chargé d'ailleurs.
- `captures/` : `node design/server-2-3/render.mjs` (136 PNG : 3 directions × 9 écrans, plus les 7 écrans communs, en sombre et en clair `-clair`, à 1440 × 900 et à 1024 × 728 `-1024`). Le script signale les erreurs de la page, une page vide, tout texte qui déborde, tout ce qui sort de la fenêtre, tout ce qu'une boîte de hauteur fixe coupe et un rail tassé sur ses icônes du bas ; la dernière passe ne signale rien. Avec `--sheets`, il écrit aussi des planches (`captures/planches/`, non versionnées) qui montrent les quatre variantes d'un écran côte à côte.

Données fictives : trois espaces, **Ce poste** (DESKTOP-7QK2, projets portail-adherents et atlas-web), **Travail** (hôte SSH `acme-escouade`, escouade.acme.internal ; facturation, api-clients, infra-k8s) et **Perso** (`vps-perso`, vps.example.net ; recettes, blog), leurs agents et ce qui y attend.

## Comment les directions ont été choisies

Avec le skill `impeccable` : la surface est neuve dans un monde visuel établi (`DESIGN.md`, la refonte 2.0, les réglages 2.2), donc seule la composition est ouverte, en mode Operate. Sept structures pour ranger les espaces, de la plus à la moins évidente :

1. **la colonne d'espaces**, à la Slack : une colonne d'icônes à gauche du rail des projets, une par machine (la proposition de `docs/REMOTE.md`) ;
2. le sélecteur de machine dans la barre de titre, en liste déroulante (comme le choix d'un compte ou d'un profil) ;
3. **les espaces rangés dans le rail des projets** : l'espace ouvert en tête de ses projets, les autres repliés en piles ;
4. une fenêtre par espace, la machine dans le cadre, un lanceur dans le tray ;
5. les onglets de machine dans la barre d'état ;
6. **l'aiguillage** : rien à l'écran qu'un bouton dans la barre de titre, qui ouvre une vue de tous les espaces avec ce qui attend dans chacun ;
7. le rail qui pivote : un clic sur l'en-tête du rail remplace les projets par la liste des espaces.

Le tirage d'impeccable (`concept-seed --scope surface --mode operate`, clé `89c623c7`) a distribué les n° 6 (en tête), 3 et 1. Elles sont vraiment différentes : la n° 6 ne met rien de permanent à l'écran, la n° 3 range les espaces avec les projets, la n° 1 leur donne une colonne à eux.

Les challengers du tirage, jugés sur les deux axes, l'identification du public et la clarté du produit :

| Challenger                                 | Verdict | Ce que les directions en gardent                                                                                                                                                                         |
| ------------------------------------------ | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Le portefeuille de billets d'avion         | Écarté  | La discipline des états qui « se tamponnent » sans rien faire disparaître : un serveur révoqué, injoignable ou d'une autre version reste dans la liste avec son état et son geste (Réglages › Serveurs). |
| La carte d'un zoo (des lobes, une côte)    | Écarté  | La règle d'une seule forme qui découpe : l'espace ouvert fait corps avec la fenêtre (la languette de C), les autres restent dans leur colonne.                                                           |
| Le tableau des départs à volets            | Écarté  | Les lignes vivantes rangées par machine, à colonnes fixes : les voies de l'aiguillage (A). Rien de son costume (déjà écarté pour le site le 2026-10-11).                                                 |
| L'annuel de design, le spécimen de Crouwel | Écartés | Rien : du papier et de la typographie d'affiche, contraires au monde de l'app.                                                                                                                           |
| Le flux vertical de vidéos                 | Écarté  | Rien : un élément à la fois, c'est l'inverse d'une vue de tous les espaces.                                                                                                                              |

## Ce qui est commun aux trois

- **Un seul espace ne coûte rien.** Avec ce poste seul, la fenêtre est celle d'aujourd'hui, pixel pour pixel : ni colonne, ni plaque, ni mot de plus. Seul change l'endroit où l'on trouve « Se connecter à un serveur » (`#un`).
- **La machine se lit partout où une erreur coûterait** : la barre de titre (dès deux espaces, « Ce poste » compris), la barre d'état (le tunnel, sa latence), le pied de la barre latérale (« Travail : /home/escouade/projets/… »), l'en-tête d'un terminal (`escouade@vps.example.net`, et l'invite du shell), « Nouveau projet » (« sur Travail », les dossiers du serveur), un fichier envoyé par un agent (« envoyé depuis Travail · copié sur ce poste dans … »), les secrets (« gardés dans un fichier sur Travail »), la connexion d'un compte (« la page s'est ouverte sur ce poste ; le compte sera gardé sur Travail »), les notifications (« Escouade · Travail »).
- **Une plaque par espace**, neutre : un écran pour ce poste, une lettre pour un serveur. La couleur reste aux projets, qui teintent la fenêtre comme avant.
- **Les états de la connexion sont dessinés** : l'hôte SSH pris dans `~/.ssh/config` (`x-hote`), le premier appairage (`connexion`), le serveur injoignable (`erreur`), le tunnel tombé et la reprise (`coupure`, et « Reconnecté à Travail · 12 événements rattrapés » dans `x-notifs`), la version différente (`x-version`), le jeton révoqué (`x-jeton`), la clé d'hôte inconnue et tous les états à la fois dans Réglages › Serveurs (`x-etats`).

## Les trois directions

### A — « L'aiguillage » (n° 6, en tête du tirage)

Rien de permanent. Dès deux espaces, la machine devient **le premier mot de la barre de titre**, un bouton : `[T] Travail acme-escouade ②  ▾ / facturation ▾`, avec le nombre de ce qui attend **dans les autres espaces**. Il ouvre (ou `Ctrl+²`) **l'aiguillage** : une voie par espace, son état (tunnel, latence), ses projets, ce qui y attend (les deux premières demandes), son raccourci (`Alt+1`…) et « Fenêtre » ; `Entrée` y va, `Ctrl+Entrée` l'ouvre dans une nouvelle fenêtre. La vue d'ensemble, la boîte « À toi » et le rail restent ceux de l'espace ouvert.

|                   | Sombre 1440                   | Clair 1024                               |
| ----------------- | ----------------------------- | ---------------------------------------- |
| Un espace         | ![](captures/a-un.png)        | ![](captures/a-un-clair-1024.png)        |
| Deux espaces      | ![](captures/a-deux.png)      | ![](captures/a-deux-clair-1024.png)      |
| Trois espaces     | ![](captures/a-trois.png)     | ![](captures/a-trois-clair-1024.png)     |
| Sur un serveur    | ![](captures/a-serveur.png)   | ![](captures/a-serveur-clair-1024.png)   |
| Tunnel tombé      | ![](captures/a-coupure.png)   | ![](captures/a-coupure-clair-1024.png)   |
| Premier appairage | ![](captures/a-connexion.png) | ![](captures/a-connexion-clair-1024.png) |
| Injoignable       | ![](captures/a-erreur.png)    | ![](captures/a-erreur-clair-1024.png)    |
| Nouveau projet    | ![](captures/a-projet.png)    | ![](captures/a-projet-clair-1024.png)    |
| Appareils         | ![](captures/a-appareils.png) | ![](captures/a-appareils-clair-1024.png) |

- **Pour** : zéro pixel pour un espace, une plaque et un mot pour plusieurs. La machine est **le contrôle même** : on ne change pas d'espace sans lire où l'on va, et c'est le premier mot qu'on lit. L'aiguillage est la vue « Partout » sans en faire une vue permanente (une stratégie qui l'interdit n'y laisse que les nombres). Il tient à 1024 et grandit avec les espaces.
- **Contre** : changer prend deux gestes (ou `Alt+n`) ; ce qui attend ailleurs n'est qu'un nombre tant qu'on n'ouvre pas l'aiguillage ; à 1024, l'hôte quitte la barre de titre (la plaque et le nom restent) ; une surface nouvelle (un panneau de voies) à apprendre.

### B — « Les piles » (n° 3)

Les espaces vivent **dans le rail des projets**. L'espace ouvert a son en-tête (sa plaque et son nom) au-dessus de ses projets ; les autres sont **repliés en piles** sous le « + » : leurs deux premiers projets en cartes décalées, leur plaque devant, ce qui y attend en pastille. Un clic sur une pile l'ouvre et replie l'autre. La vue d'ensemble et la boîte « À toi » deviennent **« Partout »** : tous les espaces, rangés par espace, ce qui attend en tête (la barre de titre dit alors « Tous les espaces ») ; un segment « Partout · Travail » les ramène à l'espace ouvert. « Se connecter à un serveur » est dans le menu du « + ».

|                   | Sombre 1440                   | Clair 1024                               |
| ----------------- | ----------------------------- | ---------------------------------------- |
| Un espace         | ![](captures/b-un.png)        | ![](captures/b-un-clair-1024.png)        |
| Deux espaces      | ![](captures/b-deux.png)      | ![](captures/b-deux-clair-1024.png)      |
| Trois espaces     | ![](captures/b-trois.png)     | ![](captures/b-trois-clair-1024.png)     |
| Sur un serveur    | ![](captures/b-serveur.png)   | ![](captures/b-serveur-clair-1024.png)   |
| Tunnel tombé      | ![](captures/b-coupure.png)   | ![](captures/b-coupure-clair-1024.png)   |
| Premier appairage | ![](captures/b-connexion.png) | ![](captures/b-connexion-clair-1024.png) |
| Injoignable       | ![](captures/b-erreur.png)    | ![](captures/b-erreur-clair-1024.png)    |
| Nouveau projet    | ![](captures/b-projet.png)    | ![](captures/b-projet-clair-1024.png)    |
| Appareils         | ![](captures/b-appareils.png) | ![](captures/b-appareils-clair-1024.png) |

- **Pour** : aucune colonne de plus ; ce qui attend partout se voit d'un coup, et répondre ne demande pas de changer d'espace. Les piles disent à quoi ressemble un espace replié (ses couleurs).
- **Contre** : deux niveaux (espaces et projets) dans une seule colonne de 52 px ; le rail s'allonge d'environ 110 px par espace et devra défiler avec un espace de travail à neuf projets. La vue « Partout » mêle les espaces que tout le reste cloisonne, et c'est elle qu'un poste géré interdit le plus souvent (`servers.aggregateView`) ; « Autoriser » y agit sur une machine que la barre de titre ne nomme pas.

### C — « La colonne » (n° 1, la forme attendue)

Dès deux espaces, **une colonne d'espaces** de 64 px s'ouvre à gauche du rail : une plaque par machine, son nom dessous, ce qui y attend en pastille ; **l'espace ouvert fait corps avec la fenêtre** (sa case prend la couleur du cadre et s'y raccorde, comme une languette). `Alt+1`…`Alt+9`, un clic, ou le menu de la plaque (nouvelle fenêtre, réglages de l'espace, notifications, se déconnecter, retirer). Tout est par espace : rail, vue d'ensemble, boîte « À toi », réglages ; aucune vue « Partout », seulement les pastilles et les notifications. Le « + » de la colonne ouvre « Ajouter un espace » en pleine fenêtre ; avec un seul espace, on le trouve dans Réglages › Serveurs.

|                   | Sombre 1440                   | Clair 1024                               |
| ----------------- | ----------------------------- | ---------------------------------------- |
| Un espace         | ![](captures/c-un.png)        | ![](captures/c-un-clair-1024.png)        |
| Deux espaces      | ![](captures/c-deux.png)      | ![](captures/c-deux-clair-1024.png)      |
| Trois espaces     | ![](captures/c-trois.png)     | ![](captures/c-trois-clair-1024.png)     |
| Sur un serveur    | ![](captures/c-serveur.png)   | ![](captures/c-serveur-clair-1024.png)   |
| Tunnel tombé      | ![](captures/c-coupure.png)   | ![](captures/c-coupure-clair-1024.png)   |
| Premier appairage | ![](captures/c-connexion.png) | ![](captures/c-connexion-clair-1024.png) |
| Injoignable       | ![](captures/c-erreur.png)    | ![](captures/c-erreur-clair-1024.png)    |
| Nouveau projet    | ![](captures/c-projet.png)    | ![](captures/c-projet-clair-1024.png)    |
| Appareils         | ![](captures/c-appareils.png) | ![](captures/c-appareils-clair-1024.png) |

- **Pour** : la forme que tout le monde connaît (Slack, Discord) ; chaque espace a sa pastille en permanence ; un clic suffit ; le cloisonnement se voit (une colonne à part, une languette). C'est la plus simple à expliquer et à construire.
- **Contre** : 64 px pris en permanence dès deux espaces (6 % de la largeur à 1024), pour ce qui est le plus souvent un aller-retour entre deux machines ; deux colonnes de tuiles côte à côte (espaces, projets) ; la machine se lit d'abord dans une colonne, pas dans la barre de titre ; rien ne montre ce qui attend ailleurs au-delà d'un nombre.

## Les écrans communs

|                        | Sombre 1440                 | Clair 1024                             |
| ---------------------- | --------------------------- | -------------------------------------- |
| L'hôte SSH             | ![](captures/x-hote.png)    | ![](captures/x-hote-clair-1024.png)    |
| Les serveurs et états  | ![](captures/x-etats.png)   | ![](captures/x-etats-clair-1024.png)   |
| Version différente     | ![](captures/x-version.png) | ![](captures/x-version-clair-1024.png) |
| Jeton révoqué          | ![](captures/x-jeton.png)   | ![](captures/x-jeton-clair-1024.png)   |
| Fichier d'un agent     | ![](captures/x-fichier.png) | ![](captures/x-fichier-clair-1024.png) |
| Compte et secret       | ![](captures/x-compte.png)  | ![](captures/x-compte-clair-1024.png)  |
| Notifications, reprise | ![](captures/x-notifs.png)  | ![](captures/x-notifs-clair-1024.png)  |

## Recommandation

**A, l'aiguillage**, avec deux emprunts à C : le menu de l'espace (nouvelle fenêtre, réglages, notifications, se déconnecter, retirer) au clic droit sur la machine de la barre de titre, et une info-bulle qui détaille, espace par espace, le nombre de l'aiguillage. Les raisons sont dans [`docs/SERVER-UX.md`](../../docs/SERVER-UX.md) § 6.

## Ce qui a été construit : la direction B (ESC-539)

L'utilisateur a choisi **B, les piles**, le 2026-11-18. Les captures de l'app construite sont dans [`espaces/`](espaces/). Elles sont prises par l'e2e `tests/e2e/spaces.spec.ts` (`ESCOUADE_E2E_SHOTS`), à 1440 et à 1024, en sombre et en clair, contre deux `escouade-server`.

| Écran | Capture | Maquette |
| --- | --- | --- |
| Deux espaces, ce poste en pile | `espaces/espaces-deux.png` | `captures/b-deux.png` |
| Trois espaces, les piles (plaques à 1024) | `espaces/espaces-piles.png`, `espaces/espaces-piles-1024.png` | `captures/b-serveur.png` |
| « Partout », la machine sur chaque demande | `espaces/espaces-partout.png` | `captures/b-trois.png` |
| Un espace coupé, sans geste | `espaces/espaces-coupe.png` | `captures/b-coupure.png` |
| La vue agrégée interdite : des nombres | `espaces/espaces-masque.png` | réglage de `captures/x-etats.png` |
| Réglages de l'espace › Appareils | `espaces/espaces-appareils.png` | `captures/b-appareils.png` |

`espaces/memoire.txt` donne la mémoire mesurée, avec un espace puis avec trois connectés.

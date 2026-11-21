# ESC-549 — l'app mobile (MO-0), trois directions

Demande de l'utilisateur (2026-11-20), en choisissant « L'app mobile (Capacitor, d'après l'étude ESC-445) » : la conception avec Impeccable, des directions à choisir, puis les premiers tickets. Précisé au début du ticket : **répondre vite d'abord**. La conception (ce que fait l'app, pour qui, les parcours, ce qu'elle ne fait pas, l'hors ligne), les choix (M1 à M16) et le découpage (MO-1 à MO-11) sont dans [`docs/MOBILE.md`](../../docs/MOBILE.md).

**L'utilisateur a choisi B, le fil**, le 2026-11-20, après avoir vu ces planches, et la création depuis le téléphone (un projet sur un serveur, un agent, un ticket dicté), rendue ici en B.

Rien du produit ne change dans ce ticket : ce sont des maquettes statiques pour choisir une direction. Aucun code de l'app.

- `index.html` : **la page qui compare les trois directions côte à côte**, écran par écran, en sombre ou en clair, à 430 (iPhone) ou à 360 (Android), avec une table qui les résume. Elle lit les images de `captures/`. Sa première vue : `captures/comparaison.png` et `comparaison-clair.png`.
- `maquette.html` : les trois directions. `?d=a|b|c`, `&theme=light`, puis `#appairage` (le premier appairage par QR code), `#accueil` (ce qui attend, partout), `#autorisation` (une autorisation, la commande entière et le geste), `#conversation` (lire, envoyer, dicter), `#boite` (un choix entre trois rendus de la boîte « À toi »), `#kanban` (en lecture, un geste simple) ou `#notification` (l'écran verrouillé) ; en B seulement, `#creer-projet`, `#creer-agent`, `#ticket-dicte` (la création). `?d=x#verrou`, `#horsligne`, `#revoque` : les écrans communs aux trois. La page prend la taille de la fenêtre : 360 × 780 dessine un Android (sa barre d'état, l'invite de l'empreinte), 430 × 932 un iPhone (l'île, Face ID). Jetons de `src/app.css`, polices de l'app (`node_modules`), icônes de `src/lib/icons.ts` (copiées dans `icons.js`, avec douze de plus : `qr`, `scan`, `face`, `finger`, `send`, `camera`, `chev-r`, `chev-l`, `chev-d`, `hand`, `offline`, `revoke`). Rien n'est chargé d'ailleurs.
- `captures/` : `node design/mobile-2-3/render.mjs` (110 PNG : 3 directions × 7 écrans, les 3 écrans de création en B et les 3 écrans communs, en sombre et en clair `-clair`, à 360 × 780 `-360` et à 430 × 932 `-430`, au double des pixels d'un écran ; plus la page de comparaison). Le script signale les erreurs de la page, une page vide, tout texte qui déborde, ce qui sort de l'écran, ce qu'une boîte de hauteur fixe coupe et un geste poussé hors de sa carte ; la dernière passe ne signale rien. Avec `--sheets`, il écrit des planches (`captures/planches/`, non versionnées) qui montrent les quatre variantes d'un écran côte à côte.

**Données fictives** : trois espaces, **Ce poste** (DESKTOP-7QK2 ; portail-adherents, atlas-web), **Travail** (escouade.acme.internal ; facturation, api-clients, infra-k8s) et **Perso** (vps.example.net, hors ligne), leurs agents et ce qui y attend. La commande montrée est inventée et ne porte aucun secret ; le QR code dessiné est un motif au hasard, qui ne contient rien. Aucun compte, aucune clé.

## Comment les directions ont été choisies

Avec le skill `impeccable` : une surface neuve dans un monde visuel établi (`DESIGN.md`, la refonte 2.0), donc seule la composition est ouverte, en mode Operate, à l'échelle du doigt (corps de 15 px, cibles de 44 et 48 px, zones sûres, retour du système). Sept structures pour le téléphone, de la plus à la moins attendue :

1. des onglets en bas (À toi, Agents, Kanban, Réglages) ;
2. une messagerie : un fil par agent, la demande en dernier message ;
3. la file de la boîte en plein écran, une demande par écran ;
4. un tableau de bord sur une page ;
5. **le fil** : la notification d'abord, la ligne a sa forme ;
6. **la pile** : les demandes en paquet de cartes, réglées d'un geste ;
7. **les espaces empilés**, comme un portefeuille.

Le tirage d'impeccable (`concept-seed --scope surface --mode operate`, clé `cade135e`) a distribué les n° 6 (en tête), 5 et 7. Les challengers et ce que les directions en gardent sont dans `docs/MOBILE.md` § 6.

## Les trois directions

Chacune montre les mêmes écrans, avec les mêmes données ; seules changent la navigation, la densité, la place de la boîte face aux agents et le geste de la réponse.

### A — « La pile » (n° 6, en tête du tirage ; ma recommandation d'abord)

Une demande à la fois, en grande carte, la pile dessous et la jauge au-dessus (« 1 sur 6 », une case par demande, ambre pour un agent arrêté). Une autorisation se règle d'un **glissé** jusqu'au bout (la carte passe de l'ambre au vert sous le pouce), puis le verrou du système ; un choix d'un toucher. Les agents au travail, le Kanban et les espaces sont dans **une feuille du bas**. « Partout » ou un espace se choisit en haut.

|              | Sombre 430                           | Clair 360                                  |
| ------------ | ------------------------------------ | ------------------------------------------ |
| Appairage    | ![](captures/a-appairage-430.png)    | ![](captures/a-appairage-clair-360.png)    |
| Accueil      | ![](captures/a-accueil-430.png)      | ![](captures/a-accueil-clair-360.png)      |
| Autorisation | ![](captures/a-autorisation-430.png) | ![](captures/a-autorisation-clair-360.png) |
| Conversation | ![](captures/a-conversation-430.png) | ![](captures/a-conversation-clair-360.png) |
| Boîte        | ![](captures/a-boite-430.png)        | ![](captures/a-boite-clair-360.png)        |
| Kanban       | ![](captures/a-kanban-430.png)       | ![](captures/a-kanban-clair-360.png)       |
| Notification | ![](captures/a-notification-430.png) | ![](captures/a-notification-clair-360.png) |

### B — « Le fil » (n° 5 ; choisie par l'utilisateur le 2026-11-20)

Un fil dense de tout ce qui attend, rangé comme la boîte du bureau (« Tes agents se sont arrêtés », « Aujourd'hui », « Plus tôt »), des puces d'espace en haut, **trois onglets en bas** (À toi, Agents, Kanban). Une ligne **se déplie sur place** : la commande, puis « Refuser » et « Autoriser » (des boutons), puis le verrou. La ligne a la forme de la notification, qui part une par agent arrêté. Le Kanban : une colonne choisie, une ligne glissée de côté pour valider ou renvoyer.

|              | Sombre 430                           | Clair 360                                  |
| ------------ | ------------------------------------ | ------------------------------------------ |
| Appairage    | ![](captures/b-appairage-430.png)    | ![](captures/b-appairage-clair-360.png)    |
| Accueil      | ![](captures/b-accueil-430.png)      | ![](captures/b-accueil-clair-360.png)      |
| Autorisation | ![](captures/b-autorisation-430.png) | ![](captures/b-autorisation-clair-360.png) |
| Conversation | ![](captures/b-conversation-430.png) | ![](captures/b-conversation-clair-360.png) |
| Boîte        | ![](captures/b-boite-430.png)        | ![](captures/b-boite-clair-360.png)        |
| Kanban       | ![](captures/b-kanban-430.png)       | ![](captures/b-kanban-clair-360.png)       |
| Notification | ![](captures/b-notification-430.png) | ![](captures/b-notification-clair-360.png) |

### C — « Les espaces » (n° 7)

Les espaces **empilés comme des cartes** : les autres en bandeau (leur nombre, « hors ligne »), celui du dessus ouvert, avec ses demandes en tuiles qui défilent de côté, ses agents au travail et, en bas, À toi · Agents · Kanban. Une demande s'ouvre dans **une feuille** qui nomme sa machine ; une autorisation se règle en **maintenant** une seconde, puis le verrou. Les notifications sont groupées par espace. Le Kanban : une rangée par colonne, le ticket choisi s'agrandit avec ses gestes.

|              | Sombre 430                           | Clair 360                                  |
| ------------ | ------------------------------------ | ------------------------------------------ |
| Appairage    | ![](captures/c-appairage-430.png)    | ![](captures/c-appairage-clair-360.png)    |
| Accueil      | ![](captures/c-accueil-430.png)      | ![](captures/c-accueil-clair-360.png)      |
| Autorisation | ![](captures/c-autorisation-430.png) | ![](captures/c-autorisation-clair-360.png) |
| Conversation | ![](captures/c-conversation-430.png) | ![](captures/c-conversation-clair-360.png) |
| Boîte        | ![](captures/c-boite-430.png)        | ![](captures/c-boite-clair-360.png)        |
| Kanban       | ![](captures/c-kanban-430.png)       | ![](captures/c-kanban-clair-360.png)       |
| Notification | ![](captures/c-notification-430.png) | ![](captures/c-notification-clair-360.png) |

## La création, en B

Choisie avec la direction (M16) : un projet sur un espace serveur (un dépôt cloné sur le serveur ou un dossier du serveur, par ce que fait déjà le client de bureau, le clonage suivi), un agent avec son premier message, un ticket dicté dont le titre et les critères se relisent avant de créer.

|                           | Sombre 430                           | Clair 360                                  |
| ------------------------- | ------------------------------------ | ------------------------------------------ |
| Un projet sur un serveur  | ![](captures/b-creer-projet-430.png) | ![](captures/b-creer-projet-clair-360.png) |
| Un agent, son premier mot | ![](captures/b-creer-agent-430.png)  | ![](captures/b-creer-agent-clair-360.png)  |
| Un ticket dicté           | ![](captures/b-ticket-dicte-430.png) | ![](captures/b-ticket-dicte-clair-360.png) |

## Les écrans communs

|                                   | Sombre 430                        | Clair 360                               |
| --------------------------------- | --------------------------------- | --------------------------------------- |
| Le verrou (Face ID, l'empreinte)  | ![](captures/x-verrou-430.png)    | ![](captures/x-verrou-clair-360.png)    |
| Hors ligne : daté, envoi suspendu | ![](captures/x-horsligne-430.png) | ![](captures/x-horsligne-clair-360.png) |
| Révoqué depuis le poste           | ![](captures/x-revoque-430.png)   | ![](captures/x-revoque-clair-360.png)   |

## Le choix

**B, le fil**, choisi par l'utilisateur le 2026-11-20 (ma recommandation était A). Ce que B garde de A et de C, et ce qu'il demande de plus, est dans [`docs/MOBILE.md`](../../docs/MOBILE.md) § 6.

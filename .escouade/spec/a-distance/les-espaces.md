---
title: "Les espaces"
subject: "a-distance"
sources: ["docs/SERVER-UX.md (§ 5 les espaces, § 6 les trois directions, § 7 S1 à S16, § 8 MS-5, § 10, § 11)","design/server-2-3/maquette.html?d=b et ses captures b-* (b-un, b-deux, b-trois, b-serveur, b-coupure, b-appareils)","docs/REMOTE.md, « Plusieurs serveurs pilotés par un seul Escouade », « Les espaces, MS-5 (ESC-539) »","src-tauri/src/servers (mod.rs, window.rs), src-tauri/src/glance.rs","src/lib/spaces.svelte.ts, src/components/servers (RailSpace, Everywhere), src/components/settings/DevicesPage.svelte","src-tauri/src/servers_tests.rs, src-tauri/src/glance_tests.rs, src/components/servers/spaces.test.ts, tests/e2e/spaces.spec.ts"]
tickets: ["ESC-539","ESC-538"]
decisions: [218]
compared: null
---

# Les espaces

Un **espace** par machine : **Ce poste** (son cœur est dans l'app) et chaque serveur que ce poste connaît (voir *Se connecter à un serveur*). Un espace a ses projets, ses agents, ses comptes, ses plugins, ses réglages et son serveur MCP ; rien ne passe de l'un à l'autre. Ce qui est à l'app sur ce poste : les serveurs connus, l'apparence et la langue, la voix, les notifications, les raccourcis, les mises à jour du client.

La direction retenue est **B, « les piles »** (choix S1 de l'utilisateur, le 2026-11-18) : les espaces dans la barre des projets, l'espace ouvert en tête de ses projets, les autres repliés en piles ; la vue d'ensemble devient **« Partout »**, filtrable par espace, et l'on y répond **sans changer d'espace**.

## Un seul espace

Tant que ce poste ne connaît aucun serveur, la fenêtre est celle d'avant, au pixel près : ni machine dans la barre de titre, ni pile, ni onglet « Partout », et les raccourcis des espaces ne prennent rien. « Se connecter à un serveur » reste dans Réglages › Serveurs et dans le menu du « + ».

## Plusieurs connexions

- **Chaque serveur connu garde son tunnel** SSH, ouvert au démarrage de l'app et tenu pendant qu'elle tourne, avec **ses propres essais** (250 ms, 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s). Un tunnel qui tombe ne touche pas les autres ; aller sur ce poste ne les ferme pas. Un serveur que la stratégie refuse, ou dont ce poste n'a plus de jeton, n'est pas joint, et son état le dit.
- **La fenêtre lit chaque autre espace par une connexion à lui** : une WebSocket de l'API distante par son tunnel (avec sa reprise, comme celle de la fenêtre), ou le cœur de ce poste quand la fenêtre est sur un serveur. Elle lui demande toutes les 3 s ce qui y attend (`space_glance`) : le nombre de demandes, les agents qui attendent (leur première demande, son argument coupé à 400 caractères) et ceux au travail (40 au plus), les demandes de la boîte « À toi », les couleurs de ses trois premiers projets. Jamais sa conversation ni son état entier.
- **Mémoire** : trois espaces connectés (ce poste et deux serveurs) ajoutent à la fenêtre deux WebSockets et un résumé de quelques kilo-octets par espace, et au poste un processus `ssh` par serveur. La mesure est faite par l'e2e `spaces` et notée dans son bilan.

## La barre des projets

- **L'espace ouvert** est en tête de ses projets : sa plaque (un écran pour ce poste, la lettre du nom pour un serveur) et son nom court (« Poste », « Travail ») ; un clic, ou le clic droit, ouvre son menu.
- **Chaque autre espace** est une pile sous le « + » : deux de ses projets en cartes décalées derrière sa plaque, ce qui y attend dans une pastille ambre, son nom dessous. Un clic y mène. Une pile coupée a son nom en ambre ; sa bulle et son nom pour un lecteur d'écran disent « Travail, 2 en attente (Alt 2) » ou « Perso, coupé (Alt 3) ».
- **Les piles se tassent en simples plaques** (la plaque et la pastille, sans les cartes) **au-delà de trois espaces, ou à 1024 px de large** : la barre ne défile jamais pour elles.
- **Le menu d'un espace** (clic droit sur une pile, clic sur l'espace ouvert) : « Aller sur … » (avec son raccourci), « Ouvrir dans une fenêtre », « Réessayer » s'il est coupé, « Réglages de … » pour l'espace ouvert d'un serveur (sa page Appareils), et « Retirer … de ce poste… » (Réglages › Serveurs, après confirmation).
- Les espaces sont une `navigation` « Espaces » ; chaque pile porte `aria-keyshortcuts`.

## « Partout »

- La vue d'ensemble (`Ctrl+Maj+A`) a, dès deux espaces, des onglets : **« Partout · 3 espaces »** (par défaut), puis un par espace. L'onglet de l'espace ouvert est la vue d'ensemble d'avant ; « Partout » et l'onglet d'un autre espace montrent ce qui attend, espace par espace, l'espace ouvert d'abord. La barre de titre dit alors « Tous les espaces / Vue d'ensemble ».
- **Chaque espace sous sa machine** : sa plaque, son nom, son adresse et sa latence (« escouade.acme.internal · 38 ms »), et « 2 attendent ». Dessous, une ligne par agent qui attend (« Autorisation · Bash : psql … », « Question · … ») puis par agent au travail (ce qu'il fait), et les demandes de la boîte « À toi ».
- **On y répond sans changer d'espace** : une autorisation lisible en entier (ni coupée, ni refusée par défaut, ni un plan) s'accorde par **« Autoriser sur Travail »**, envoyé à ce serveur-là par sa propre connexion. Une question, un plan, une demande coupée ou refusée par défaut se répondent dans leur conversation : « Ouvrir » y mène (la fenêtre passe sur l'espace et ouvre l'agent).

### Les trois garde-fous

1. **La machine est écrite sur chaque demande et sur le geste qui y répond** : chaque ligne porte la plaque et le nom de son espace, le bouton dit « Autoriser sur Travail » (et, pour un lecteur d'écran, « Autoriser sur Travail la demande de relances-impayes »), pour qu'on n'autorise jamais une commande sur le mauvais serveur. **Un espace coupé** est en ambre, « reconnexion » et « ce que tu vois date de 14:32 » dans son en-tête, avec « Réessayer » ; ses lignes sont grisées, **sans aucun geste**.
2. **`servers.aggregateView` à `false`** : « Partout » ne montre d'un serveur de travail que ses nombres (« 2 en attente · 1 au travail »), ni texte ni geste, et « Le détail ne s'affiche que dans Travail » avec « Aller sur Travail ». Un serveur de travail est un serveur de `servers.allowed` quand ce poste en pose la liste, sinon tout serveur ; jamais ce poste. Les textes ne viennent même pas à la fenêtre : elle demande à ce serveur ses seuls nombres. Posée sur un serveur, la stratégie vaut pour lui : son cœur ne donne que ses nombres, quoi qu'on lui demande. **`servers.contentlessNotifications`** retire le texte des notifications d'un serveur de travail : « Un agent attend ta réponse (le détail ne s'affiche que dans Escouade) ».
3. **Les piles se tassent en plaques** au-delà de trois espaces ou à 1024 px (ci-dessus), mesuré aux deux tailles par l'e2e.

## Passer d'un espace à l'autre

- Un clic sur une pile, « Aller sur … », « Ouvrir » de « Partout », une notification, ou les raccourcis : **`Alt+1`…`Alt+9`** (`⌃1`…`⌃9` sous macOS) pour le n-ième espace, ce poste étant le premier, et **`Ctrl+²`** (la touche sous Échap) pour l'espace suivant. Ils se lisent par la place de la touche (AZERTY compris), jamais avec `Maj` ni AltGr, ni depuis le pavé numérique ; ils valent aussi dans un terminal et dans la vue VS Code, pas dans un dialogue. `Ctrl+1`…`9` restent aux projets. **Une question à l'écran garde `Alt+1`…`9` pour ses options**, comme avant.
- Passer d'un espace à l'autre recharge la fenêtre sur l'autre cœur ; l'app rouvre sur le dernier espace où elle était.

## Un espace dans sa fenêtre

- « Ouvrir dans une fenêtre », du menu d'un espace : une fenêtre de plus, sur cet espace, avec sa barre de titre et sa machine. **Une fenêtre par espace au plus** : la demander de nouveau ramène la première ; demander l'espace où est déjà la fenêtre principale la ramène. Aller sur un espace qui a sa fenêtre ramène cette fenêtre au lieu de recharger la principale.
- Elle ne lit pas les autres espaces (ni piles ni « Partout ») et ne change pas d'espace ; la fermer la ferme (seule la fenêtre principale va dans la zone de notification).

## Les notifications

- Une demande nouvelle d'un autre espace serveur est notifiée par la fenêtre, **son espace en tête** : « Travail · facturation · relances-impayes », « Autorisation · Bash : psql … ». Le son, la notification du système et ce qui se notifie suivent les réglages de ce poste ; la fenêtre au premier plan ne reçoit que le son. Un clic ramène la fenêtre et la mène à l'agent, dans son espace.
- Les notifications du cœur de ce poste disent « Ce poste · … » dès qu'un serveur est connu.

## Réglages de l'espace › Appareils

- Sur un serveur, la page **Appareils** des réglages : les appareils appairés au serveur, avec leur dernier passage ; **ce poste marqué « Ce poste »**, son geste « Se déconnecter » menant à Réglages › Serveurs (où « Retirer » oublie le serveur et le prie de révoquer ce poste) ; **« Retirer »** un autre appareil, après confirmation : son jeton ne vaut plus, ses connexions se ferment dans les deux secondes, la trace du serveur dit qui l'a retiré.
- **« Créer un code »** (S5) : un code d'appairage de 10 minutes, pour un seul appareil, créé depuis un client déjà appairé (le chemin du téléphone) ; il remplace celui qui attendait.

## Ce que le serveur sert pour les espaces

Commandes servies (`remoteapi/list.rs`) : `space_glance` (ce qui attend, `counts` pour les seuls nombres), `remote_devices`, `remote_device_revoke`, `remote_pair_code`. Commandes du bureau : `server_links`, `server_link` d'une fenêtre d'espace, `server_retry` d'un espace, `space_window`, `space_notify`.

## Avec ce qui change de machine (ESC-538)

- **Les ports relayés** d'un serveur (un serveur de test, le retour d'une connexion de compte) appartiennent au serveur où est la fenêtre : ils s'arrêtent quand elle va sur un autre espace, revient sur ce poste ou que l'app se ferme. Les tunnels des espaces, eux, restent.
- **Ce que publie le serveur où est la fenêtre** (une notification, un son, le compte de ce qui attend) vient de la connexion de la fenêtre et se montre nommé par son espace. Ce que publient les autres serveurs n'est pas repris de leur connexion : leurs demandes nouvelles sont notifiées par la lecture de « Partout », nommées par leur espace. Rien n'est donc notifié deux fois, ni sous le nom d'un autre espace.

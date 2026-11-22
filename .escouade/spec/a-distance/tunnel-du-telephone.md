---
title: "Le tunnel du téléphone"
subject: "a-distance"
sources: ["docs/MOBILE.md § 3.1, § 3.10, § 8 (MO-2), M12","docs/REMOTE.md, « Le téléphone » (« Architecture retenue », « Configuration, pour l'utilisateur », « Points durs (sur le PC) », « Alternative sans WireGuard dans l'app », « Le tunnel côté cœur, MO-2 »)","src-tauri/src/phone (mod.rs, stack.rs, nat.rs, tailscale.rs)","src-tauri/src/remoteapi/server.rs (Via, serve_stream, Listen::Nothing)","src/components/settings/PhonePage.svelte","crates/escouade-policy/src/list.rs (servers.phone)"]
tickets: ["ESC-567","ESC-630"]
decisions: [230,262,285]
compared: null
---

# Le tunnel du téléphone

Le téléphone pilote un cœur d'Escouade (le poste, ou un serveur) par l'API distante (voir *L'API distante*), servie dans un **tunnel WireGuard intégré à Escouade**. Le côté téléphone (l'app, sa clé, le QR code de l'appairage) vient avec l'app mobile (voir *L'app mobile*) ; ce qui suit est le côté du cœur, sur le poste comme dans `escouade-server`.

## Un pair WireGuard dans Escouade

- Escouade embarque un pair WireGuard **en espace utilisateur** sur un port UDP de la machine, et sa propre pile TCP/IP dans le tunnel. Aucune interface réseau du système, aucun pilote, aucun VPN de la machine : **rien ne demande les droits d'administrateur**, et seul le trafic d'Escouade passe dans le tunnel.
- Le cœur a l'adresse `10.77.0.1` dans le tunnel ; chaque téléphone la sienne, de `10.77.0.2` à `10.77.0.254`, la seule d'où ses paquets sont pris.
- **Un pair inconnu n'a aucune réponse** : une poignée de main d'une clé que le cœur ne connaît pas, ou un datagramme qui n'est pas WireGuard, est jeté sans un mot. Un scan du port ne voit rien. Sous une rafale de poignées de main, le cœur demande les cookies de WireGuard.
- **L'API n'est servie que dans le tunnel** pour un téléphone, à `ws://10.77.0.1/remote`. Sur le poste, quand l'accès est allumé, l'API n'a alors **aucun port sur la machine** : elle n'est jointe que par le tunnel, jamais par l'adresse publique ni le réseau local. Sur un serveur, elle garde en plus son port de 127.0.0.1 (le tunnel SSH du poste, voir *Le mode serveur*).
- Dans le tunnel aussi, **le jeton d'appareil est demandé** (défense en profondeur si la clé d'un téléphone fuit), le `Host` doit être l'adresse du tunnel, et seule l'`Origin` de la fenêtre d'Escouade ou de l'app mobile (`capacitor://localhost`, `https://localhost`, `http://localhost`) passe ; ces dernières restent refusées sur 127.0.0.1, où elles seraient une page du poste.
- **Un téléphone appairé à travers le tunnel** (le code de l'appairage donné par son pair) appartient à cet appareil : par son pair, seul le jeton de cet appareil est pris. **Révoquer l'appareil retire aussi son pair** : sa clé n'est plus reconnue.
- **La clé du tunnel** peut être changée (Réglages › Téléphone › Avancé › « Changer la clé du tunnel », après confirmation) : chaque téléphone doit être appairé de nouveau, l'ancienne clé n'a plus de réponse.

## Ce qui est gardé

- `phone.json`, dans le dossier des données, lisible du seul propriétaire : l'accès allumé ou non, le port UDP, Tailscale ou non, les téléphones (nom, clé publique, adresse dans le tunnel, l'appareil qu'ils sont), l'adresse publique vue et quand elle a changé.
- **La clé privée du tunnel** au trousseau du système ; sur un serveur, dans son fichier de secrets ; dans `phone.json` quand le trousseau la refuse. La clé privée d'un téléphone n'est jamais gardée par le cœur.

## Le port et la box

- Le port UDP est **51820** par défaut (celui de WireGuard), modifiable. Pris par un autre programme, l'accès ne s'allume pas et le dit : « Le port UDP 51820 est déjà pris sur cette machine (un autre WireGuard ?) : choisis-en un autre. »
- Allumé sur le poste, Escouade **essaie d'ouvrir le port dans la box**, par UPnP puis NAT-PMP, pour une heure renouvelée tant que l'accès est allumé, et le retire quand on l'éteint. Il dit s'il y est arrivé (« Port UDP 51820 ouvert dans la box par UPnP »), sinon **le geste à faire**, avec l'adresse de la machine : « Dans la box, redirige le port UDP 51820 vers 192.168.1.20 (cette machine) », et ce que la box a répondu.
- **L'adresse publique** est demandée à un serveur STUN (une requête de liaison, rien d'autre ne part : `stun.cloudflare.com`, puis `stun.l.google.com`), avec l'adresse que la box dit avoir sur Internet. Elle est **relue toutes les cinq minutes** ; quand elle change, Réglages › Téléphone le dit (« Changée le … (avant : …) : un téléphone appairé avant elle doit reprendre la nouvelle »).
- **Un CGNAT est détecté et dit** : quand l'adresse de la box est partagée par l'opérateur (100.64.0.0/10) ou privée (une box derrière une autre), ou n'est pas l'adresse vue de dehors, une redirection n'y fait rien. L'écran le dit, avec les issues : une IPv4 dédiée demandée à l'opérateur, l'IPv6 de la box, Tailscale, un serveur Escouade sur un VPS.
- Une **IPv6 publique** de la machine est dite, avec le port à ouvrir dans le pare-feu IPv6 de la box.
- **Sur un serveur**, il n'y a pas de box : rien n'est demandé à une box, et l'écran dit d'ouvrir le port dans son pare-feu (« ufw allow 51820/udp »). Une machine qui a l'adresse publique elle-même est dite comme telle.

## Tailscale quand il est là

Quand Tailscale tourne sur la machine (sa commande, ou la route vers son adresse 100.100.100.100), Escouade le dit, avec l'adresse et le nom de la machine dans le tailnet, et **sert aussi l'API sur l'adresse Tailscale**, au même numéro de port en TCP : un téléphone de ce tailnet la joint sans port à ouvrir, CGNAT compris. Seule une machine du tailnet est prise (100.64.0.0/10, `fd7a:115c:a1e0::/48`), le jeton d'appareil reste demandé. « Servir aussi sur l'adresse Tailscale » se coupe dans Réglages › Téléphone. Escouade ne refait pas Tailscale : ni perçage de NAT, ni relais.

## Réglages › Téléphone

La page dit, pour le cœur où est la fenêtre (le poste, ou le serveur de son espace : la même page y agit sur le serveur) :

- **l'accès** (allumé ou éteint) et **le port UDP** ;
- **l'état** : éteint, prêt sur le port, ne démarre pas (et pourquoi), l'adresse de l'API dans le tunnel, le nombre de téléphones ;
- **la box** : ouvert par UPnP ou NAT-PMP, ou le geste à faire, un CGNAT et ses issues, l'IPv6, l'adresse publique et son changement, « Revoir le réseau » ; sur un serveur, son pare-feu ;
- **Tailscale** : là ou non, son adresse, l'interrupteur ;
- **les téléphones** : leur nom, leur dernière poignée de main (« Tunnel ouvert · dernière poignée de main il y a 3 min » ou « Tunnel au repos »), leur adresse, « Retirer » ; sans téléphone : « L'app d'Escouade pour Android et iOS les appairera par un QR code » ;
- **Avancé** : la clé publique du tunnel, « Changer la clé du tunnel ».

## La stratégie du poste

`servers.phone` à `false` interdit le tunnel (voir *Réglages imposés*), comme `servers.remoteApi` à `false` : l'accès est refusé à l'allumage, un tunnel allumé **s'éteint aussitôt** (à la différence d'une connexion de l'API déjà ouverte, qui continue), le port est retiré de la box et l'API n'est plus servie sur l'adresse Tailscale. L'interrupteur de Réglages › Téléphone est grisé, avec son cadenas et qui l'impose. La stratégie retirée, l'accès revient s'il était allumé.

## Sur un serveur

`escouade-server phone` règle le tunnel du serveur depuis son terminal : `status`, `on [PORT]`, `off`, `remove ID`, et `add NOM [--endpoint HÔTE]`, qui ajoute un téléphone et affiche sa config WireGuard et son QR code (pour l'app WireGuard officielle, en attendant l'appairage de l'app mobile) ; cette config porte la clé privée du téléphone, montrée une fois et gardée nulle part. Le serveur qui tourne relit le fichier dans les deux secondes. Une page ouverte dans le tunnel qui n'est pas l'API est dite dans son journal.

## Le journal

Le tunnel dit son port, chaque poignée de main d'un téléphone, chaque téléphone ajouté ou retiré, un changement de clé ou d'adresse publique, l'ouverture du port dans la box et son refus ; jamais une clé privée, ni ce qui passe dans le tunnel.

## Pas encore

L'essai réel avec un téléphone en 4G (un geste de l'utilisateur) ; le QR code de l'appairage depuis Réglages › Téléphone et la sorte « téléphone » des appareils ; l'app elle-même ; un nom DynDNS à la place de l'adresse publique ; le pare-feu IPv6 de la box ouvert par Escouade ; empêcher la veille du poste tant que l'accès est allumé.

## Retirer un téléphone (ESC-630)

« Retirer » un téléphone, dans Réglages › Téléphone ou par `escouade-server phone remove ID`, retire son pair **et révoque l'appareil de l'API distante qui lui est lié** (appairé à travers lui), avec les appareils nés de lui : son jeton ne vaut plus nulle part, ni par Tailscale, ni par la boucle locale d'un serveur, ni par un autre pair. L'écran le dit : « Pixel retiré et son accès révoqué ». Un pair qui n'a jamais été lié à un appareil est retiré du tunnel, et l'écran dit qu'aucun jeton n'était lié à lui.

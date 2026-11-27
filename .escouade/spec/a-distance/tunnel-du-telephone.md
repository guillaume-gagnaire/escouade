---
title: "Le tunnel du téléphone"
subject: "a-distance"
sources: ["docs/MOBILE.md § 3.1, § 3.10, § 8 (MO-2), M12","docs/REMOTE.md, « Le téléphone » (« Architecture retenue », « Configuration, pour l'utilisateur », « Points durs (sur le PC) », « Alternative sans WireGuard dans l'app », « Le tunnel côté cœur, MO-2 »)","src-tauri/src/phone (mod.rs, stack.rs, nat.rs, tailscale.rs)","src-tauri/src/remoteapi/server.rs (Via, serve_stream, Listen::Nothing)","src/components/settings/PhonePage.svelte","crates/escouade-policy/src/list.rs (servers.phone)","Revue de sécurité de la 2.3 (ESC-624), rapport privé SR-1 « Réseau et téléphone », SR1-05 et SR1-11 (hors du dépôt)","website/content/privacy (fr.md, en.md) : « L’accès du téléphone, si tu l’allumes »"]
tickets: ["ESC-567","ESC-630","ESC-631","ESC-641","ESC-570"]
decisions: [230,262,285,328,333,348]
compared: null
---

# Le tunnel du téléphone

Le téléphone pilote un cœur d'Escouade (le poste, ou un serveur) par l'API distante (voir *L'API distante*), servie dans un **tunnel WireGuard intégré à Escouade**. Le côté téléphone (l'app, sa clé, le QR code de l'appairage) vient avec l'app mobile (voir *L'app mobile*) ; ce qui suit est le côté du cœur, sur le poste comme dans `escouade-server`.

## Un pair WireGuard dans Escouade

- Escouade embarque un pair WireGuard **en espace utilisateur** sur un port UDP de la machine, et sa propre pile TCP/IP dans le tunnel. Aucune interface réseau du système, aucun pilote, aucun VPN de la machine : **rien ne demande les droits d'administrateur**, et seul le trafic d'Escouade passe dans le tunnel.
- Le cœur a l'adresse `10.77.0.1` dans le tunnel ; chaque téléphone la sienne, de `10.77.0.2` à `10.77.0.254`, la seule d'où ses paquets sont pris.
- **Un pair inconnu n'a aucune réponse** : une poignée de main d'une clé que le cœur ne connaît pas, ou un datagramme qui n'est pas WireGuard, est jeté sans un mot. Un scan du port ne voit rien. Sous une rafale de poignées de main, le cœur demande les cookies de WireGuard.
- **L'API n'est servie que dans le tunnel** pour un téléphone, à `ws://10.77.0.1/remote`. Sur le poste, quand l'accès est allumé, l'API n'a alors **aucun port sur la machine** : elle n'est jointe que par le tunnel, jamais par l'adresse publique ni le réseau local. Sur un serveur, elle garde en plus son port de 127.0.0.1 (le tunnel SSH du poste, voir *Le mode serveur*).
- Dans le tunnel aussi, **le jeton d'appareil est demandé** (défense en profondeur si la clé d'un téléphone fuit), le `Host` doit être l'adresse du tunnel, et seule l'`Origin` de la fenêtre d'Escouade ou de l'app mobile passe : `capacitor://localhost` (iOS), `https://app.escouade.invalid` (Android, un nom propre à l'app depuis ESC-631), et `https://localhost` ou `http://localhost` d'une app construite avant ce nom ; ces dernières restent refusées sur 127.0.0.1, où elles seraient une page du poste.
- **Un téléphone appairé à travers le tunnel** (le code de l'appairage donné par son pair) appartient à cet appareil : par son pair, seul le jeton de cet appareil est pris. **Révoquer l'appareil retire aussi son pair** : sa clé n'est plus reconnue. Les échecs d'un pair se comptent à lui seul : les essais ratés d'un autre pair, de Tailscale ou du réseau ne ferment jamais la porte à un pair lié à son appareil (voir *L'API distante*, « Les bornes »).
- **La clé du tunnel** peut être changée (Réglages › Téléphone › Avancé › « Changer la clé du tunnel », après confirmation) : chaque téléphone doit être appairé de nouveau, l'ancienne clé n'a plus de réponse.

## Ce qui est gardé

- `phone.json`, dans le dossier des données, lisible du seul propriétaire : l'accès allumé ou non, le port UDP, Tailscale ou non (éteint tant que l'utilisateur ne l'allume pas), STUN ou non (allumé par défaut), les téléphones (nom, clé publique, adresse dans le tunnel, l'appareil qu'ils sont), l'adresse publique vue et quand elle a changé.
- **La clé privée du tunnel** au trousseau du système ; sur un serveur, dans son fichier de secrets ; dans `phone.json` quand le trousseau la refuse. La clé privée d'un téléphone n'est jamais gardée par le cœur.

## Le port et la box

- Le port UDP est **51820** par défaut (celui de WireGuard), modifiable. Pris par un autre programme, l'accès ne s'allume pas et le dit : « Le port UDP 51820 est déjà pris sur cette machine (un autre WireGuard ?) : choisis-en un autre. »
- Allumé sur le poste, Escouade **essaie d'ouvrir le port dans la box**, par UPnP puis NAT-PMP, pour une heure renouvelée tant que l'accès est allumé, et le retire quand on l'éteint. Il dit s'il y est arrivé (« Port UDP 51820 ouvert dans la box par UPnP »), sinon **le geste à faire**, avec l'adresse de la machine : « Dans la box, redirige le port UDP 51820 vers 192.168.1.20 (cette machine) », et ce que la box a répondu.
- **L'adresse publique** est demandée à un serveur STUN (une requête de liaison, rien d'autre ne part : `stun.cloudflare.com`, puis `stun.l.google.com`), avec l'adresse que la box dit avoir sur Internet. Elle est **relue toutes les cinq minutes** ; quand elle change, Réglages › Téléphone le dit (« Changée le … (avant : …) : un téléphone appairé avant elle doit reprendre la nouvelle »).
- **STUN est dit et se coupe** (SR1-11, ESC-641) : Réglages › Téléphone a la ligne « Demander l'adresse publique à STUN », visible même l'accès éteint, qui dit qui est demandé (Cloudflare et Google, par le nom de leurs serveurs), quand (à l'allumage, puis toutes les 5 minutes tant que l'accès est allumé) et ce qu'ils voient (l'adresse IP et le port d'où part la demande, rien d'autre). Allumée par défaut ; éteinte, **plus rien ne part vers eux** : la box (UPnP, NAT-PMP) est encore demandée, l'adresse que la box dit avoir sur Internet sert d'adresse publique quand elle en est une (pas celle d'un CGNAT), et l'adresse donnée à la main (`escouade-server phone add --endpoint`) sert encore. La page de confidentialité du site le dit aussi (« L'accès du téléphone, si tu l'allumes »).
- **Un CGNAT est détecté et dit** : quand l'adresse de la box est partagée par l'opérateur (100.64.0.0/10) ou privée (une box derrière une autre), ou n'est pas l'adresse vue de dehors, une redirection n'y fait rien. L'écran le dit, avec les issues : une IPv4 dédiée demandée à l'opérateur, l'IPv6 de la box, Tailscale, un serveur Escouade sur un VPS.
- Une **IPv6 publique** de la machine est dite, avec le port à ouvrir dans le pare-feu IPv6 de la box.
- **Sur un serveur**, il n'y a pas de box : rien n'est demandé à une box, et l'écran dit d'ouvrir le port dans son pare-feu (« ufw allow 51820/udp »). Une machine qui a l'adresse publique elle-même est dite comme telle.

## Tailscale quand il est là

- **Reconnu par sa commande, ou par sa propre interface** (SR1-05, ESC-641) : quand la commande `tailscale` est là (installée ou sur le `PATH`), elle seule fait foi (`tailscale ip -4`, puis son nom par `status --json`) ; sans adresse de sa part, pas de Tailscale. Sans la commande, la route vers 100.100.100.100 ne suffit plus : l'adresse d'où elle part (dans 100.64.0.0/10) doit être celle de **l'interface de Tailscale** : `tailscale0` (Linux), l'adaptateur « Tailscale » (Windows), ou un `utun` (macOS) dont 100.100.100.100 répond au DNS comme MagicDNS. Une adresse de 100.64.0.0/10 sur une autre interface est le CGNAT d'un opérateur (une clé 4G, un partage de connexion, certains hébergeurs) : ce n'est pas Tailscale, et l'API n'y est jamais servie.
- **Éteint tant que l'utilisateur ne l'allume pas** : trouvé, Réglages › Téléphone le dit, avec l'adresse et le nom de la machine dans le tailnet, et l'interrupteur « Servir aussi sur l'adresse Tailscale » est éteint (« Éteint tant que tu ne l'allumes pas »). Allumé, l'API est **servie aussi sur l'adresse Tailscale**, au même numéro de port en TCP : un téléphone de ce tailnet la joint sans port à ouvrir, CGNAT compris. Un `phone.json` qui ne dit rien de Tailscale le prend éteint.
- Seule une machine du tailnet est prise (100.64.0.0/10, `fd7a:115c:a1e0::/48`), le jeton d'appareil reste demandé. **Par Tailscale, seule l'`Origin` de l'app du téléphone passe** (`capacitor://localhost`, `https://app.escouade.invalid`), ou celle de la fenêtre d'Escouade : plus aucun `http(s)://localhost`, qu'une page web servie sur une autre machine du tailnet (le serveur de développement d'un dépôt pas sûr) pourrait avoir (SR1-09, ESC-631). Chaque machine du tailnet compte ses échecs à son adresse. Escouade ne refait pas Tailscale : ni perçage de NAT, ni relais.

## Réglages › Téléphone

La page dit, pour le cœur où est la fenêtre (le poste, ou le serveur de son espace : la même page y agit sur le serveur) :

- **l'accès** (allumé ou éteint), **le port UDP**, et **STUN** (qui, quand, quoi, et son interrupteur) ;
- **l'état** : éteint, prêt sur le port, ne démarre pas (et pourquoi), l'adresse de l'API dans le tunnel, le nombre de téléphones ;
- **la box** : ouvert par UPnP ou NAT-PMP, ou le geste à faire, un CGNAT et ses issues, l'IPv6, l'adresse publique et son changement, « Revoir le réseau » ; sur un serveur, son pare-feu ;
- **Tailscale** : là ou non, son adresse, l'interrupteur (éteint tant que l'utilisateur ne l'allume pas) ;
- **les téléphones** : leur nom, leur dernière poignée de main (« Tunnel ouvert · dernière poignée de main il y a 3 min » ou « Tunnel au repos »), leur adresse, « Retirer » ; sans téléphone : « L'app d'Escouade pour Android et iOS les appairera par un QR code » ;
- **Avancé** : la clé publique du tunnel, « Changer la clé du tunnel ».

## La stratégie du poste

`servers.phone` à `false` interdit le tunnel (voir *Réglages imposés*), comme `servers.remoteApi` à `false` : l'accès est refusé à l'allumage, un tunnel allumé **s'éteint aussitôt** (à la différence d'une connexion de l'API déjà ouverte, qui continue), le port est retiré de la box et l'API n'est plus servie sur l'adresse Tailscale. L'interrupteur de Réglages › Téléphone est grisé, avec son cadenas et qui l'impose. La stratégie retirée, l'accès revient s'il était allumé.

## Sur un serveur

`escouade-server phone` règle le tunnel du serveur depuis son terminal : `status`, `on [PORT]`, `off`, `remove ID`, et `add NOM [--endpoint HÔTE]`, qui ajoute un téléphone et affiche sa config WireGuard et son QR code (pour l'app WireGuard officielle, en attendant l'appairage de l'app mobile) ; cette config porte la clé privée du téléphone, montrée une fois et gardée nulle part. Le serveur qui tourne relit le fichier dans les deux secondes. Une page ouverte dans le tunnel qui n'est pas l'API est dite dans son journal.

## Le journal

Le tunnel dit son port, chaque poignée de main d'un téléphone, chaque téléphone ajouté ou retiré, un changement de clé ou d'adresse publique, l'ouverture du port dans la box et son refus ; jamais une clé privée, ni ce qui passe dans le tunnel.

## Pas encore

L'essai réel avec un téléphone en 4G (un geste de l'utilisateur) ; l'essai réel de l'app Android sous son nom propre par Tailscale (un geste de l'utilisateur) ; la reconnaissance de Tailscale sans sa commande sur un vrai Mac (`utun` et MagicDNS) et sur un poste derrière une clé 4G en CGNAT (des gestes de l'utilisateur) ; le QR code de l'appairage depuis Réglages › Téléphone et la sorte « téléphone » des appareils ; l'app elle-même ; un nom DynDNS à la place de l'adresse publique ; le pare-feu IPv6 de la box ouvert par Escouade ; empêcher la veille du poste tant que l'accès est allumé.

## Retirer un téléphone (ESC-630)

« Retirer » un téléphone, dans Réglages › Téléphone ou par `escouade-server phone remove ID`, retire son pair **et révoque l'appareil de l'API distante qui lui est lié** (appairé à travers lui), avec les appareils nés de lui : son jeton ne vaut plus nulle part, ni par Tailscale, ni par la boucle locale d'un serveur, ni par un autre pair. L'écran le dit : « Pixel retiré et son accès révoqué ». Un pair qui n'a jamais été lié à un appareil est retiré du tunnel, et l'écran dit qu'aucun jeton n'était lié à lui.

## Le QR code d'un téléphone (ESC-570)

- **Où** : Réglages › Téléphone › « Appairer un téléphone » (le poste, accès allumé), Réglages de l'espace › Appareils › « Appairer un téléphone » (un serveur, depuis un client de bureau, appareil de la portée poste), et `escouade-server pair --qr [--endpoint HÔTE]` (le QR en caractères dans le terminal du serveur, l'accès du téléphone allumé). Un téléphone ne peut pas en demander.
- **Ce qu'il porte** : la configuration WireGuard du téléphone (une clé faite pour lui, la clé publique du cœur, l'adresse où le joindre, son adresse dans le tunnel, l'API dans le tunnel), le code d'appairage, le nom de la machine, si c'est ce poste, la version d'Escouade et sa fin. **C'est un secret** : la fenêtre n'en reçoit que l'image (noir sur blanc, quel que soit le thème), le code à 8 caractères écrit à côté, le temps qu'il vaut encore (« Valable encore 9 min 42 s, pour un seul téléphone ») et l'adresse que le téléphone joindra (« à vérifier » quand aucune adresse publique n'a encore été vue) ; jamais son texte, jamais dans le journal, ni dans le presse-papiers. La page dit de ne pas le photographier.
- **Dix minutes, un téléphone** : son pair est ajouté aussitôt, **en attente** (« En attente de son QR code » dans la liste des téléphones) ; son code est de la portée « téléphone ». Un nouveau QR prend la place du précédent et retire son pair qui attendait ; un pair dont le QR a fini sans servir part de lui-même. Appairé par lui, le pair appartient à l'appareil né, n'attend plus, et le QR quitte l'écran (« Téléphone appairé : le QR code a servi »). Masqué, ou à la fin de ses dix minutes (« Le QR code a expiré : demandes-en un nouveau »), il n'est plus montré.
- **La révocation** du téléphone, depuis le poste ou le serveur (« Retirer » dans Réglages › Téléphone, « Retirer » dans Réglages de l'espace › Appareils, `escouade-server devices revoke`), retire son jeton, la clé de son appairage, son inscription au push et son pair WireGuard, aussitôt ; ses connexions se ferment dans les deux secondes.
- La liste des téléphones vide dit désormais « Aucun téléphone appairé : « Appairer un téléphone » plus haut montre son QR code. » ; `escouade-server phone add` reste pour l'app WireGuard officielle.

---
title: "L'API distante"
subject: "a-distance"
sources: ["docs/REMOTE.md, « La fondation commune » (points 2 à 6), « La fondation 2a », « La fondation 2b », « Le classement des commandes », « Le mode serveur, MS-1 », « Le mode serveur, MS-3 »","src-tauri/src/remoteapi (list.rs, server.rs, proto.rs, devices.rs, pairing.rs, outbox.rs, sessions.rs, window.rs)","src-tauri/src/termlog.rs (le tampon numéroté des terminaux), src-tauri/src/tui.rs (TuiOutput)","src/lib/transport.ts (le front sur deux transports), src/lib/connection.svelte.ts, src/components/ConnectionBand.svelte","crates/escouade-command (la liste unique des commandes)","crates/escouade-policy/src/list.rs (servers.remoteApi)","Revue de sécurité 2.3 (ESC-624), constats SR1-02, SR1-03, SR1-04, SR1-09, SR1-10 : src-tauri/src/commands.rs (remote_device_revoke, remote_pair_code, phone_remove), src-tauri/src/remoteapi_tests.rs, src-tauri/src/phone_tests.rs","apps/mobile/scripts/android-hostname.mjs (l'origine propre de l'app Android, ESC-631)","Rapport d'audit Mantis du serveur d'OshOEz (fork public https://github.com/OshOEz/escouade, branche feat/serveur, docs/security/audit-mantis-2026-11-serveur/rapport-mantis.md), données d'un tiers : ses constats « Pairing cancelled by five wrong codes », « Revoking a device does not cut its in-flight or queued file transfers », « Proxy password … serialized to every paired device », « A call that never returns keeps its global call permit », « A paired device holds all 16 connection slots », « Agent-authored link opened without noreferrer » (ESC-654)","Étude ESC-655 (le fork d'OshOEz en mode serveur), § 5 : le garde de l'API distante et les en-têtes doublés (ESC-688)"]
tickets: ["ESC-478","ESC-495","ESC-514","ESC-515","ESC-537","ESC-567","ESC-630","ESC-654","ESC-631","ESC-688"]
decisions: [189,193,202,216,285,289,328,339]
compared: null
---

# L'API distante

Le cœur d'Escouade (agents, Git, Kanban, terminaux) peut être piloté par un client qui tourne ailleurs : le poste qui vise un serveur, puis le téléphone. Il le fait par l'**API distante** : les mêmes commandes que la fenêtre, servies sur une WebSocket. C'est la fondation commune du mode serveur (voir *Le mode serveur* : `escouade-server` la sert) et du téléphone (`docs/REMOTE.md`) ; rien ne la montre encore dans la fenêtre.

## Éteinte par défaut, jamais ouverte sur Internet

- L'API est **éteinte** tant qu'on ne l'allume pas. Elle s'allume dans `escouade-server` (voir *Le mode serveur*), et dans les essais : un build de développement de l'app par `ESCOUADE_REMOTE_API` (le port), le cœur sans fenêtre quand il le demande.
- Elle n'écoute que sur **127.0.0.1** : aucune autre interface, ni IPv6, sauf `escouade-server --listen`, demandé en toutes lettres et qui avertit (Docker, un tunnel, un proxy). On l'atteint par un tunnel (SSH, puis WireGuard : les tickets suivants). Pas de TLS ici : c'est le tunnel, ou le proxy TLS devant, qui chiffre.
- Avant même d'ouvrir la WebSocket, le cœur refuse (HTTP 403) une requête dont le `Host` n'est pas le bouclage (`127.0.0.1`, `localhost`, `[::1]`, avec n'importe quel port, celui d'un tunnel compris), ou qui porte l'`Origin` d'une page web : **une page web du poste ne peut pas s'y connecter**. Seule passe l'`Origin` de la fenêtre d'Escouade elle-même (`tauri://localhost`, `http://tauri.localhost`), pour que la fenêtre puisse passer par l'API. Un serveur derrière un proxy TLS accepte en plus le nom de domaine qu'on lui donne en toutes lettres (`escouade-server --allow-host`, voir *Le mode serveur*) ; tout autre nom reste refusé.
- Un message de plus de 16 Mo ferme la connexion ; ce qui est plus gros (un fichier écrit ou lu) passe en parties.
- La stratégie du poste **`servers.remoteApi`** à `false` interdit l'API : elle ne démarre pas, et chaque nouvelle connexion est refusée (« L'API distante est interdite par la stratégie de ce poste (servers.remoteApi). »), un appairage compris.

## Le jeton d'appareil

- Chaque appareil qui peut se connecter a **son jeton**, révocable un par un. Révoquer un appareil ferme aussitôt ses connexions, les inactives comprises, **lâche ses appels en cours et en attente** (voir « Les bornes ») et oublie ses sessions ; son jeton est ensuite refusé. Révoqué par `escouade-server devices revoke` pendant que le serveur tourne, il est déconnecté en quelques secondes.
- Les jetons sont gardés comme les autres secrets d'Escouade : au trousseau du système, ou, s'il le refuse (Linux), dans le fichier des appareils, lisible du seul propriétaire ; sur un serveur, dans son fichier de secrets (voir *Le mode serveur*). La liste des appareils (nom, date de création, dernière connexion) ne contient pas de jeton quand le trousseau ou le fichier de secrets les garde.
- Un jeton présenté est comparé **en temps constant** à tous ceux des appareils, à chaque connexion.
- Un nouvel appareil reçoit son jeton par **l'appairage** : un code d'`escouade-server pair`, à usage unique et de courte durée, donné à l'API à la place de la poignée de main (voir *Le mode serveur*). Le téléphone aura le sien.

## Ce que vaut un jeton

Un client connecté peut **tout ce que peut la fenêtre** sur ce cœur : créer des projets, lancer des agents et leur parler, répondre à leurs autorisations, ouvrir des terminaux et y taper, lancer des commandes de test, lire et écrire les fichiers des dépôts. Un jeton vaut donc l'accès à la machine du cœur avec les droits de son compte, et se garde comme une clé SSH. C'est pourquoi l'API reste derrière un tunnel, et le jeton n'est qu'une défense de plus si le tunnel fuit.

Ne sont refusées à distance que les commandes du poste, et celles qui changent de machine sans marcher encore telles quelles (ci-dessous). Un client ne reprend jamais le terminal d'un autre appareil : les sessions (ci-dessous) sont à leur appareil.

## La connexion

1. Le client ouvre `ws://127.0.0.1:<port>/remote`.
2. **La poignée de main** : son premier message, dans les 10 secondes après l'ouverture de la connexion (l'ouverture de la WebSocket comprise), donne sa version, son jeton et, s'il revient après une coupure, sa session (`{"t":"hello","version":"2.3.0","token":"…","session":"…"}`).
   - Un jeton absent, inconnu ou révoqué : refusé (`token`).
   - Une **autre version** que celle du serveur : refusée (`version`), avec la version du serveur et le côté à mettre à jour, le plus ancien, pour que le client propose « Mettre à jour le serveur » (ou le client). Quand c'est le serveur, le refus dit aussi comment le mettre à la version du client (`serverUpdate`) : la commande à lancer sur lui (`escouade-server update --to <version du client>`, qui vérifie la signature et attend la fin des tours), ou, pour le serveur de l'image Docker, la nouvelle image à prendre (`docker pull …:<version>`). Le client le garde (son `Problem::Version`, et la fenêtre dans `connection.refusal.serverUpdate`) ; l'écran qui le propose reste à faire. L'API suit la version de l'app : il n'y a pas de compatibilité entre versions.
   - Trop de connexions de cet appareil, ou en tout : refusée (`too-many-connections`).
   - Acceptée : `{"t":"welcome","version","client","device","session","resumed","desk","secretsInFile"}` : la session (celle qu'il a redonnée quand elle est encore à lui, sinon une nouvelle), les commandes du poste (`desk`), que le client fait chez lui s'il a un poste, et si le cœur garde ses secrets dans un fichier plutôt qu'au trousseau (`secretsInFile` : un serveur), que le client dit là où un secret se donne.
   - **L'appairage**, à la place de la poignée de main, pour un nouvel appareil : `{"t":"pair","code":"8NUC-W6HS","name":"Portable du bureau"}`. Réponse : `{"t":"paired","device":{…},"token":"…"}`, ou un refus `pairing` avec sa raison (`none` : aucun code n'attend ; `expired` ; `wrong` : faux ; `wait` : trop d'essais faux dans la minute, le code n'a pas été lu) ; puis la connexion se ferme, et l'appareil revient avec son jeton. **Un code faux n'annule jamais le code qui attend** : il vaut jusqu'à son heure ou jusqu'au bon code (ESC-654).
3. **Les appels** : `{"t":"call","id":7,"cmd":"create_agent","args":{…}}`, avec les arguments de l'IPC, par les mêmes noms. La réponse reprend l'id : `{"t":"reply","id":7,"ok":…}`, ou `{"t":"reply","id":7,"error":…}` avec l'erreur **telle que l'IPC la donne** (le texte de la commande, ou « command X missing required key y »), et un `code` quand c'est l'API qui refuse (`desk-only`, `not-served-yet`, `unknown-command`, `unavailable`, `busy`, `too-large`).
   - **Un corps brut** (les octets d'un fichier écrit, les morceaux d'un fichier joint) passe avant son appel, en trames binaires de 16 Mo au plus ; l'appel dit sa longueur et ses en-têtes (`"body":n,"headers":{…}`). Un corps qui n'a pas la longueur dite est refusé (`too-large`).
   - **Une réponse en octets** (un fichier lu) arrive en binaire, en parties quand elle dépasse 16 Mo.
4. **Les canaux** : un canal se donne comme l'IPC le donne (`"__CHANNEL__:<id>"`, l'id choisi par le client, unique dans sa session). Ce qu'il reçoit revient sur la même connexion : en JSON (`{"t":"channel","channel":1,"data":…}` : les événements de l'interface, par `subscribe`), ou en **binaire** pour la sortie d'un terminal, **numérotée** : chaque trame dit la fin de ses octets dans toute la sortie du terminal.
5. **Ce que montrerait le bureau** sur un cœur sans fenêtre (un carillon, une notification, le compte de ce qui attend, un lien à ouvrir) est poussé au client (`{"t":"host","event":…}`), qui le rend chez lui.

Les appels d'une connexion passent dans l'ordre de leur commande, comme dans la fenêtre : les frappes d'un terminal gardent leur ordre.

## La fenêtre sur l'API

La fenêtre d'Escouade sait parler au cœur par l'IPC de Tauri ou par l'API distante, sans qu'aucun écran ne sache lequel : les mêmes commandes, les mêmes résultats, les mêmes erreurs, les mêmes canaux. Le transport se choisit au démarrage ; aujourd'hui, seul un build de développement met la fenêtre sur l'API, sans réglage visible : `ESCOUADE_REMOTE_API_TOKEN` donné avec `ESCOUADE_REMOTE_API` (l'API du cœur de l'app elle-même) ou avec `ESCOUADE_REMOTE_URL` (celle d'un autre cœur, un `escouade-server` atteint par une redirection locale, sur 127.0.0.1 seulement : les essais du mode serveur).

- Les erreurs que l'API nomme arrivent dans la langue de la fenêtre : « « voice_start » se fait sur ce poste, pas à distance. », « Le jeton de cet appareil est absent, inconnu ou révoqué. »…
- Les commandes du poste (la voix, les pages des lancements, l'updater, les liens, la fin de l'app) sont faites par la fenêtre elle-même, chez elle.
- Restent à la fenêtre, quel que soit le transport : ses dialogues (choisir un dossier), le navigateur du système pour un lien, sa barre de titre, son thème et son zoom, ses menus et son tray, le glisser-déposer natif.

## Après une coupure

- Chaque terminal, chaque lancement (une commande de lancement, une étape d'un lancement de test), chaque connexion de compte et chaque interface d'agent garde côté cœur **ses derniers octets, numérotés** : 1 Mo, 4 Mo pour l'interface d'un agent (celle qu'une vue rejoue déjà à son ouverture).
- Le client qui perd sa connexion la rouvre de lui-même, à des délais croissants (250 ms, puis 500 ms, 1 s, 2 s, 4 s, puis toutes les 5 s). Pendant ce temps, une bande le dit en haut de la fenêtre : « **Connexion au cœur perdue · nouvel essai…** ».
- À la reconnexion, il redonne sa session et **reprend chaque terminal là où il en était** : il dit la fin des derniers octets reçus, et reçoit la suite ; sans trou ni doublon. S'il est trop en retard (la suite n'est plus dans le tampon), il reçoit tout ce qui reste, et le terminal écrit ce qui a été perdu (« [12 Ko perdus pendant la coupure] ») ; l'interface d'un agent redessine alors son écran.
- **L'état est relu en entier** (les projets, les agents, les tickets, les réglages…) ; les choix de la fenêtre (le projet actif, la vue, l'agent choisi) sont gardés ; un terminal fini pendant la coupure est dit fini.
- **Un appel resté sans réponse à la coupure est rendu en erreur, jamais refait** : une commande n'est pas idempotente (« Connexion au cœur perdue avant la réponse : « merge_agent » n'a peut-être pas été faite. »). Un appel fait pendant la coupure échoue aussitôt. Côté cœur, l'appel de la connexion coupée est lâché (voir « Les bornes ») : une commande asynchrone s'arrête, une commande en attente derrière une autre ne part plus ; une commande bloquante déjà partie va au bout.
- Un **jeton refusé**, une **autre version** ou la stratégie du poste **arrêtent les essais**, et la bande dit pourquoi (« Connexion au cœur refusée : Le jeton de cet appareil est absent, inconnu ou révoqué. »).
- Une session est gardée 15 minutes après sa dernière connexion (8 par appareil, 64 en tout) ; au-delà, le client en reçoit une nouvelle et ne reprend que l'état.

## Les bornes

Un client lent, muet ou hostile ne fait pas grossir la mémoire du cœur, et ne ferme la porte à personne :

- **La file d'envoi** d'une connexion est bornée (2048 messages, 8 Mo). Au-delà, les événements de l'interface se fondent en une seule relecture de l'état (le dernier état gagne), et la sortie d'un terminal n'est plus mise en file : le client est prévenu dès qu'il y a de la place, et la reprend par son numéro. Les réponses ne sont jamais jetées ; mais quand plus de 96 Mo attendent, ou que rien n'est lu pendant 30 secondes alors que quelque chose attend, la connexion est fermée (code 4002, `slow-client`).
- **Les connexions** : 4 par appareil, 16 en tout, plus **4 places gardées pour la première connexion d'un appareil** : un appareil qui n'est pas connecté entre toujours, par exemple pour révoquer celui qui a pris les autres (ESC-654).
- **Les poignées de main en cours** : 8 au plus, chacune en 10 secondes en tout. Quand elles sont toutes prises, **la plus ancienne est fermée pour la nouvelle** : des connexions qui ne disent rien ne tiennent plus la porte, ni contre le propriétaire, ni contre un appairage (ESC-654).
- **Les poignées de main ratées se comptent par source** (ESC-654, ESC-631, SR1-04) : l'adresse de l'autre bout (127.0.0.1 pour ce qui vient de la machine ou d'un tunnel SSH ; l'adresse d'une machine du tailnet ou du réseau de `--listen`), le pair du tunnel du téléphone (chaque téléphone le sien), ou, **derrière un proxy déclaré** (`escouade-server --trust-proxy`), le client qu'il dit faire suivre : la dernière adresse de son `X-Forwarded-For`, celle que le proxy écrit (celles d'avant, le client a pu les écrire lui-même). D'une adresse qui n'est pas un proxy déclaré, cet en-tête n'est jamais lu : un client ne choisit pas où vont ses échecs. Au-delà de 20 dans la minute, les codes d'appairage de cette source sont refusés sans être lus (`wait`) jusqu'à la fin de la minute. Le code qui attend vaut toujours.
- **Le plafond de toutes les sources** se compte à part pour **cette machine** (la boucle locale : sa fenêtre, `escouade-server pair`, un client par SSH) et pour **ailleurs** (le tunnel, Tailscale, le réseau de `--listen`, les clients d'un proxy déclaré) : au-delà de 120 codes faux dans la minute venus d'ailleurs, tout code venu d'ailleurs est refusé sans être lu jusqu'à la fin de la minute, **jamais un code tapé sur cette machine** ; et la fenêtre le dit (« 120 codes d'appairage faux en une minute venus du réseau… Le code qui attend vaut toujours, et se donne depuis cette machine ou par SSH »), comme le journal. Un inconnu ne peut donc ni brûler le code de l'utilisateur, ni l'empêcher de s'appairer depuis le serveur, ni le faire à son insu. Un proxy posé sur la machine même (un Caddy devant 127.0.0.1) se déclare, sans quoi ses clients comptent comme cette machine.
- **Un bon jeton passe toujours** : il fait 256 bits, il n'y a rien à deviner ; les échecs des autres ne ferment jamais la porte au propriétaire, ni à un pair du tunnel lié à son appareil.
- **La force brute d'un code** reste hors de portée : 240 codes lus par minute au plus (120 de cette machine, 120 d'ailleurs), 2 400 pendant les 10 minutes d'un code, sur 32⁸ = 2⁴⁰ ≈ 1,1 × 10¹² codes, soit une chance sur 450 millions environ, quel que soit le nombre de sources.
- **Les appels** : 256 en attente de réponse par connexion (`busy` au-delà). **Un appel est lâché quand sa connexion se ferme**, et donc quand son appareil est révoqué (ESC-654) : une commande asynchrone est arrêtée, une commande qui attend son tour sur sa file ne part plus ; une commande bloquante déjà partie va au bout de ce qu'elle fait (un fil ne s'arrête pas du dehors), sans réponse à personne.
- **Ce que donne le client** est borné et vérifié : un numéro de reprise au-delà de la fin vaut la fin, 1024 terminaux par reprise, une session de 64 caractères, 32 en-têtes, un corps brut de 64 Mo au plus, un nom d'appareil de 64 caractères ; la version qu'il donne ne passe dans la commande de mise à jour du serveur que réduite à ses chiffres, lettres, points et tirets.

## Plusieurs clients à la fois

- La fenêtre est un client comme les autres : chaque client reçoit tous les événements du cœur.
- **L'état de l'interface** (projet actif, vue, agent choisi, disposition) est gardé par client. Celui de la fenêtre est enregistré comme avant ; celui d'un client distant vit le temps de sa connexion. Ce qu'un client choisit (l'agent qu'il vient de créer, le projet qu'il ajoute) est à lui ; ce qui disparaît (un agent supprimé, un projet fermé) est oublié par tous.
- **Une réponse donnée par un client se ferme chez les autres** : une autorisation, une question ou un plan répondus depuis le téléphone disparaissent de la fenêtre, comme une réponse venue de claude.ai.

## Ce qui est refusé à distance

Chaque commande du cœur a sa classe, dans une **liste unique** d'où l'IPC de la fenêtre et l'API sont faits tous deux (le détail, commande par commande, dans `docs/REMOTE.md`, « Le classement des commandes ») :

- **servies telles quelles** : presque toutes, l'écriture des octets d'un fichier comprise ;
- **du poste seulement**, refusées avec l'erreur `desk-only` : la fenêtre et ses pages de lancement, ses liens et sa fin, les dialogues (enregistrer une image), la voix (micro, haut-parleurs, modèles), le raccourci global de l'Overseer, l'updater du client ;
- **qui changent de machine** en mode serveur : servies quand elles marchent telles quelles (un chemin de la machine du cœur, une image collée envoyée en données, la phrase dite d'une conversation vocale, les fichiers joints envoyés en morceaux) ; sinon refusées avec `not-served-yet` jusqu'au mode serveur : ouvrir ou montrer un fichier sur la machine du cœur, lire son presse-papiers, l'origine d'un fichier déposé, les serveurs de langage de l'éditeur.

## Le journal

`app.log` (`server.log` et la sortie standard d'un serveur) dit chaque connexion et sa fin avec le **nom de l'appareil**, une session reprise, chaque appareil appairé, chaque refus et sa raison (jeton, version, `Origin`, `Host`, stratégie, code d'appairage refusé et sa source, trop de connexions, commande du poste demandée), une connexion fermée parce qu'elle ne lisait plus, les poignées de main ratées en nombre d'une source, les codes faux en nombre (de cette machine, ou d'ailleurs), les proxys déclarés, un appel lâché à la fin de sa connexion, et chaque appareil révoqué. Jamais un jeton, un code d'appairage, une session, un chemin du disque, les arguments d'un appel ni ce qu'écrit un terminal ; un texte venu du client n'y paraît que coupé à 80 caractères.

## Pas encore

Une adresse `wss://` d'un domaine dans le client (le tunnel SSH et « Se connecter à un serveur » : voir *Se connecter à un serveur*) ; le téléphone.

## Dans le tunnel du téléphone (ESC-567)

L'API est aussi servie **dans le tunnel WireGuard du téléphone** (voir *Le tunnel du téléphone*), et sur l'adresse Tailscale de la machine quand Tailscale est là ; c'est le même serveur, les mêmes commandes, les mêmes bornes (les connexions du tunnel comptent dans les mêmes nombres ; leurs échecs, à leur pair ou à leur adresse du tailnet, du côté « ailleurs »).

- **Sur le poste**, l'accès du téléphone allumé fait tourner l'API **sans aucun port sur la machine** : elle n'est jointe que par le tunnel. Un build de développement qui l'allume aussi sur 127.0.0.1 (`ESCOUADE_REMOTE_API`) la donne aux deux.
- **Le `Host`** : dans le tunnel, l'adresse du tunnel (`10.77.0.1`) ; sur Tailscale, l'adresse ou le nom de la machine dans le tailnet. Le bouclage n'y est pas pris (un rebond de DNS à travers le tunnel), ni l'adresse du tunnel sur 127.0.0.1.
- **L'`Origin`** (SR1-09, ESC-631) : celle de la fenêtre partout ; celle de l'app mobile par le tunnel et Tailscale seulement, jamais sur la boucle locale. L'app a une origine **qui n'est qu'à elle** : `capacitor://localhost` sous iOS (aucun navigateur ne fait ce schéma), `https://app.escouade.invalid` sous Android (un nom sous `.invalid`, réservé, qu'aucun site n'a jamais). **Par Tailscale, seules ces deux origines passent** : plus aucun `http(s)://localhost`, qu'une page servie sur une autre machine du tailnet peut avoir. Par le tunnel, dont chaque pair est un téléphone, `https://localhost` et `http://localhost` d'une app construite avant ce nom passent encore.
- **Un pair du tunnel lié à un appareil** (appairé à travers lui) ne fait passer que le jeton de cet appareil ; un autre jeton par ce pair est refusé (`token`) et compte comme une poignée de main ratée.
- Réglages › Téléphone appelle `phone_status`, `phone_set`, `phone_check`, `phone_rotate_key` et `phone_remove`, **servies** : la page d'un espace serveur agit sur le tunnel du serveur.

## Qui révoque, qui appaire (ESC-630)

- **L'auteur d'une révocation vient de la connexion** : la trace d'un appareil révoqué (ce que son client lit : « révoqué le …, depuis … ») nomme l'appareil de la connexion qui l'a demandée, ou « la fenêtre d'Escouade » quand c'est la fenêtre du cœur ; `escouade-server devices revoke` n'en nomme aucun. Jamais un nom donné par l'appelant : `remote_device_revoke` ne prend que l'id de l'appareil.
- **Chaque appareil garde d'où il est né** (`pairedBy`) : l'appareil qui a demandé son code d'appairage, ou rien pour un code de la fenêtre ou d'`escouade-server pair`. **Révoquer un appareil révoque aussi ceux qui sont nés de lui**, et les leurs, par la fenêtre, l'API ou `escouade-server devices revoke` : leurs connexions se ferment, chacun garde sa trace. Un code demandé par un appareil révoqué depuis n'appaire plus aucun appareil (refus `pairing`, comme un code qui n'attend plus).
- **Un code d'appairage ne se demande pas à distance** : `remote_pair_code` n'est donné qu'à la fenêtre du cœur ; un client de l'API qui le demande est refusé, avec sa raison (« lance escouade-server pair sur le serveur, ou crée-le dans la fenêtre d'Escouade de cette machine »). Un jeton qui fuit ne fait donc naître aucun appareil. Les appareils « poste » (les portées de MO-4) y auront droit.
- **Le désappairage par le tunnel** (`unpair`) fait le contrôle de la poignée de main : par le pair d'un téléphone lié à un appareil, seul le jeton de cet appareil est pris ; un autre jeton y est refusé (`token`) et ne révoque rien.
- **« Retirer » un téléphone** (`phone_remove`) retire son pair et **révoque l'appareil qui lui est lié** (et ceux nés de lui) : son jeton ne passe plus par aucune voie, ni Tailscale, ni la boucle locale, ni un autre pair. Un pair jamais lié à un appareil est retiré du tunnel, et la réponse le dit (`neverLinked`) : aucun jeton n'était lié à lui.

## Face à l'audit du serveur d'OshOEz (ESC-654)

Un utilisateur, OshOEz, a écrit son propre mode serveur sur un fork de la 2.2.7 et l'a fait auditer ; six de ses constats ont été vérifiés dans notre API distante (son code et son rapport sont des données d'un tiers, relues comme telles).

1. **L'appairage annulé de l'extérieur** : valait chez nous (cinq codes faux de n'importe qui finissaient le code). Corrigé : un code faux ne l'annule plus, les essais sont bornés par source et en tout (« Les bornes »).
2. **Les places de connexion prises** : un appareil ne peut pas prendre les 16 places (4 par appareil, et un appareil ne fait naître aucun appareil, ESC-630), et la révocation par `escouade-server devices revoke` comme le code d'`escouade-server pair` passent par les fichiers, sans place. Mais des connexions muettes tenaient les 8 places d'une poignée de main : corrigé (la plus ancienne fermée pour la nouvelle), et 4 places sont gardées à la première connexion d'un appareil.
3. **Un appel qui ne rend jamais la main** : ses places (les appels en attente, ses files) sont celles de sa connexion et partent avec elle, mais l'appel continuait et ceux qui attendaient derrière lui partaient ensuite, pour un client parti. Corrigé : lâchés à la fin de la connexion.
4. **La révocation et les transferts** : les transferts de fichiers sont des appels de la connexion (corps bruts, octets lus) et se ferment avec elle, les connexions inactives de l'appareil aussi ; ce qu'il avait en cours et en attente est désormais lâché (point 3).
5. **Le mot de passe du proxy** gardé hors trousseau (`proxyPasswordInFile`, SEC-10) : ce champ n'existe que sur la ligne 2.2 (ESC-595), et sa sortie de l'état envoyé au front y est corrigée (ESC-653). Un essai de l'API distante garantit qu'il n'arrive dans l'état d'aucun client distant, une fois la 2.2 reversée.
6. **Le Referer** : nos liens s'ouvrent dans le navigateur du système, qui n'envoie aucun Referer, ceux du Markdown portent `noreferrer`, et l'API ne sert aucune page. La page de la fenêtre dit en plus `<meta name="referrer" content="no-referrer">`, et les pages des extensions de l'éditeur `Referrer-Policy: no-referrer`.

## Un seul nom par requête (ESC-688)

Avant d'ouvrir la WebSocket, par toutes les voies (la boucle locale, `--listen` et un proxy déclaré, le tunnel du téléphone, Tailscale), le cœur refuse en **HTTP 400** une requête qui porte **deux fois l'en-tête `Host`**, **deux fois l'`Origin`**, ou une **URI absolue dont l'autorité n'est pas celle du `Host`** (à la casse près) : un proxy devant le serveur et le serveur lisent toujours le même nom, et ce que le garde vérifie (le `Host` du bouclage ou d'`--allow-host`, l'`Origin` de la fenêtre ou de l'app) est bien ce que la requête demande. Le journal dit le refus et sa raison. Rien d'autre ne change : une URI absolue de la même autorité que le `Host` passe, comme un chemin seul ; les `Host` et `Origin` acceptés et refusés plus haut le restent.

---
title: "L'app mobile"
subject: "a-distance"
sources: ["docs/MOBILE.md (ESC-549, MO-0 : la conception, les choix M1 à M16, le découpage MO-1 à MO-11)","design/mobile-2-3/README.md (les trois directions, la création en B, la page qui les compare)","docs/MOBILE-STUDY.md (ESC-445 : Capacitor recommandé)","docs/REMOTE.md (Le téléphone ; Ajouts du 2026-11-14 : l'app mobile ; Plusieurs serveurs pilotés par un seul Escouade ; Ce qui change de machine, MS-4)","docs/SERVER-UX.md (les espaces, direction B, et « Partout »)","docs/INBOX.md (I15 : la boîte en file, premier écran du téléphone)","Préférence de l'utilisateur du 2026-11-20, relayée par le chef : « Firebase obligatoire ? J'aurais bien tout hosté moi-même sur Coolify. »","Choix de l'utilisateur du 2026-11-20, relayés par le chef : la direction B ; créer depuis le téléphone (« Je pilote depuis le téléphone, comment je crée un nouveau projet ? »)"]
tickets: ["ESC-549","ESC-445"]
decisions: [227,233,234,235,236,228,229,230,231,232,237]
compared: null
---

> **Tranché, pas encore construit.** Cette page résume la conception d'ESC-549 (`docs/MOBILE.md`). L'utilisateur a fait tous les choix le 2026-11-20 : la direction (B, « Le fil ») et la création depuis le téléphone dans la conversation du chef d'escouade, puis les autres tels que recommandés (M2 à M15, une seule réponse). Rien n'est construit : les tickets MO-1 à MO-11 sont à écrire.

## Ce que c'est

Une app pour le téléphone (Android et iOS, en Capacitor, l'interface Svelte d'Escouade reprise) qui suit et pilote Escouade hors de chez soi. Elle est un **client de l'API distante** du cœur, comme une fenêtre connectée à un serveur : aucun agent, aucun harness, aucun secret ne vit sur le téléphone. Elle sert d'abord à **répondre vite** : voir ce qui attend dans tous ses espaces, lire une demande en entier et y répondre d'un pouce ; puis à suivre (une conversation, le Kanban) et à créer (un projet sur un serveur, un agent, un ticket dicté).

## La forme : B, « Le fil »

Un fil dense de tout ce qui attend, rangé comme la boîte du bureau (« Tes agents se sont arrêtés », « Aujourd'hui », « Plus tôt »), chaque ligne avec la plaque et le nom de son espace ; des puces d'espace en haut (« Partout », Ce poste, Travail, Perso) ; trois onglets en bas (À toi, Agents, Kanban) ; le « + » de l'en-tête pour créer. Une ligne se déplie sur place ; une demande réglée ouvre la suivante. Les maquettes sont dans `design/mobile-2-3/` (`index.html` compare les trois directions rendues ; A « La pile » et C « Les espaces » n'ont pas été retenues).

## Ce qu'elle fait

- **Les espaces** : la même liste que sur le bureau (ce poste, chaque serveur), appairés un par un, chacun son tunnel, son jeton et sa reprise. L'accueil est « Partout » ; une puce ramène à un espace. Les stratégies d'un espace de travail (`servers.aggregateView`, `servers.contentlessNotifications`) valent sur le téléphone.
- **Le premier appairage, par QR code** : Réglages › Téléphone sur le poste, Réglages de l'espace › Appareils d'un client connecté à un serveur, ou `escouade-server pair --qr`. Le QR (10 minutes, un téléphone, un secret jamais journalisé) porte la configuration WireGuard du téléphone, l'adresse de l'API dans le tunnel et un code d'appairage ; le téléphone y gagne son jeton d'appareil ; un code à 8 caractères en recours. Clé et jeton au Keystore (Android) ou au Trousseau (iOS). Pas d'appairage sans verrou d'écran.
- **Le tunnel** : WireGuard intégré au cœur et à l'app (`boringtun`, `smoltcp`), aucun VPN du système ; l'API n'est joignable que dans le tunnel, derrière le jeton. L'adresse Tailscale quand Escouade la détecte.
- **Ce qui attend** : les agents arrêtés (autorisation, question, plan) en tête, puis la boîte « À toi » dans ses groupes, chaque demande avec son espace, son projet et son agent ; l'onglet Agents pour ceux qui travaillent.
- **Répondre** : la commande d'une autorisation se déplie en entier (jamais coupée) ; « Autoriser » ne s'active qu'une fois la commande vue jusqu'au bout (au-delà de douze lignes à 360 px, la demande s'ouvre en plein écran) ; puis le verrou du système (empreinte, visage) pour toute réponse qui fait agir un agent ou la machine (autoriser, approuver un plan, oui à un feu vert, valider ou lancer un ticket, lancer un agent) ; refuser, répondre à une question, choisir une option, envoyer un message s'en passent. « Toujours autoriser » n'existe pas sur le téléphone. Une réponse donnée ailleurs ferme la demande aussitôt.
- **La boîte « À toi »** entière : les sortes, les vues, les planches et le sceau d'un choix, la visionneuse, le PDF, les fichiers téléchargés, « Plus tard », « Un mot », revenir sur sa réponse.
- **Envoyer un message** à un agent, une photo jointe, la dictée du système (sur l'appareil ; l'audio ne part jamais au cœur).
- **Le Kanban** d'un projet : une colonne choisie, ses tickets ; une ligne glissée de côté pour valider (sous verrou) ou renvoyer avec un mot ; lancer, « L'agent résout ».
- **Créer** : **un projet sur un espace serveur**, depuis l'adresse d'un dépôt (cloné sur le serveur, avec ses clés Git) ou un dossier du serveur, comme le client de bureau (`clone_repo`, `fs_browse`), le clonage suivi et la notification quand il est prêt (Ce poste est grisé : un projet du poste se crée au poste) ; **un agent avec son premier message**, tapé ou dicté, sans mode Bypass ; **un ticket dicté**, son titre et ses critères tirés de la dictée et relus avant de créer, « Créer dans À faire » ou « Créer et lancer ».
- **Les notifications**, l'app fermée, chiffrées de bout en bout : un agent arrêté (une notification par agent, groupées par espace), le résumé de la boîte, un ticket à tester, une CI rouge, un projet cloné ; **jamais** le texte d'une commande, d'une demande ou d'un message, un chemin ni un nom de fichier ; aucune action sur la notification. La toucher rouvre la ligne dépliée, ou dit « Déjà réglée ».
- **Le push** : un relais **auto-hébergé par l'éditeur sur son Coolify** (`push.escouade.dev`, ses clés FCM et APNs en secrets de Coolify, jamais dans le dépôt), qui ne voit qu'un réveil scellé ; le téléphone déchiffre. Un utilisateur ne peut pas monter ce relais lui-même : seul le détenteur des clés du projet de l'app envoie à ses téléphones. De Firebase, **seulement FCM** (ni Analytics, ni Crashlytics, ni base, ni hébergement ; l'inscription automatique du SDK coupée jusqu'à l'accord de l'utilisateur) ; **APNs** pour iOS, sans SDK Firebase. En option, pour un Android sans Google : **UnifiedPush** par un ntfy auto-hébergé et l'app ntfy, où le cœur envoie lui-même.
- **La révocation** depuis le poste ou un serveur retire le pair WireGuard, le jeton et l'inscription au push ; le téléphone le dit à sa prochaine connexion et efface ce qu'il savait de l'espace.
- **Hors ligne** : un bandeau par espace (« Travail injoignable depuis 14:32 »), l'écran daté, les réponses désactivées, l'envoi suspendu et le brouillon gardé. Le téléphone ne garde que le dernier coup d'œil de chaque espace et les brouillons, jamais une conversation, une commande ni un fichier.
- **Les appareils** : l'iPhone et les téléphones Android ; pas de mise en page pour l'iPad en 2.3 (l'app d'iPhone agrandie), une tablette Android centrée. Les deux plateformes sont construites ensemble ; le test fermé de Google Play part le premier.

## Ce qu'elle ne fait pas

L'éditeur, les terminaux interactifs et la vue d'un harness, les réglages fins (agents, comptes, plugins, intégrations, stratégies, serveur MCP), un projet sur ce poste, un dépôt neuf, l'écriture d'un ticket en entier (ses fichiers, ses dépendances, une branche à reprendre), « Toujours autoriser » et le mode Bypass, les comptes des harness et les secrets, la voix d'Escouade, l'aperçu du serveur de test d'un agent (après la 2.3), une mise en page pour l'iPad.

## Les choix

Tous faits par l'utilisateur le 2026-11-20 (`docs/MOBILE.md` § 7) : la direction (M1), le verrou (M3), les notifications (M5), le relais (M7), le Kanban (M8), l'iPad (M10), l'ordre des plateformes (M11), le tunnel (M12), UnifiedPush (M15) et la création (M16) ont chacun leur décision ; la navigation (M2), le geste (M4), le regroupement des notifications (M6), la dictée (M9), ce qui reste hors ligne (M13) et l'aperçu du serveur de test (M14) en ont une commune.

## Les tickets

MO-1 la coque ; MO-2 le tunnel côté cœur ; MO-3 le Rust du téléphone ; MO-4 l'appairage et la révocation ; MO-5 ce qui attend et répondre ; MO-6 les notifications ; MO-7 le Kanban ; MO-8 la publication ; MO-9 UnifiedPush, en option ; MO-10 créer un projet et un agent ; MO-11 le ticket dicté (`docs/MOBILE.md` § 8). Avant MO-1, deux essais de l'utilisateur : la maquette de l'étude sur un vrai Android d'entrée de gamme, et le push par un vrai projet Firebase réduit à FCM et une vraie clé APNs. Les comptes des stores, les fiches et le test fermé de Google Play sont ses gestes.

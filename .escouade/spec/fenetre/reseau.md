---
title: "Réseau : proxy et certificats"
subject: "fenetre"
sources: ["docs/security/audit-2026-11/rapport.md (branche ticket/esc-418), fiches SEC-07 et SEC-28","website/content/docs/fr/settings-network.md","crates/escouade-plugins/src/tls.rs","src-tauri/src/tls_tests.rs","docs/security/audit-2026-11/rapport.md, SEC-18 (recommandations appliquées, accord a-7 du 2026-11-21)","src-tauri/src/offline.rs","src-tauri/src/sentfiles.rs","docs/TEAM-SERVER.md §6.3 (branche release/2.3)","docs/security/audit-2026-11/rapport.md, fiches SEC-10 et SEC-14 (ESC-595)","src-tauri/src/proxysecret.rs","crates/escouade-updates/src/net.rs"]
tickets: ["ESC-425","ESC-435","ESC-596","ESC-595"]
decisions: [154,214,"audit-sec-09-15-et-18-hotes-trop-larges","audit-sec-07-10-et-14-le-jeton-du-mcp-ho"]
compared: null
---

## Ce que c'est

Réglages › Avancé › Réseau : le proxy HTTP(S), ses exclusions, et à qui les connexions d'Escouade font confiance. Toutes les connexions de l'app (quotas, intégrations, store, mises à jour, dictée, images distantes, réseau des plugins en `fetch` et en WebSocket) suivent la même confiance (`src-tauri/src/tls.rs`).

## Comportement

### Le mot de passe du proxy (ESC-595, audit SEC-10)

- L'adresse s'écrit comme avant, identifiant et mot de passe compris (`http://utilisateur:motdepasse@proxy:3128`). Le mot de passe va au **trousseau du système** (entrée `proxy`, sous le service de l'app), jamais dans `settings.json` : le fichier garde l'adresse sans lui (`http://utilisateur@proxy:3128`).
- La fenêtre ne le reçoit pas non plus : au démarrage de l'app, le champ montre l'adresse sans mot de passe (dans la session où il a été tapé, il garde ce que l'utilisateur a écrit), et dit sous lui « Le mot de passe de l'adresse est gardé dans le trousseau du système, jamais dans les réglages : l'adresse s'affiche sans lui. Pour le changer, écris l'adresse avec le nouveau. » Rendue avec le même identifiant, l'adresse garde son mot de passe (un autre port, un autre hôte compris) ; un autre identifiant, ou plus d'identifiant, le retire du trousseau.
- En mémoire, l'adresse est entière : les connexions de l'app et les variables des agents (`HTTPS_PROXY`…) ont le mot de passe, comme avant.
- **La migration** : au démarrage, une adresse écrite avec son mot de passe par une version d'avant le donne au trousseau, et `settings.json` est réécrit sans lui, une fois.
- **Sans trousseau** (Linux sans service de secrets, WSL) : le mot de passe reste dans `settings.json`, à part de l'adresse (`proxyPasswordInFile`), lisible de son seul propriétaire. Un trousseau apparu depuis le reprend au démarrage suivant.

### « Ne jamais sortir sans le proxy » (ESC-595, audit SEC-14)

- Une rangée sous l'adresse du proxy, **allumée d'office dès qu'un proxy est configuré**, tant que l'utilisateur ne l'éteint pas (`proxyNeverDirect`, vide jusqu'à son geste). Sans proxy, elle est éteinte et grisée : « Sans proxy configuré, rien à imposer : toutes les connexions sont directes. »
- Allumée, un contrôle ou un téléchargement de mise à jour dont le proxy ne répond pas (connexion refusée, délai, nom inconnu, tunnel sans réponse) **échoue avec sa raison**, au lieu de retenter une fois en connexion directe : « connexion impossible à github.com par le proxy 127.0.0.1:9000 (…) ; rien n'est tenté sans le proxy : « Ne jamais sortir sans le proxy » est allumé (Réglages › Avancé › Réseau) ». Rien ne part en direct.
- Allumée, **un agent ne démarre pas** (son process, sa vue Claude Code, un process rattaché) quand le mot de passe du proxy, gardé au trousseau, n'y est plus au démarrage (`proxyPasswordKept` dans `settings.json` dit qu'il y a été) : il partirait avec une adresse que le proxy refuse. La conversation dit « Le mot de passe du proxy n'est plus dans le trousseau du système, et « Ne jamais sortir sans le proxy » est allumé : l'agent n'est pas lancé. Réécris l'adresse du proxy avec son mot de passe dans Réglages › Avancé › Réseau. » Réécrire l'adresse avec son mot de passe lève le refus. Une adresse qui n'a jamais eu de mot de passe (un identifiant seul) n'est pas refusée.
- Éteinte, le comportement d'avant : un proxy injoignable est contourné une fois par contrôle, la tentative directe vérifiant toujours le certificat. Un proxy qui **répond** par un refus n'est jamais contourné, allumée ou non.
- Les autres connexions de l'app (quotas, intégrations, store, réseau des plugins) n'ont pas de route directe de secours quand un proxy est configuré : seuls les hôtes des exclusions et les adresses du poste (la boucle locale, les routes d'un plugin vers le poste) vont en direct.
- C'est un réglage simple, de clé `proxyNeverDirect` : un poste géré pourra l'imposer avec la 2.3 (rang 6, liste des politiques d'ESC-532, clé `proxy.neverDirect`).

### La confiance

- Les autorités de Mozilla, celles du magasin du système, et celles du fichier « Autorités de certification supplémentaires » (un `.pem`, par exemple le certificat racine de Zscaler). Le fichier s'ajoute aux autres, il ne les remplace pas.
- « Choisir… » prend le fichier ; sous le champ, l'app dit combien de certificats elle y lit (« 1 certificat lu »), ou pourquoi elle ne le lit pas (« aucun certificat PEM dans … »). Un fichier illisible au moment d'une connexion est écrit au journal et laissé de côté : le reste tient.
- Les agents reçoivent le fichier au prochain démarrage de leur process : `NODE_EXTRA_CA_CERTS` (Claude Code, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode) et `CODEX_CA_CERTIFICATE` (Codex). Les programmes des plugins les reçoivent aussi. Mistral Vibe (Python) n'a pas de variable qui ajoute sans remplacer : rien ne lui est passé.

### Les exclusions du proxy (ESC-435, audit SEC-28)

- **Une seule lecture des exclusions** décide à la fois de la route (par le proxy ou directe) et de la vérification du certificat, pour les connexions de l'app comme pour le réseau des plugins : les deux ne peuvent plus diverger.
- Séparées par des virgules, des espaces ou des points-virgules, elles comprennent : `*` (tout) ; un nom, qui vaut aussi pour ce qui est dessous, écrit avec ou sans `.` ou `*.` en tête, avec ou sans port, avec ou sans point final (le port n'est pas comparé) ; une adresse IPv4 ou IPv6, entre crochets ou non, avec ou sans port ; une plage d'adresses (`10.0.0.0/8`, `fd00::/8`), qui vaut aussi pour une IPv4 écrite en `::ffff:…`.

### « Ignorer la vérification des certificats TLS »

- Gardée pour les installations qui l'ont cochée, éteinte par défaut. Elle ne vaut que pour une connexion qui passe vraiment par le proxy configuré : sans proxy, elle n'a aucun effet ; vers un hôte des exclusions (une plage d'adresses comprise), ou une route directe d'un plugin, la connexion est directe et le certificat est vérifié. Le contrôle des mises à jour qui retente sans le proxy (« Ne jamais sortir sans le proxy » éteint) vérifie le certificat.
- Les agents reçoivent `NODE_TLS_REJECT_UNAUTHORIZED=0` seulement quand un proxy est configuré.
- Tant qu'elle est cochée, un bandeau permanent, sous la barre de titre, le dit : « Certificats TLS non vérifiés par le proxy <hôte:port> : une interception, jetons compris, passerait inaperçue. » (sans proxy : « … est cochée, sans proxy : elle n'a pas d'effet. »). Il ne montre jamais l'utilisateur ni le mot de passe du proxy. Son bouton « Choisir le certificat racine » ouvre Réglages › Avancé › Réseau.
- Sa description propose le fichier à la place : « Avec Zscaler ou un proxy d'entreprise : choisis son certificat racine (un fichier .pem). »

## Limites

- Les terminaux intégrés ne reçoivent ni le fichier ni la case.
- Les agents reçoivent les exclusions telles qu'écrites (`NO_PROXY`) : leur lecture est celle de leur harness.

## Le mode hors ligne (ESC-596, audit SEC-18)

- **« Mode hors ligne »**, dans Réglages › Avancé › Réseau, éteint par défaut, coupe d'un geste les quatre flux qu'Escouade lance de lui-même : les mises à jour (de l'app, toutes les 5 minutes, et des plugins, au démarrage), le catalogue du store, la liste de révocation (et son attestation de fraîcheur), les statistiques d'usage (et les comptes d'installations du store). Pendant qu'il dure, aucune statistique n'est gardée pour partir plus tard.
- **C'est le cœur qui refuse**, quoi que demande la fenêtre. « Rechercher des mises à jour » le dit (« Mode hors ligne : Escouade ne contacte ni les mises à jour, ni le store, ni la liste de révocation, ni les statistiques. »), « Chercher maintenant » des plugins aussi (« Mode hors ligne : offres du jj/mm hh:mm »). Le catalogue et la liste gardés s'appliquent.
- **Ce qui ne change pas** : ce que l'utilisateur demande d'un geste (installer un plugin, afficher une image), le travail des agents, `git fetch`, la lecture des quotas. La description du réglage le dit.
- **La barre de statut** dit « Hors ligne » tant qu'il dure ; un clic ouvre le réglage.
- **Imposable par un poste géré** : la stratégie `network.offline` (un booléen) l'allume, quoi que disent les Réglages. Elle est lue là où la 2.3 lira les stratégies du poste : `HKLM\SOFTWARE\Policies\Escouade` puis `HKCU\…` (valeur `network.offline`, `REG_DWORD` 1) sous Windows, les préférences gérées de macOS (`/Library/Managed Preferences/[<utilisateur>/]dev.gagnaire.escouade.plist`), `/etc/escouade/policies/*.json` sous Linux (le dossier et les fichiers à root, écrits par root seul). Relue chaque minute. Imposé, le réglage est grisé, allumé, avec « Imposé par la stratégie de ce poste. », et l'infobulle de la barre de statut le dit.

## Les images distantes (ESC-596, audit SEC-18)

- Une image distante (« Afficher l'image ») n'est jamais téléchargée depuis la machine ni le réseau local : une adresse écrite (`127.0.0.1`, `[::1]`, `10.…`, `192.168.…`, `169.254.…`), un nom de la machine (`localhost`), ou un nom que cette machine résout vers une telle adresse, même derrière un proxy (qui résoudrait lui-même). Chaque redirection est jugée de même. Le message : « Une image de cette machine ou du réseau local ne s'affiche pas ».
- Le mot de passe du proxy reste dans les variables d'environnement des agents (`HTTPS_PROXY`) : leurs harness en ont besoin pour s'authentifier.
- Sur la ligne 2.2, « Ne jamais sortir sans le proxy » ne s'impose pas encore par un poste géré : la liste des politiques est celle de la 2.3.

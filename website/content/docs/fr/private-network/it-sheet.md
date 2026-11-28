# La fiche pour le service informatique

Pour le RSSI ou le service informatique à qui l’on demande d’installer escouade-connector : ce qu’il ouvre, ce qu’il laisse entrer, ce qu’il journalise, ce qu’il garde, comment l’arrêter, ce qu’il ne fait pas et ce qui reste à la charge de l’entreprise.

## En bref

Un développeur de l’entreprise fait travailler des agents de code (Claude Code, Codex…) sur un serveur Escouade qu’il héberge hors du réseau de l’entreprise. Ces agents ont besoin de quelques services internes : un git, un registre de paquets, une API de recette. `escouade-connector` est un petit programme que **le service informatique** installe sur une machine de son réseau : il ouvre une connexion sortante vers ce serveur, et ne laisse passer que les connexions vers les `hôte:port` que **le service informatique** écrit dans son fichier. Il est open source (GPL-3.0 ou ultérieure), écrit en Rust, sans télémétrie. Son installation est décrite dans [le connecteur](private-network/connector#installer-le-connecteur).

## Ce qu’il ouvre

- **Une sortie TCP vers un nom, sur un port** : l’entrée des connecteurs du serveur Escouade, en WebSocket sur TLS (`wss://escouade.exemple.fr/connector`, l’adresse que donne le développeur). Le 443 est conseillé.
- **À travers le proxy de l’entreprise, s’il le faut** : `proxy` dans `connector.toml`, ou `HTTPS_PROXY`, en `http://` et avec une authentification Basic seulement. Une inspection TLS est acceptée : son autorité va dans `authorities`. Ce que le proxy voit passer, c’est la demande de WebSocket, puis du Noise chiffré, qu’il ne peut ni lire ni imiter.
- **Aucun port entrant** dans le réseau de l’entreprise, et **aucun port qui écoute** sur la machine du connecteur : il n’a aucune socket à l’écoute dans son code.
- **Les hôtes de son plafond**, depuis sa machine, quand le serveur le lui demande.
- **Les releases d’Escouade** (`github.com`), seulement si on lance `update` ou règle `auto_update = true`.

## Ce qu’il laisse entrer

- **Des connexions TCP vers les seuls `hôte:port` de son plafond** (`allow` dans `connector.toml`), que le service informatique écrit lui-même : `gitlab.acme.internal:22`, `*.npm.acme.internal:443`, une adresse écrite telle quelle. Le serveur **ne peut pas l’élargir** : il reçoit le plafond pour l’afficher, rien de plus. Des plages d’adresses (`ranges`) peuvent borner en plus chaque adresse jointe.
- **Les noms sont résolus dans le réseau de l’entreprise**, par le DNS de la machine du connecteur, une seule fois ; chaque adresse résolue est vérifiée, puis jointe telle quelle. Jamais la boucle locale, le lien local, les métadonnées d’un cloud, la machine du connecteur elle-même, ni une adresse hors des plages : un DNS qui change sa réponse ne fait rien joindre d’autre.
- **Bornées** : 256 connexions à la fois, 32 ouvertures par seconde, une connexion muette fermée après 10 minutes, réglables dans `[limits]`.
- **Pour un seul serveur** : celui dont la clé a été épinglée à l’appairage, par un code à usage unique valable 10 minutes. Un autre serveur, ou un proxy qui se ferait passer pour lui, est refusé.

## Ce qu’il journalise

- **Chaque connexion, permise ou refusée**, une ligne JSON sur la sortie standard : journald sous Linux (`journalctl -u escouade-connector`), l’Observateur d’événements sous Windows (Journaux Windows › Application, source `escouade-connector`), les journaux du conteneur.
- **Ce qu’une ligne contient** : l’heure (UTC), l’hôte, le port, l’adresse résolue et jointe, les octets dans chaque sens, la durée, la raison d’un refus, et l’agent et le projet annoncés par le serveur, à titre indicatif : c’est le serveur qui les dit.
- **La vie du service** : démarrage, connexion au serveur (sa version, le plafond envoyé), coupure, relecture de `connector.toml`, mise à jour.
- **Jamais le contenu** de ce qui passe, ni le code d’appairage, ni une clé privée.

Le serveur tient son propre journal des connexions de ses agents, que le développeur voit dans l’espace.

## Ce qu’il garde

- **Sa clé privée** (`connector.key`) et **la clé publique du serveur** avec son adresse (`paired.toml`), dans son dossier d’état : `/var/lib/escouade-connector`, en mode 0700, sous Linux ; `%ProgramData%\escouade-connector` sous Windows, la clé scellée par DPAPI, le dossier ouvert à SYSTEM, aux administrateurs et à LocalService seuls ; un volume dans un conteneur.
- **Rien d’autre**, hormis, avec la mise à jour automatique, le fichier téléchargé le temps de le poser. Ni cache, ni données des connexions.

## Comment l’arrêter

- **Tout de suite** : arrêter le service (`systemctl stop escouade-connector`, `sc stop escouade-connector`, `docker compose stop`). Plus rien du réseau de l’entreprise n’est joignable par le serveur ; les connexions ouvertes sont fermées.
- **Pour de bon** : `escouade-connector unpair`, qui écrit des zéros sur la clé privée puis l’efface ; ou désinstaller (`install-connector.sh --uninstall --purge`, `escouade-connector uninstall --purge`). Le développeur peut aussi le retirer de son côté ; sa clé n’y vaut alors plus rien.
- **Restreindre sans arrêter** : retirer une ligne de `allow`. Le fichier est relu en 2 secondes, et une connexion ouverte que le nouveau plafond ne permet plus est fermée.

## Ce qu’il ne fait pas

- Ni UDP, ni ICMP, ni multicast : du TCP seulement.
- Aucun relais vers Internet : il ne joint que son serveur, le proxy de l’entreprise, les hôtes de son plafond et, sur demande, les releases d’Escouade.
- Aucun accès au réseau de l’entreprise pour qui que ce soit d’autre que le serveur appairé.
- Rien du réseau de l’entreprise vers le serveur : il ne porte que les connexions que le serveur demande, vers son plafond.
- Aucune télémétrie. Escouade, l’éditeur, ne voit rien : aucun relais à lui, le tunnel va de la machine du connecteur au serveur du développeur.
- Aucune mise à jour qu’il ferait seul, sauf `auto_update = true` ; signée dans tous les cas, jamais vers une version plus ancienne.

## Ce qui reste à la charge de l’entreprise

- **La sortie des données.** Un agent qui lit le GitLab peut envoyer ce qu’il lit sur Internet depuis le serveur, qui a Internet pour joindre ses modèles. Le connecteur borne l’entrée dans le réseau de l’entreprise, pas la sortie du serveur. Ne mettre au plafond que ce que l’entreprise accepterait de voir lu par un prestataire.
- **Les droits fins.** Le plafond ne connaît ni chemins ni méthodes : des identifiants à la mesure, une clé de déploiement en lecture seule, un jeton du registre en lecture, un compte de recette.
- **La machine du connecteur** : ses mises à jour, son segment réseau, la règle du proxy pour sa sortie. Les hôtes internes voient son adresse comme source.
- **La lecture du journal**, et l’alerte sur ce qui compte (un refus répété, un volume inhabituel).
- **La mise à jour du connecteur**, si elle n’est pas automatique.

## Les fichiers et leurs signatures

Chaque release publie l’archive Linux (x64, arm64, un programme statique), l’`.exe` Windows signé par Azure Artifact Signing, l’image `ghcr.io/guillaume-gagnaire/escouade/escouade-connector` (multi-architecture), et pour chacun sa SHA-256 et sa signature par la clé des mises à jour d’Escouade, avec une attestation de provenance (`gh attestation verify`). Le code est public : `crates/escouade-connector` dans le dépôt d’Escouade.

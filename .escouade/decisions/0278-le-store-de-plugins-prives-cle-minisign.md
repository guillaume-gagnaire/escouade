---
number: 278
id: "le-store-de-plugins-prives-cle-minisign"
title: "Le store de plugins privés : clé minisign à part scellée sur le poste, contrôles rejoués par qui signe, révocation de l'équipe"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-579"
pages: ["serveur-equipe/store-prive","plugins/vue-d-ensemble"]
tickets: ["ESC-579"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/store-prive","added":45,"removed":0},{"page":"plugins/vue-d-ensemble","added":2,"removed":0}]
accepted_at: "2026-11-22T01:15:03+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-14 (docs/TEAM-SERVER.md §7) : une équipe veut publier ses propres plugins sur son serveur, installables par ses seuls membres. Le serveur est aveugle (C10-A) : il ne peut ni contrôler ni signer. C13 recommandait A (une paire à part, hors ligne, avec une clé de secours). Il fallait dire où la clé vit, qui contrôle et qui signe, ce que l'app refuse en dur, comment une version se retire, et ce qu'un membre peut installer seul.

## Décision

- C13-A : la clé de l'équipe est une paire minisign à part (forme pré-hachée, lisible par minisign), et sa clé de secours, tirées sur le poste d'un administrateur. La clé de tous les jours reste scellée par un mot de passe (PBKDF2-HMAC-SHA256 600 000 tours, XChaCha20-Poly1305) dans le dossier de données, hors du dossier de l'équipe ; la clé de secours, scellée de même, va dans un dossier choisi par l'administrateur. Ses clés publiques et son espace de noms sont déclarés par une entrée d'administrateur d'un flux d'équipe (escouade/plugins), ce qui tient lieu d'entrée du journal des membres.
- N'importe quel membre propose : les contrôles du store public tournent sur son poste (dont la compilation deux fois dans un dossier neuf) ; un administrateur qui garde la clé relit l'archive sur le serveur, rejoue les mêmes contrôles et ne signe que si l'empreinte est la même. L'archive et le module sont des fichiers chiffrés par la clé d'équipe. `escouade-plugin team-check` joue les mêmes contrôles sans l'app.
- L'app refuse en dur, à la lecture du flux puis avant d'écrire : hors de l'espace, dev.escouade.* (ni un espace qui le contient), un espace qui contient un plugin installé d'une autre origine ; un plugin d'équipe ne remplace jamais un plugin du même id venu d'ailleurs, ni l'inverse (l'origine `team` d'un plugin installé).
- La liste de révocation de l'équipe a le format de celle du store et son commentaire, son serial ne recule jamais ; elle s'applique à chaque lecture de l'équipe aux seuls plugins de l'équipe ; la clé de secours peut révoquer la clé de tous les jours.
- La politique d'installation reprend les clés existantes (plugins.allowedOrigins avec team-store, plugins.allowed, plugins.denied) : ce qu'un membre installe seul est ce que ces bornes permettent, et il n'y a pas d'autre installation. Installer reste son geste, depuis la vue Plugins « Équipe », après la fenêtre des permissions. Aucun outil MCP ne publie ni ne signe.

## Conséquences

Nouvelles crates/modules : escouade-team (signing.rs, plugins.rs), escouade-plugin-cli (team.rs, la commande team-check), escouade-plugins (team.rs, Origin::Team), le cœur (teamplugins.rs). Dépendances ajoutées à escouade-team : blake2, pbkdf2, sha2 0.10, minisign-verify. Un administrateur qui oublie son mot de passe perd la clé de tous les jours et la clé de secours (même mot de passe) : l'équipe déclare alors une nouvelle clé. Une archive plus grosse que la borne des fichiers du serveur (25 Mo d'office) ne part pas. Le coupe-circuit de TS-15 lira la même liste. Pas de nouvelle clé de stratégie : « ce qu'un membre installe seul » n'ajoute pas d'approbation par un administrateur.

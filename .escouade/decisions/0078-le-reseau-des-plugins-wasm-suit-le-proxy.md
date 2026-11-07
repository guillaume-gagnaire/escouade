---
number: 78
id: "le-reseau-des-plugins-wasm-suit-le-proxy"
title: "Le réseau des plugins WASM suit le proxy et le TLS de Réglages › Réseau, s'ouvre à {account} et local:shared, et gagne flux, WebSocket et minuteurs"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-332"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-332"]
documents: []
supersedes: [32]
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":13,"removed":0},{"page":"plugins/programmes-js-ts-en-wasm","added":9,"removed":0}]
accepted_at: "2026-11-05T03:36:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 32 faisait passer `net/fetch` à côté du proxy pour juger toutes les adresses résolues, et limitait les plugins à https, 10 s et 1 Mio. Les décisions du 2026-11-04 mettent tous les plugins en WASM en 2.2 : un poste derrière un proxy d'entreprise perdrait tous ses plugins, une instance auto-hébergée serait injoignable, et la voix, OpenCode et les harness ont besoin de flux, de WebSocket et de minuteurs (docs/PLUGINS.md §8.4, ESC-330).

## Décision

- `net/fetch`, `net/stream` et `ws/open` suivent le proxy et le TLS de Réglages › Réseau, lus à chaque appel. Par un proxy, seul le nom est jugé, et le journal le dit ; sans proxy, ou pour un hôte qu'il exclut, le jugement des adresses résolues de la décision 32 reste entier. Le proxy du système n'est pas suivi. Un proxy réglé qui n'est pas une adresse arrête l'appel.
- `"network": ["{account}"]` ouvre l'hôte saisi par l'utilisateur pour un compte que le plugin sert, sur le port saisi, lui seul, en http et sur le réseau local.
- `"network": ["local:shared"]` ouvre http vers `127.0.0.1`, sur le port que le cœur a lu dans la sortie du process partagé qu'il a lancé pour ce plugin (`Launch::shared.listens`), tant qu'il vit, jamais par le proxy.
- `limits.fetchSeconds` et `limits.fetchBytes` sont déclarés au manifeste, plafonnés par l'hôte à 120 s et 32 Mio ; `bodyBase64` va dans les deux sens.
- L'hôte sert `net/stream` (crédit de 256 Kio, 8 flux, 120 s sans octet hors `sse`), `ws/*` (`wss`, 4 sockets, 1 Mio par message, 256 Kio en file) et `timer/*` (10 ms au moins, 64), et le prélude donne `setTimeout` et `clearTimeout`.
- Le journal garde la méthode et l'hôte, jamais le chemin, la requête ni les en-têtes.
- Ne change pas : le reste de la décision 32 (`escouade-js` embarqué dans l'exécutable, la Javy épinglée, le cache des modules).
- Deux choix de ce ticket que l'étude ne tranchait pas : un `ws/send` long passe quand rien n'attend, sinon la file de 256 Kio interdirait les messages entre 256 Kio et 1 Mio ; un message reçu attend d'être remis au module avant le suivant, le WebSocket n'ayant pas de crédit.

## Conséquences

Un plugin isolé marche derrière un proxy d'entreprise, et joint les instances que l'utilisateur lui donne. Par un proxy, un DNS menteur n'est plus jugé par Escouade, mais par ce proxy. Le prélude change : les modules se refont par leur clé, et `goldens.json` est refait. Un lecteur de `net/fetch` doit regarder `body` et `bodyBase64`. Le délai de bout en bout de la voix reste à mesurer en P6.

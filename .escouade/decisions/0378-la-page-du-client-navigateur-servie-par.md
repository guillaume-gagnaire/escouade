---
number: 378
id: "la-page-du-client-navigateur-servie-par"
title: "La page du client navigateur servie par escouade-server : embarquée, ses en-têtes, ses origines"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-704"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-704"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/mode-serveur","added":21,"removed":9},{"page":"a-distance/api-distante","added":11,"removed":3}]
accepted_at: "2026-11-28T16:51:02+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les décisions 357 (le client navigateur, éteint par défaut, allumé par --browser) et 358 (derrière un proxy, en https seulement) disent quoi faire ; BC-1 (ESC-704) devait dire comment la page est construite, embarquée et servie, quels en-têtes elle porte et quelles origines l'API accepte, d'après l'étude ESC-655 (docs/BROWSER-CLIENT-STUDY.md, §§ 1, 3, 4, 5). Avant, l'API ne servait aucune page et refusait toute Origin web hors de la fenêtre et du téléphone.

## Décision

1) La build web (vite.web.config.ts, npm run build:web, dist-web/) est celle du bureau, son dossier public/ compris (What's up ?, les images de la visite, le logo, comme la fenêtre), chaque module @tauri-apps/* remplacé par un bouchon qui refuse d'une TauriUnavailableError ; un module ou un nom sans bouchon, ou un paquet qui garde __TAURI_INTERNALS__ ou ipc.localhost, fait échouer la build (les tests de Tauri du code, App.svelte, transport.ts et main.ts, tombent à la minification : la page est l'app, et What's up ? y attend app.ready comme dans la fenêtre). 2) La page est embarquée par le build.rs d'un crate à part (crates/escouade-web), tout l'arbre de dist-web/, compressé par gzip, quand ESCOUADE_WEB_DIST la nomme ; seul bin/escouade-server.rs nomme ce crate, donc l'app de bureau ne la porte pas ; la release la construit avant le binaire et server-package.sh refuse un escouade-server sans elle. Elle ajoute 12,7 Mo au binaire (mesuré ; 7,5 Mo sans public/). Sans elle, --browser refuse de démarrer. 3) --browser ou ESCOUADE_SERVER_BROWSER=1 la sert sur le port de l'API, par la socket du serveur seulement : GET/HEAD de / et du chemin exact de chaque autre fichier (ni décodage, ni .., ni segment caché) ; éteint, aucune requête n'est lue avant tungstenite. 4) Chaque réponse porte la CSP de la fenêtre (lue dans tauri.conf.json) sans les schémas de Tauri, avec un nonce de styles tiré par réponse et posé sur les <style> (hors scripts et commentaires), connect-src ws(s)://<Host>, l'empreinte des scripts en ligne d'un HTML, et nosniff, Referrer-Policy no-referrer, COOP et CORP same-origin, une Permissions-Policy qui ferme micro, caméra et géolocalisation ; assets/ immutable, le reste no-cache avec un ETag et 304 (sauf index.html et un HTML), refus no-store. 5) Allumé, l'API accepte Origin http://<Host> quand le Host est de la boucle locale et https://<nom>[:port] pour un nom de --allow-host, sur la boucle locale seulement ; http:// d'un nom, un autre port, un Host rebondi, X-Forwarded-Proto ne passent pas ; les requêtes de la page passent les mêmes contrôles que la WebSocket (Host et Origin uniques, autorité de l'URI, ESC-688), plus un Host de forme stricte. 6) La marque <meta name="escouade-served" content="escouade-server"> dans index.html dit à la page qu'elle est servie.

## Conséquences

Un serveur qui n'allume pas --browser ne change pas. Un serveur qui l'allume expose une page et une origine de plus, bornées par les mêmes contrôles que la WebSocket ; BC-9 les revoit avant la bêta. La page ne se connecte pas encore (BC-2 lit la marque, l'appairage, la clé de l'appareil) : What's up ? attend l'interface prête, il s'ouvrira donc avec BC-2. Un nouveau script de public/ chargé par index.html est servi sans rien changer au serveur. La release prend une étape de plus (npm ci, build:web, environ 2 minutes par architecture) ; les archives de debug de la CI disent ESCOUADE_NO_WEB_PAGE=1. Changer la page ne recompile pas le cœur (le crate à part).

---
number: 286
id: "le-lien-d-invitation-d-equipe-est-escoua"
title: "Le lien d'invitation d'équipe est escouade:// seul, et ne passe jamais par une page web"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-633"
pages: ["serveur-equipe/entree","serveur-equipe/serveur"]
tickets: ["ESC-633","ESC-634"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/entree","added":9,"removed":4},{"page":"serveur-equipe/serveur","added":3,"removed":3}]
accepted_at: "2026-11-22T15:18:47+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (SR2-08, haute) a montré que le lien `https://<serveur>/join/<id>#s=…&t=…`, ouvert dans un navigateur, chargeait une page servie par le serveur d'équipe, dont le script lisait ce qui suit le `#`. HTTP n'envoie pas le fragment, mais une page de l'origine du serveur le lit : un serveur piraté (la menace n°1 du §3.1 de docs/TEAM-SERVER.md) aurait changé sa page, tiré du secret le jeton d'inscription et le MAC de la demande, et serait entré à la place de l'invité, admis par l'administrateur. La spec (§3.3, point 2) disait à tort que rien après le `#` n'atteint le serveur.

## Décision

1. Le lien canonique est `escouade://team/join/<serveur>/<id>#s=…&t=…` (et `&m=…` pour un appareil d'un membre) : cliqué, scanné ou collé dans Escouade. Ouvert d'ailleurs, il remplit « Rejoindre une équipe » et montre le serveur et les six mots ; rien ne part avant « Rejoindre ».
2. Aucune forme `https://` ne porte le secret : ni sur le serveur d'équipe, ni sur une page statique d'escouade.dev, qui ferait d'un autre site (et de son hébergement) un tiers de confiance pour toutes les équipes, auto-hébergées comprises. Une messagerie qui ne rend pas `escouade://` cliquable : on copie et on colle le lien dans Escouade, ce que la fenêtre dit sous le lien.
3. Les liens de l'ancienne forme (`https://…/join/…`) sont refusés à la saisie et à l'ouverture, avec « demande un nouveau lien » ; l'appareil qui a fait une invitation avant n'admet plus personne avec elle (listée expirée), MAC valide ou non, car son lien a pu passer par la page du serveur.
4. La page `/join/<id>` du serveur reste, réduite à un texte fixe sans aucun script (CSP `script-src 'none'`) qui dit d'en demander un neuf.

## Conséquences

Le secret d'une invitation n'est plus jamais exposé à une page du serveur. Les invitations créées avant la 2.3 (seulement en essai : le serveur d'équipe n'est pas encore publié) sont à refaire. Un lien n'est plus cliquable dans les messageries qui ne connaissent que http(s) : il se colle. Le format des vecteurs de test (`team-crypto-v1.json`, l'url d'invitation) change pour l'app mobile. La vérification hors bande à l'admission (un code de confirmation, ESC-634) suit dans un autre ticket et protège aussi d'un lien qui fuit ailleurs.

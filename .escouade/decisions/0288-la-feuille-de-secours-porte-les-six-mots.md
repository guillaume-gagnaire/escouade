---
number: 288
id: "la-feuille-de-secours-porte-les-six-mots"
title: "La feuille de secours porte les six mots de l'équipe, et la récupération ne suit qu'une enveloppe ancrée par un administrateur"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-632"
pages: ["serveur-equipe/entree","serveur-equipe/client"]
tickets: ["ESC-632","ESC-625"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/entree","added":12,"removed":7},{"page":"serveur-equipe/client","added":7,"removed":2}]
accepted_at: "2026-11-22T17:14:26+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-625, constat SR2-09, haute) : la récupération par les 24 mots suivait la dernière enveloppe du flux d'équipe, que tout appareil inscrit écrit ; HPKE en mode Base ne dit pas qui a scellé, et rien ne vérifiait que le groupe rejoint était celui de l'équipe. Un membre, ou le serveur, pouvait y faire entrer le propriétaire dans un groupe MLS à lui. Le dessin d'abord tranché (l'empreinte de l'équipe tapée depuis la feuille, l'enveloppe signée d'une feuille administrateur vérifiée sur la GroupInfo) ne suffisait pas seul : l'empreinte est faite de l'id du groupe et de la genèse, qu'un membre connaît et peut recopier dans un groupe où il se fait administrateur. Il fallait un secret que les membres n'ont pas. Le séquestre avait le même défaut, avant que son lecteur n'arrive. Choix fait par le chef d'escouade sur la délégation de l'utilisateur du 2026-11-22 (« la sécurité qui prime »), l'ancre retenue par l'overseer à la question d'ESC-632.

## Décision

1. La feuille de secours montre les 24 mots et, sous eux, les six mots de l'empreinte de l'équipe, à noter ensemble ; la case dit « J'ai noté les 24 mots et les six mots de l'équipe ». « Nouvelle clé de secours » montre une feuille neuve de même.
2. L'écran « Récupérer une équipe » demande l'adresse, le jeton de l'hébergeur, les 24 mots et les six mots, lus sur la feuille (jamais reçus du serveur ni dictés). Les six mots sont tapés plutôt que dérivés dans la clé : aussi simple pour l'utilisateur, ils restent lisibles et comparables à l'empreinte que montre chaque membre, et c'est l'ancre (ci-dessous) qui porte la garantie forte.
3. Une ancre, clé de signature tirée des 24 mots, n'est tenue que des appareils administrateurs (donnée scellée à la clé de feuille de chacun, dans le flux) ; sa moitié publique est au registre. Chaque enveloppe de secours porte l'empreinte et deux signatures, la feuille administrateur qui l'a faite et l'ancre. La récupération vérifie l'ancre, les six mots, puis sur la GroupInfo l'empreinte, la clé de secours et que la feuille signataire est administrateur, avant toute adhésion. Le commit d'un membre ne fait pas d'enveloppe : un appareil administrateur en ligne écrit celle de l'époque.
4. Le séquestre de même : son ancre tirée de sa graine vient avec « la moitié pour l'équipe » (`escouade-escrow-team:2:`), qui remplace la moitié publique (`escouade-escrow-public:1:`, refusée : refaire la paire) ; ses enveloppes portent l'empreinte, le registre et les deux signatures.
5. Une feuille sans les six mots (ou une clé d'avant l'ancre) ne ramène plus l'équipe : l'écran le dit (« Ma feuille n'a pas ces six mots ») et renvoie à « Nouvelle clé de secours » sur un appareil administrateur ; la carte d'une équipe dont la clé date d'avant le dit à ses administrateurs.

## Conséquences

Un membre, un ancien membre ou le serveur ne peuvent plus détourner une récupération ni glisser un faux enregistrement à l'organisation. Ce qui reste : un ancien administrateur garde l'ancre des mots de son temps ; retiré, il lui faut le serveur pour écrire dans le flux ; redevenu membre, il y écrit encore : chaque appareil retient qui a été administrateur sous la clé en vigueur, la carte dit alors aux administrateurs de refaire la clé de secours, et la récupération essaie les enveloppes ancrées de la plus récente à la plus ancienne (une qui échoue ne cache pas une bonne). Après le départ du dernier administrateur, le commit d'un membre laisse l'équipe sans enveloppe de sa dernière époque : plus de récupération possible. La clé de feuille Ed25519 sert aussi de clé X25519 pour recevoir l'ancre (comme age et libsodium) : point pour l'audit externe. Les équipes de la bêta créées avant doivent faire une nouvelle clé de secours (la carte le dit) et refaire une paire de séquestre. Aucun changement côté serveur.

---
number: null
id: "pages-des-agents-p7-p8-un-interrupteur-d"
title: "Pages des agents P7, P8 : un interrupteur de Réglages › Agents en 2.2, Fonctionnalités et la partie en 2.3 ; la page reste sur le poste"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-666"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-666","ESC-661"]
documents: []
supersedes: [321]
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":24,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La décision 321 (ESC-661, choix de l'utilisateur a-36) mettait l'interrupteur « Pages des agents » dans Réglages › Fonctionnalités et les outils dans la partie « Lancement et navigateurs ». Le 2026-11-26, l'utilisateur a voulu que les pages sortent en 2.2, de `main`. Or `main` n'a ni Fonctionnalités, ni les parties d'un projet, ni l'API distante, ni le téléphone, ni le serveur d'équipe. Il faut une place en 2.2, qui devienne celle de la 2.3 sans rien perdre.

## Décision

**P7, en 2.2** : un interrupteur « Pages des agents », allumé par défaut, dans Réglages › Agents. Il est placé sous « Les agents peuvent piloter Escouade », dont il dépend, à côté d'« Agents et navigateurs ». Coupé :
- `show_page`, `get_page` et `close_page` quittent la liste des outils ;
- `drive_browser` refuse les lignes `page:` ;
- la consigne se tait.

Rien n'est effacé.

**P7, en 2.3** : la même clé devient l'interrupteur de Réglages › Fonctionnalités, et la partie « Lancement et navigateurs » porte les outils (`parts::of_tool`), comme choisi par l'utilisateur. Écartée : une partie à elle.

**P8** : la page reste sur le poste.
- En 2.2, rien n'est à distance.
- En 2.3, les commandes `page_*` sont du poste seulement, et la demande d'une page va partout, avec « Une page l'accompagne, à voir sur ton poste ».
- L'aperçu en image (AP-10) et la page servie à une fenêtre connectée à un serveur (AP-11) viennent après la 2.3.

Rien ne part au serveur d'équipe.

## Conséquences

- AP-2 pose l'interrupteur de Réglages › Agents.
- AB-R le passe dans Fonctionnalités, avec la partie, le `desk-only`, le champ `page` des demandes pour le téléphone et la carte d'une fenêtre sur un serveur.
- La 2.2 tient sans aucun travail sur l'API distante.

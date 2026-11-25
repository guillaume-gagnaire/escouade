---
number: 314
id: "une-commande-d-hebergeur-retire-l-entree"
title: "Une commande d'hébergeur retire l'entrée qui gèle une équipe ou un projet, et les appareils lisent le trou"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-636"
pages: ["serveur-equipe/serveur"]
tickets: ["ESC-636","ESC-635","ESC-625"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":10,"removed":0}]
accepted_at: "2026-11-25T20:33:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

SR2-02 (revue de sécurité de la 2.3, ESC-625) : le serveur garde le premier commit d'une époque sans pouvoir le lire et refuse tout autre commit de cette époque. Un commit illisible d'un membre (ou d'un agent détourné sur son poste) gèle donc toute l'équipe, ou un projet : plus aucun appareil ne synchronise, l'administrateur ne peut ni retirer le membre ni changer les clés. La correction de fond (un commit invalide dépassé par les appareils) est ESC-635 ; il fallait d'ici là, et pour toute entrée qui bloquerait encore, que l'hébergeur dégèle sans chirurgie de la base. Et un appareil prenait jusqu'ici toute entrée disparue pour un retour en arrière fatal.

## Décision

`escouade-team-server drop-entry --team <id> (--stream team|<nom> | --project <id>) --seq <n>` retire une entrée, après confirmation (ou `--yes`), en SQLite et en PostgreSQL. Il refuse sans l'id de l'équipe ; un commit doit être le dernier de son flux (son époque redevient libre). Le flux garde sa numérotation. La base garde une ligne d'audit (qui, quand, quel flux, quel numéro, l'empreinte de l'entrée telle que servie), jamais son contenu ; le journal n'en dit qu'une ligne sans identifiant ; `--list` relit l'audit. Le serveur liste les entrées retirées aux appareils (`GET /v1/dropped`, celles d'un projet à ses seuls appareils, sans dire qui les a retirées). Un appareil dont la dernière entrée lue a disparu n'y voit pas un retour en arrière si elle est listée avec l'empreinte qu'il en a gardée, et lit la suite. Les administrateurs voient les entrées retirées sur la carte de l'équipe pendant 30 jours.

## Conséquences

Une équipe ou un projet gelé se dégèle par l'hébergeur en une commande, et l'administrateur reprend la main (retrait du membre, rotation). Le serveur peut désormais cacher une entrée sans que ce soit une alarme, mais jamais en silence : il doit la lister, avec l'empreinte exacte, et les administrateurs le voient. La commande ne retire bien que l'entrée qui bloque : un commit déjà lu, une opération valide d'un projet ou une entrée signée d'un administrateur, retirés, cassent ce qui suit (fourche, chaîne d'appareil, entrée perdue pour tous) ; la doc du serveur et le guide d'installation le disent.

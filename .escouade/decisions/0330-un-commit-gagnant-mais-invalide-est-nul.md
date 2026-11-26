---
number: 330
id: "un-commit-gagnant-mais-invalide-est-nul"
title: "Un commit gagnant mais invalide est nul et un administrateur l'annule par une entrée signée"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-635"
pages: ["serveur-equipe/serveur","serveur-equipe/client"]
tickets: ["ESC-635","ESC-625","ESC-636"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":4,"removed":2},{"page":"serveur-equipe/client","added":9,"removed":0}]
accepted_at: "2026-11-26T04:45:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

SR2-02 de la revue de sécurité de la 2.3 (ESC-625) : le serveur garde le premier commit d'une époque sans pouvoir le lire, puis refuse tout autre commit de cette époque. Un membre, ou un agent détourné sur son poste, postait un commit d'octets quelconques (ou un commit MLS valide contraire aux règles) à l'époque courante : plus aucun appareil ne synchronisait, l'administrateur ne pouvait ni retirer le membre ni changer les clés ; le même geste gelait le groupe d'un projet. La commande de l'hébergeur `drop-entry` (ESC-636) n'était qu'un recours manuel. Il fallait décider du sort d'un tel commit, et comment le dépasser sans fourche : une annulation signée par un administrateur, ou une époque comptée par les appareils.

## Décision

1. Un commit de l'époque courante que MLS ou les règles refusent est **nul** sur chaque appareil : gardé (numéro, empreinte, appareil qui l'a posté), l'époque MLS ne bouge pas, la lecture continue. Seul le refus du commit lui-même le rend nul, et seulement si le groupe n'a pas bougé.
2. Un appareil **administrateur** l'annule par une **entrée « annulation »** du flux (genre `void`), signée par sa feuille sur le groupe, l'époque, le numéro et l'empreinte du commit. Le serveur ne la lit pas ; il la prend d'un appareil administrateur seulement, pour l'époque du dernier commit, une fois. Le commit reste servi tel quel ; l'époque redevient libre, **réservée aux administrateurs jusqu'à leur prochain commit** (le changement qui attendait, sinon une rotation faite seule).
3. Les appareils vérifient l'annulation (feuille d'un administrateur de l'époque, ce groupe). Un appareil qui avait pris le commit annulé, ou qui voit l'époque d'après un commit qu'il a trouvé nul, le dit comme une fourche.
4. Défense en profondeur : les commits sont aux administrateurs, sauf le changement d'un membre sur ses propres appareils. Le commit signe sa portée dans ses données authentifiées ; le client la dit en clair au serveur (`own`), qui refuse le commit d'un membre qui ne la dit pas. Seule la portée signée compte pour les appareils. Dans le groupe d'un projet, un membre n'ajoute que ses propres appareils.
5. La même chose pour le groupe d'un projet. Les administrateurs voient « Commits nuls » sur la carte de l'équipe pendant 30 jours.

Pourquoi l'annulation plutôt qu'une époque comptée par les appareils : le serveur continue d'ordonner les commits (le premier gagne, `409 epoch_taken` aux autres), un appareil ne fusionne son commit qu'une fois le serveur d'accord, et deux commits vivants pour une époque n'existent jamais. Compter l'époque côté appareil laisserait le serveur prendre deux commits par époque : un « pris » ne voudrait plus dire « gagné », et chaque appareil devrait choisir, une porte vers la fourche.

## Conséquences

L'équipe et ses projets continuent après un commit invalide ; l'administrateur retire le membre et change les clés, et sait qui l'a posté. Un administrateur fraîchement nommé ne commite comme administrateur qu'une fois les rôles du serveur à jour (au passage suivant d'un administrateur). Un membre ne peut plus ajouter l'appareil d'un autre membre au groupe d'un projet : un administrateur le fait. Le schéma du serveur gagne une colonne par flux (le commit déclaré nul) et une par équipe et par projet (l'époque réservée), migrées en SQLite comme en PostgreSQL ; l'API un genre d'entrée (`void`) et un champ (`own`) ; un client d'avant n'est pas pris en charge (la 2.3 est en bêta). Reste hors d'atteinte : un commit qu'un membre rend valide pour une partie des appareils seulement (MLS ne l'empêche pas) devient une fourche dite, jamais suivie ; et un groupe de projet sans administrateur se dégèle par `drop-entry` de l'hébergeur, qui ne retire jamais une annulation ni le commit qu'elle a déclaré nul.

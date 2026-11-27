---
number: 356
id: "le-coupe-circuit-est-un-flux-scelle-de-l"
title: "Le coupe-circuit est un flux scellé de l'équipe, appliqué comme une restriction de rang 5 du verrou"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-546"
pages: ["serveur-equipe/coupe-circuit","fenetre/reglages-imposes"]
tickets: ["ESC-546"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/coupe-circuit","added":40,"removed":0},{"page":"fenetre/reglages-imposes","added":7,"removed":0}]
accepted_at: "2026-11-21T08:53:38+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-15 (docs/TEAM-SERVER.md §9) demande une entrée signée par un administrateur au journal de l'équipe, appliquée dans la seconde, levée par un administrateur, qui ne fait que restreindre et passe par le moteur des couches et le verrou de TS-11. Depuis C9-B, le journal des membres est le groupe MLS. Il fallait choisir où vit l'entrée, comment un rejeu est refusé, comment elle s'applique sur le poste, ce qu'il advient d'un plugin (qui est de l'app, pas d'un projet), et comment le hors-ligne est tenu.

## Décision

1. L'entrée va dans un flux de l'équipe à part (`escouade/breaker`), comme la liste des projets (TS-8) : un objet scellé par la clé d'équipe de son époque, signé par la feuille MLS de son auteur, jugé sur chaque poste (un administrateur de son époque, ou d'aujourd'hui pour une époque non vécue). Ce n'est pas une extension des commits MLS. Le serveur refuse l'entrée d'un membre (403) et le flux n'est restreint par personne.
2. Chaque entrée (pose ou levée) porte un `serial` plus haut que tous ceux lus avant. Une entrée qui n'est pas au-dessus est refusée. Deux administrateurs en même temps : l'app du second réécrit son entrée au-dessus.
3. Sur le poste, c'est une restriction de rang 5 du verrou, qu'un `locked_by` « breaker » nomme : des `denies` sur `harnesses.allowed`, `harnesses.allowedModels` et `plugins.denied`, et `halt` pour « tous les agents ». Rien n'est réécrit : à la levée, tout revient. Un plugin coupé passe à l'état « Désactivé » (comme un plugin retiré par la liste de révocation du store), l'interrupteur du membre intact. Il n'est désinstallé que si l'administrateur le dit.
4. Un plugin est de l'app : sa coupure vaut pour tout le poste. Un harness, un modèle et « tous les agents » ne valent que pour les projets de l'équipe (ou pour un seul d'entre eux), jamais pour un projet personnel.
5. Hors ligne, ce que le poste a lu tient. Une équipe dont la dernière lecture a échoué fait relire ses coupe-circuits avant tout démarrage d'un agent de ses projets (trois secondes au plus). La stratégie du poste `team.maxOfflineHours` refuse un nouvel agent d'équipe passé N heures sans contact.

## Conséquences

Le coupe-circuit réutilise ce que TS-8 et TS-13 ont posé (flux scellé, alarme du serveur menteur, réécriture par un administrateur restant) et les refus du verrou (chaque démarrage, le MCP, l'installation d'un plugin), sans second chemin. Les tests mesurent environ 110 ms pour l'application sur l'autre poste. Coupé, un plugin de harness coupe ce harness sur tout le poste, projets personnels compris. L'écran conseille donc de retirer le harness plutôt que son plugin. La liste des stratégies passe à 34 clés (ADMX, profils et page du site régénérés). Le serveur dit maintenant aux appareils de l'équipe lesquels ont une connexion en direct ouverte (`online` de `/v1/devices`), pour la confirmation « sur N postes en ligne ».

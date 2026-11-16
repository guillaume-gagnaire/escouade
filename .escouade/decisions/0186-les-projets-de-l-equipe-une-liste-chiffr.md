---
number: 186
id: "les-projets-de-l-equipe-une-liste-chiffr"
title: "Les projets de l'équipe : une liste chiffrée tenue par les administrateurs, des adresses https et ssh seulement, un clone déjà là rattaché par le geste du membre"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-480"
pages: ["serveur-equipe/projets-equipe","serveur-equipe/fichier-equipe"]
tickets: ["ESC-480"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/projets-equipe","added":36,"removed":0},{"page":"serveur-equipe/fichier-equipe","added":1,"removed":1}]
accepted_at: "2026-11-16T10:18:13+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-8 (docs/TEAM-SERVER.md §5) : la liste des projets d'une équipe, clonables d'un clic. Il fallait trancher où elle vit, qui l'écrit et qui le fait respecter, quelles adresses d'un dépôt entrent dans la liste (ce qui vient de l'équipe n'est pas de confiance aveugle), comment un clone déjà sur le poste se reconnaît, et s'il se rattache tout seul.

## Décision

- La liste vit dans un flux de projet à part du serveur (`escouade/project`), chaque entrée scellée sous la clé d'équipe et signée par l'appareil de son auteur, comme le fichier d'équipe (TS-13). Le serveur n'y lit ni nom, ni adresse, ni branche.
- Seuls les administrateurs l'écrivent, et c'est tenu trois fois : l'app refuse avant l'envoi, le serveur (qui connaît les rôles) refuse l'entrée d'un membre (403), chaque appareil refuse à la lecture une entrée qui n'est pas d'un administrateur. Les projets d'un administrateur parti sont réécrits par un administrateur encore là.
- Seules les adresses `https://` et ssh entrent ; `file://`, `http://`, `git://`, `ext::`, une option de git glissée, un chemin local et un identifiant dans l'adresse sont refusés, à l'écriture comme à la lecture ; le logo est une image PNG, JPEG ou WebP, jamais un SVG.
- Un clone se reconnaît à l'adresse d'un remote de ses dépôts : https et ssh, avec ou sans `.git`, port et utilisateur, hôte en toute casse, sont la même adresse ; sur GitHub, GitLab, Bitbucket, Azure DevOps et Codeberg, la casse du chemin non plus.
- Un clone déjà là ne se rattache que par le geste du membre (« Rattacher à l'équipe »), et garde son nom et son logo, ce que l'écran dit. La case de TS-13 reste pour les projets hors de la liste.
- Le clone passe par git et les comptes du membre, l'adresse relue dans la liste (jamais dans la fenêtre) et donnée après `--` ; un refus d'accès devient « Demande l'accès à acme/api sur GitHub ».

## Conséquences

Le serveur porte une règle de plus (une écriture de membre refusée sur un id de flux connu de lui) : il ne lit toujours rien. Un nouvel administrateur n'écrit la liste qu'une fois son rôle porté au serveur par la lecture d'un administrateur. Un dépôt auto-hébergé atteint en `http://` ou par un chemin réseau n'entre pas dans la liste. Le rattachement automatique au clone et l'id du projet synchronisé (TS-5) d'un projet de la liste viendront avec TS-7.

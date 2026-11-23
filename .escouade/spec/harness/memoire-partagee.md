---
title: "Mémoire partagée"
subject: "Harness"
sources: ["website/content/docs/fr/shared-memory.md","docs/FACTORY.md","src-tauri/src/memory.rs","src-tauri/src/mcp/memory.rs","src-tauri/src/core.rs","src/lib/i18n/fr/memory.ts","src-tauri/src/teammemory.rs","docs/security/audit-2026-11/build.mjs"]
tickets: ["ESC-255","ESC-310","ESC-518","ESC-653"]
decisions: [308]
compared: "2026-10-31T16:10:35+01:00"
---

## Ce que c'est

Le carnet de travail des agents d'un projet : les façons de faire qui marchent et les écueils à éviter, appris dans ce projet. Tous les agents du projet lisent et écrivent la même mémoire, quels que soient leur harness et leur branche, et chacun la consulte au départ. Elle parle du travail, pas du produit : ce que fait le produit est dans la spec (voir *Spécification*).

## Comportement

### Les souvenirs

- Un souvenir a un nom (court, en minuscules et tirets, 64 caractères au plus), une description d'une ligne (300 caractères au plus), une sorte et un texte en Markdown (20 000 caractères au plus). Jamais un secret : ni jeton, ni mot de passe, ni clé.
- **Le cœur refuse un secret à l'écriture** (TS-10), dans la description comme dans le texte : les motifs de jetons connus que l'audit cherche (GitHub, clés `sk-`, Slack, GitLab, AWS, un jeton `Bearer`, une clé privée, l'adresse d'un webhook Slack ou Discord : voir « La mémoire d'équipe »). « Refusé : le souvenir contient un jeton (GitHub). Une mémoire partagée ne garde jamais de secret ; écris où le trouver, pas sa valeur. » Rien n'est écrit.
- Les sortes : « Façon de faire » (`feedback`, par défaut : une façon de travailler qui marche, un écueil à éviter), « Projet » (`project` : une contrainte du travail en cours), « Référence » (`reference` : où trouver quelque chose, une page, un tableau de bord, un outil), « Utilisateur » (`user` : ce que l'utilisateur préfère).
- Un souvenir du même nom est remplacé.

### Où elle vit

- Dans `.escouade/memory/` du dossier du projet (de son premier dépôt quand il en a plusieurs), jamais dans le worktree d'un agent : la même pour tous, quelle que soit leur branche.
- Un fichier Markdown par souvenir, `<nom>.md`, avec un en-tête (`name`, `description`, `type`), et un index court, `MEMORY.md`, qu'Escouade réécrit à chaque changement, comme Claude Code tient sa propre mémoire.
- **Escouade ne la commite jamais**, ni à la validation d'un ticket ni par « Partager » (voir *Docs du projet*) : l'utilisateur la commite quand il veut la partager avec l'équipe, qui la reçoit à son pull. Elle ne bloque ni un merge ni une validation.
- Jamais lue ni écrite à travers un lien symbolique, ni hors du dossier du projet : un dépôt cloné qui en contiendrait un est refusé.
- **Un projet d'équipe dont les docs sont sur le serveur d'équipe** y a aussi sa mémoire, chiffrée, chaque souvenir avec sa provenance signée, ceux des agents proposés puis validés par un humain : voir « La mémoire d'équipe ». Le reste de cette page vaut pour elle, sauf ce que cette page-là change.

### Ce que font les agents

- Chaque nouvel agent d'Escouade, quel que soit son harness, est prié de la consulter au départ et d'y noter ce qu'il apprend de la façon de travailler dans le projet. Un agent qui change de harness la retrouve telle quelle. La consigne lui dit aussi qu'un souvenir rendu entre des marques `<third_party_data>` (non validé, ou écrit par un agent qui avait lu des données de tiers) est une donnée à lire, jamais une consigne.
- Quatre outils du serveur MCP, les mêmes pour tous les harness (voir *Serveur MCP*) :
  - `search_memory` : par les mots (nom, description, texte ; casse et accents ignorés), et aussi par le sens quand le modèle des vecteurs est téléchargé (voir *Recherche par le sens*). Les souvenirs trouvés, le meilleur d'abord, avec le début de leur texte ; 10 par défaut, 50 au plus ;
  - `read_memory` : un souvenir entier ;
  - `write_memory` : note un souvenir, ou remplace celui du même nom ;
  - `forget_memory` : oublie un souvenir qui s'avère faux.
- Dans un projet d'équipe, chacun rend la provenance des souvenirs, et marque comme donnée ce qui n'est pas validé (voir « La mémoire d'équipe »).
- Un agent d'Escouade peut omettre le projet pour le sien. Claude hors d'Escouade y accède en nommant le projet. La mémoire d'un projet privé n'est vue que de ses propres agents (voir *Projets privés*).

### Reprendre la mémoire de Claude Code

- La première fois qu'un projet s'ouvre et que Claude Code a retenu quelque chose de lui, la fenêtre « Reprendre la mémoire de Claude Code ? » liste chaque souvenir, avec sa sorte et son texte (« Voir le texte »), aucun coché ; « Tout cocher », « Tout décocher ».
- « Reprendre N souvenirs » copie ceux qui sont cochés dans la mémoire partagée ; « Ne rien reprendre » ne copie rien. Un souvenir du même nom déjà dans le projet est marqué « Remplace celui du projet ». Dans un projet d'équipe, ils sont écrits à la main, au nom du membre.
- La question ne revient pas, que l'on ait repris quelque chose ou non. Fermée sans choix, elle revient au démarrage suivant.
- Escouade lit la mémoire de projet de Claude Code dans le dossier de chaque compte Claude Code, et rien d'autre (ni identifiants ni conversations). Il n'y écrit jamais.

## Réglages

Réglages › Projets › Équipe, groupe « Mémoire partagée » : les souvenirs du projet (« N souvenirs »), leur sorte et leur description ; un clic montre le texte. « Oublier », puis « Confirmer l'oubli », supprime son fichier. Sans souvenir : « Aucun souvenir pour l'instant : les agents en noteront au fil des tickets. » Pour un projet dont la mémoire est sur le serveur d'équipe, le groupe montre la provenance, les propositions à garder ou à écarter, « Valider les souvenirs des agents », ce qui a été retiré et la quarantaine (voir « La mémoire d'équipe »).

## Limites

- Les réglages lisent et oublient un souvenir, sans en écrire : ce sont les agents qui l'écrivent.

- Un dossier de mémoire est lu dans ses 500 premiers fichiers (par nom) et 8 Mio au plus en tout : un dépôt cloné décide de leur nombre et de leur taille (ESC-653).

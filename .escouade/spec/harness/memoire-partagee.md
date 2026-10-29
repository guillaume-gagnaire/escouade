---
title: "Mémoire partagée"
subject: "Harness"
sources: ["website/content/docs/fr/shared-memory.md","docs/FACTORY.md","src-tauri/src/memory.rs","src-tauri/src/mcp/memory.rs","src-tauri/src/core.rs","src/lib/i18n/fr/memory.ts"]
tickets: ["ESC-255"]
decisions: []
compared: null
---

## Ce que c'est

Le carnet de travail des agents d'un projet : les façons de faire qui marchent et les écueils à éviter, appris dans ce projet. Tous les agents du projet lisent et écrivent la même mémoire, quels que soient leur harness et leur branche, et chacun la consulte au départ. Elle parle du travail, pas du produit : ce que fait le produit est dans la spec (voir *Spécification*).

## Comportement

### Les souvenirs

- Un souvenir a un nom (court, en minuscules et tirets, 64 caractères au plus), une description d'une ligne (300 caractères au plus), une sorte et un texte en Markdown (20 000 caractères au plus). Jamais un secret : ni jeton, ni mot de passe, ni clé.
- Les sortes : « Façon de faire » (`feedback`, par défaut : une façon de travailler qui marche, un écueil à éviter), « Projet » (`project` : une contrainte du travail en cours), « Référence » (`reference` : où trouver quelque chose, une page, un tableau de bord, un outil), « Utilisateur » (`user` : ce que l'utilisateur préfère).
- Un souvenir du même nom est remplacé.

### Où elle vit

- Dans `.escouade/memory/` du dossier du projet (de son premier dépôt quand il en a plusieurs), jamais dans le worktree d'un agent : la même pour tous, quelle que soit leur branche.
- Un fichier Markdown par souvenir, `<nom>.md`, avec un en-tête (`name`, `description`, `type`), et un index court, `MEMORY.md`, qu'Escouade réécrit à chaque changement, comme Claude Code tient sa propre mémoire.
- **Escouade ne la commite jamais**, ni à la validation d'un ticket ni par « Partager » (voir *Docs du projet*) : l'utilisateur la commite quand il veut la partager avec l'équipe, qui la reçoit à son pull. Elle ne bloque ni un merge ni une validation.
- Jamais lue ni écrite à travers un lien symbolique, ni hors du dossier du projet : un dépôt cloné qui en contiendrait un est refusé.

### Ce que font les agents

- Chaque nouvel agent d'Escouade, quel que soit son harness, est prié de la consulter au départ et d'y noter ce qu'il apprend de la façon de travailler dans le projet. Un agent qui change de harness la retrouve telle quelle.
- Quatre outils du serveur MCP, les mêmes pour tous les harness (voir *Serveur MCP*) :
  - `search_memory` : par les mots (nom, description, texte ; casse et accents ignorés), et aussi par le sens quand le modèle des vecteurs est téléchargé (voir *Index du code*). Les souvenirs trouvés, le meilleur d'abord, avec le début de leur texte ; 10 par défaut, 50 au plus ;
  - `read_memory` : un souvenir entier ;
  - `write_memory` : note un souvenir, ou remplace celui du même nom ;
  - `forget_memory` : oublie un souvenir qui s'avère faux.
- Un agent d'Escouade peut omettre le projet pour le sien. Claude hors d'Escouade y accède en nommant le projet. La mémoire d'un projet privé n'est vue que de ses propres agents (voir *Projets privés*).

### Reprendre la mémoire de Claude Code

- La première fois qu'un projet s'ouvre et que Claude Code a retenu quelque chose de lui, la fenêtre « Reprendre la mémoire de Claude Code ? » liste chaque souvenir, avec sa sorte et son texte (« Voir le texte »), aucun coché ; « Tout cocher », « Tout décocher ».
- « Reprendre N souvenirs » copie ceux qui sont cochés dans la mémoire partagée ; « Ne rien reprendre » ne copie rien. Un souvenir du même nom déjà dans le projet est marqué « Remplace celui du projet ».
- La question ne revient pas, que l'on ait repris quelque chose ou non. Fermée sans choix, elle revient au démarrage suivant.
- Escouade lit la mémoire de projet de Claude Code dans le dossier de chaque compte Claude Code, et rien d'autre (ni identifiants ni conversations). Il n'y écrit jamais.

## Réglages

Réglages › Projets, groupe « Mémoire partagée » : les souvenirs du projet (« N souvenirs »), leur sorte et leur description ; un clic montre le texte. « Oublier », puis « Confirmer l'oubli », supprime son fichier. Sans souvenir : « Aucun souvenir pour l'instant : les agents en noteront au fil des tickets. »

## Limites

- Les réglages lisent et oublient un souvenir, sans en écrire : ce sont les agents qui l'écrivent.

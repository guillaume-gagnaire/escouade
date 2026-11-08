# La mémoire partagée

Les agents d’un projet tiennent ensemble un carnet de travail : les façons de faire et les écueils qu’ils ont appris dans ce projet, retrouvés par chacun au départ, quel que soit son harness.

## Ce qu’elle contient

La mémoire parle du travail, pas du produit : « lancer les e2e après un build de debug », « ce test est instable sous charge, le relancer seul », « les ports d’un worktree ne s’écrivent jamais dans un fichier versionné ». Ce que fait le produit reste dans sa spécification.

Chaque souvenir a un nom, une description d’une ligne, une sorte et un texte :

| Sorte | Pour |
|---|---|
| Façon de faire (`feedback`) | une façon de travailler qui marche, un écueil à éviter ; la sorte par défaut |
| Projet (`project`) | une contrainte du travail en cours |
| Référence (`reference`) | où trouver quelque chose : une page, un tableau de bord, un outil |
| Utilisateur (`user`) | ce que tu préfères |

## Où elle vit

Dans le dossier `.escouade/memory/` du dossier du projet (dans son premier dépôt quand il en a plusieurs), et non dans le worktree de chaque agent : tous les agents du projet lisent et écrivent la même, quelle que soit leur branche.

- Un fichier Markdown par souvenir, `<nom>.md`, avec un en-tête (`name`, `description`, `type`), comme la mémoire de Claude Code.
- Un index court, `MEMORY.md`, qu’Escouade réécrit à chaque changement.
- **Escouade ne la commite jamais**, pas même en validant un ticket : tu la commites quand tu veux la partager avec l’équipe, qui la reçoit à son pull. Elle ne bloque ni un merge ni la validation d’un ticket.
- Escouade ne la lit ni ne l’écrit à travers un lien symbolique, ni hors du dossier du projet : un dépôt cloné qui en contiendrait un est refusé.

## Ce que font les agents

Chaque nouvel agent d’Escouade est prié de la consulter au départ, et d’y noter ce qu’il apprend sur la façon de travailler dans le projet. Il passe par quatre outils du [serveur MCP](mcp), les mêmes pour [tous les harness](harness) : un agent qui passe de Claude Code à Codex retrouve la mémoire telle quelle.

| Outil | Ce qu’il fait |
|---|---|
| `search_memory` | Cherche par les mots (nom, description, texte ; casse et accents ignorés), et aussi par le sens quand le modèle des vecteurs d’Escouade est téléchargé. Rend les souvenirs trouvés, le meilleur d’abord, avec le début de leur texte. |
| `read_memory` | Lit un souvenir entier. |
| `write_memory` | Note un souvenir, ou remplace celui du même nom. |
| `forget_memory` | Oublie un souvenir qui s’avère faux. |

Un agent d’Escouade peut omettre le projet pour le sien. La mémoire d’un [projet privé](mcp#les-projets-prives) n’est vue que de ses propres agents. Claude hors Escouade y accède aussi, en nommant le projet.

## Reprendre la mémoire de Claude Code

Claude Code tient sa propre mémoire de chaque projet. La première fois que tu ouvres un projet dont Claude Code a retenu quelque chose, Escouade propose de le reprendre dans la mémoire partagée :

- une petite fenêtre liste chaque souvenir, avec sa sorte et son texte, aucun coché au départ ;
- coche ceux que tous les agents doivent connaître, puis « Reprendre » ; « Ne rien reprendre » ne copie rien ;
- un souvenir du même nom déjà dans le projet est signalé « Remplace celui du projet » ;
- la question ne revient pas, que tu aies repris quelque chose ou non. Fermée sans choix, elle revient au prochain démarrage.

Escouade lit la mémoire de projet de Claude Code dans le dossier de chacun de tes comptes Claude Code, et rien d’autre : ni tes identifiants ni tes conversations. Il n’y écrit jamais.

## Dans les réglages

Réglages › Projets › Équipe, groupe « Mémoire partagée » : la liste des souvenirs du projet, leur sorte et leur description. Un clic sur un souvenir montre son texte. « Oublier », puis « Confirmer l’oubli », supprime son fichier. Les agents écrivent la mémoire eux-mêmes : les réglages servent à la lire et à la nettoyer. Voir [les réglages des projets](settings-projects).

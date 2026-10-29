---
title: "Tickets partagés"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-190","ESC-243"]
decisions: []
compared: null
---

## Ce que c'est

Un ticket peut se partager avec l'équipe. Escouade l'écrit dans un fichier YAML du dépôt, que l'utilisateur commite et pousse lui-même. Les autres le voient dans leur Kanban, savent qui l'a pris, et peuvent le lancer chez eux. Ce qui tient au poste (boucles, coût, agent) n'y va pas.

## Comportement

### Le fichier

- Partager un ticket écrit `.escouade/tickets/<id>.yml` dans le dossier du projet, jamais dans un worktree. Pour un projet de plusieurs dépôts dont le dossier n'en est pas un, c'est dans le premier dépôt.
- `<id>` : 8 caractères tirés au hasard (minuscules et chiffres, sans 0, o, 1 ni l), stable, jamais la clé.
- Les champs, dans cet ordre :
  - `version` (1), `id`, `key`, `title` ;
  - `status` : `todo`, `in-progress`, `review` ou `done` ;
  - `author`, `taken_by`, `updated_by` : `{ name, id }` ;
  - `created`, `updated` : ISO 8601 avec le fuseau ;
  - `after` : les ids des fichiers des tickets attendus ;
  - `description` : en bloc littéral quand elle a plusieurs lignes ;
  - `criteria` ;
  - `images` et `files` : `{ name, file }`, copiés dans `.escouade/tickets/<id>/` ; un fichier joint jusqu'à 5 Mo seulement ;
  - `external` : `{ source, key, url }` ;
  - `outcome`.
- Trois lignes de commentaire en tête, en anglais : la page de doc, « Escouade never commits it », le schéma pour l'éditeur. Le schéma, `schemas/escouade-ticket.schema.json`, est publié par le site.
- Restent sur le poste : boucles, coût, avancement, critères cochés, compte, modèle, agent, branche.
- **Escouade ne commite ni ne pousse jamais ces fichiers.** Ils ne comptent pas dans la vérification « dossier propre » du merge et de la validation.

### Écriture et relecture

- Chaque fichier est comparé à l'empreinte gardée de sa dernière lecture. Un fichier qui a changé hors d'Escouade est lu : titre, description, critères (les cochés le restent), « Après », personnes, et la colonne, sauf si un agent de ce poste tient le ticket.
- Un fichier inchangé est réécrit quand le ticket dit autre chose : un changement de colonne, d'agent, de texte. Jamais à une boucle : `updated` et `updated_by` seuls ne comptent pas.
- La relecture a lieu après chaque enregistrement de l'état de l'app, quand le dossier du projet change (pull, checkout, modification), et au démarrage, avant que la fenêtre lise les tickets. L'écriture est atomique.

### Le nom de chacun

- « Ton nom » (Réglages › Application › Équipe), et « Ton nom dans ce projet » (Réglages › Projets › Kanban partagé ; vide : celui de l'app). Jamais dans `.escouade.yml`.
- Chaque poste a un identifiant : 8 chiffres hexadécimaux de l'empreinte SHA-256 de l'e-mail git global (au hasard sans e-mail), fait une fois. C'est lui qui dit si un ticket est à soi.
- Au premier lancement, après la question des statistiques, la fenêtre « Ton nom pour l'équipe » s'ouvre, préremplie par `git config user.name`, avec « Plus tard » et « Enregistrer ».
- Sans nom donné, le nom est celui de git : celui du projet, sinon celui de l'app, sinon `git config user.name` lu dans le dossier du projet (l'identité propre d'un dépôt avant la globale). Il n'est jamais enregistré ni écrit dans `.escouade.yml`.
- Réglages › Application pré-remplit « Ton nom » avec le nom git global (« Repris de git »), sans rien enregistrer tant qu'il n'est pas changé ; vidé exprès, il ne l'est plus. Réglages › Projets le montre en exemple dans le champ.
- Un partage sans nom ni nom git rouvre la fenêtre avant de partager ; le cœur le refuse.

### Ce qu'on voit

- La carte d'un ticket partagé par soi porte une marque « équipe » après la clé (infobulle : son fichier).
- La carte du ticket d'un autre porte le nom de l'auteur, avec une pastille à son initiale (teinte tirée de son id).
  - Dans « À faire » : « Ajouté par Alice · le pilote auto ne le lance pas », et « Lancer », pilote auto allumé ou non.
  - Ailleurs : « Pris par », « À tester chez », « Fait par ». Sans « Valider » : seul celui qui l'a pris le valide.
- Menu d'un ticket à soi : « Partager avec l'équipe » / « Arrêter le partage ». Menu du ticket d'un autre : « Lancer », « Modifier », « Masquer chez moi » (bandeau « n tickets masqués », « Afficher »), « Supprimer pour l'équipe… ».
- La fenêtre du ticket a l'interrupteur « Partager avec l'équipe ». Un nouveau ticket suit « Partager chaque nouveau ticket » du projet.
- Panneau Git, « Tout le projet » : le groupe « Tickets partagés » en tête, chaque fichier par la clé et le titre de son ticket, et une ligne qui dit qu'Escouade ne les commite jamais.

### Le pilote auto

- Le pilote auto ne prend jamais le ticket d'un autre (seul « Lancer » le fait), ni un ticket dont le fichier ne se lit pas, ni un ticket masqué.
- Un ticket « En cours » ou « À tester » chez un autre (partagé, jamais lancé ici) ne prend aucune place et n'est jamais repris au démarrage.

### Les clés

Un ticket partagé garde la clé de son auteur. Si elle existe déjà ici, le ticket reçu devient `DEM-12@alice` ; la carte montre `DEM-12` et le nom d'Alice.

### Les cas limites

- Fichier en conflit git (marqueurs) : ni lu ni réécrit. La carte propose « Garder le mien » (réécrit depuis le poste) et « Prendre le sien » (le côté reçu).
- Fichier illisible : la carte « Ticket partagé illisible », avec la ligne en cause ; jamais lancée ni réécrite.
- Fichier d'une version plus récente : lu pour ce qui est connu, jamais réécrit.
- Fichier disparu : le ticket d'un autre quitte le Kanban, sauf s'il a été lancé ici (il reste) ; le sien est marqué, avec « Partager à nouveau ».
- Deux personnes sur le même ticket : si le fichier reçu dit qu'un autre l'a pris pendant que l'agent d'ici le tient, la carte le dit, l'agent continue, et le prochain statut réécrit le fichier.
- Import d'un ticket externe déjà partagé (même `source` et `key`) : pas de doublon, l'import pas lancé devient le partagé.
- Un projet privé se partage aussi.

### Par le serveur MCP

- `list_tickets` et `get_ticket` ajoutent `shared` : `by`, `byMe`, `takenBy`, `file`, `problem`.
- `create_ticket` et `update_ticket` prennent `shared` ; `update_ticket` avec `shared` seul vaut dans toutes les colonnes.
- `start_ticket` et `delete_ticket` refusent le ticket partagé d'un autre.

## Réglages

- « Ton nom » : Réglages › Application › Équipe.
- « Ton nom dans ce projet » : Réglages › Projets › Kanban partagé.
- « Partager chaque nouveau ticket » : réglage du projet.

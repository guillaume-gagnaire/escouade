# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Choisir un jeton

Deux façons :

- **La CLI `az`**, sur dev.azure.com seulement. Si `az` est installée et connectée (`az login`), laisse le champ du jeton vide : Escouade demande à `az` un jeton Entra ID et le redemande avant qu'il expire. Un Azure DevOps Server ne connaît pas ces jetons : il lui faut un jeton personnel.
- **Un jeton d'accès personnel (PAT)**, créé sur la page des jetons de ton organisation (`<organisation>/_usersSettings/tokens`, qu'ouvre « Créer un jeton »), avec ces droits :
  - Code : lecture et écriture ;
  - Build : lecture et exécution ;
  - Work Items : lecture et écriture, pour Azure Boards.

Azure ne dit pas les droits d'un jeton : Escouade ne peut pas les vérifier à la connexion. Un jeton sans Work Items se voit au premier refus d'Azure Boards, et le message dit où en créer un autre.

Le jeton agit en ton nom : ce qu'il ouvre ou publie l'est sous ton compte.

## 3. Connecter

1. Dans Escouade, ouvre Réglages › Services connectés › « Hébergeurs de code », puis « Connecter un hébergeur… ». Un dépôt dont le remote est sur une organisation sans compte propose aussi « Connecter » suivi de son hôte.
2. « Hébergeur » : Azure DevOps.
3. « Adresse » : celle de ton organisation, `dev.azure.com/<organisation>` (une ancienne adresse `<organisation>.visualstudio.com` est la même), ou pour Azure DevOps Server celle de sa collection, comme `https://serveur/tfs/DefaultCollection`. Seul https est accepté (http seulement vers cette machine).
4. « Jeton (vide : celui de az) » : colle ton jeton, ou laisse vide pour celui de `az`. Le formulaire rappelle les droits : `vso.code_write`, `vso.build_execute`, `vso.work_write`.
5. Clique sur « Connecter ». La ligne dit « Connecté · » suivi de ton compte et de l'organisation (« ada@acme.dev · acme »), et « Sert aussi à Azure Boards ».

Dans « Sources de tickets », Azure Boards n'a pas de compte à lui : sa ligne dit « Compte de l'hébergeur Azure DevOps : il se connecte et se déconnecte dans Hébergeurs de code. », rappelle « Son jeton doit avoir le droit Work Items (lecture et écriture). » et propose « Connecter Azure DevOps… ».

## 4. Lier un projet Azure Boards

Dans le groupe « Sources liées à `<projet>` », la liste « Projet Azure Boards » propose les projets de tes organisations, rangés par organisation ; celui du projet, quand son remote est sur l'organisation connectée, vient en premier. Choisis-le : Escouade charge les états de ses types d'éléments et pré-remplit la correspondance des statuts. Enregistre les réglages. Depuis le Kanban, « Importer » sans source liée propose directement « Lier … (Azure Boards) ».

Les PR, elles, n'ont rien à lier : chaque projet trouve son compte par son remote, sous toutes ses formes (`https://dev.azure.com/<organisation>/<projet>/_git/<dépôt>`, `<organisation>.visualstudio.com`, ssh `git@ssh.dev.azure.com:v3/…`, et ceux d'un Server).

## Ce qu'Escouade garde, et où

- Ton jeton personnel va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul, et la ligne le dit.
- Pour un compte qui passe par `az`, Escouade redemande son jeton à `az` quand il le faut.
- Le reste du compte (l'adresse de l'organisation, ton identité) est dans `~/.escouade/integrations.json`.

« Déconnecter » oublie le compte et son jeton. Une organisation sans compte n'empêche rien : la validation d'un ticket pousse sa branche et ouvre dans ton navigateur la page de création de la PR, remplie.

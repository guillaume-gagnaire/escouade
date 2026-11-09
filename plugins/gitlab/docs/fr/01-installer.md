# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Choisir un jeton

Deux façons :

- **La CLI `glab`.** Si `glab` est installée et connectée à l'instance (`glab auth login`), laisse le champ du jeton vide : Escouade demande son jeton à `glab` quand il le faut. Le compte s'affiche alors avec « Jeton de glab ».
- **Un jeton d'accès personnel**, créé dans les réglages de ton compte GitLab, avec le droit `api`. « Créer un jeton », dans le formulaire, ouvre la page des jetons de l'instance tapée dans « Adresse » (gitlab.com quand elle est vide), avec le nom « Escouade » et le droit `api` déjà choisis.

Le jeton agit en ton nom : ce qu'il ouvre ou publie l'est sous ton compte.

## 3. Connecter

1. Dans Escouade, ouvre Réglages › Services connectés › « Hébergeurs de code », puis « Connecter un hébergeur… ». Un dépôt dont le remote est sur une instance sans compte propose aussi « Connecter » suivi de son hôte.
2. « Hébergeur » : GitLab. Pour une adresse que l'app ne connaît pas, elle cherche seule le type d'hébergeur (« Recherche du type d'hébergeur… ») en demandant `/api/v4/version` à l'instance.
3. « Adresse » : vide pour gitlab.com ; sinon celle de ton instance, chemin compris si elle en a un (`https://acme.dev/gitlab`). Seul https est accepté (http seulement vers cette machine).
4. « Jeton (vide : celui de glab) » : colle ton jeton, ou laisse vide pour celui de `glab`. Le formulaire rappelle « Droits à donner au jeton : api ».
5. Clique sur « Connecter ». Escouade demande à GitLab qui tu es. La ligne dit « Connecté · @ton-login ».

Chaque compte GitLab sert aussi à GitLab Issues : sa ligne dit « Sert aussi à GitLab Issues ». Dans « Sources de tickets », GitLab Issues n'a pas de compte à lui : sa ligne dit « Compte de l'hébergeur GitLab : il se connecte et se déconnecte dans Hébergeurs de code. » et propose « Connecter GitLab… ».

## 4. Lier un projet GitLab pour ses issues

Dans le groupe « Sources liées à `<projet>` », la liste « Projet GitLab Issues » propose tes projets, rangés par instance, avec leurs groupes et sous-groupes dans le chemin (« acme/back/api »), les derniers actifs d'abord ; celui du projet, quand son remote est sur une instance connectée, vient en premier. Choisis-le, puis enregistre les réglages. Depuis le Kanban, « Importer » sans source liée propose directement « Lier acme/api (GitLab Issues) ».

Les MR, elles, n'ont rien à lier : chaque projet trouve son compte par son remote.

## Ce qu'Escouade garde, et où

- Ton jeton va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul, et la ligne le dit.
- Pour un compte qui passe par `glab`, Escouade redemande son jeton à `glab` quand il le faut.
- Le reste du compte (l'adresse de l'instance, ton login) est dans `~/.escouade/integrations.json`.

« Déconnecter » oublie le compte et son jeton. Une instance sans compte n'empêche rien : la validation d'un ticket pousse sa branche et ouvre dans ton navigateur la page de création de la MR, remplie.

# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer le jeton sur l'instance

1. Dans Escouade, ouvre Réglages › Services connectés › « Hébergeurs de code », puis « Connecter un hébergeur… ». Un dépôt dont le remote est sur une instance sans compte propose aussi « Connecter » suivi de son hôte.
2. « Hébergeur » : Gitea / Forgejo. Pour une adresse que l'app ne connaît pas, elle cherche seule le type d'hébergeur (« Recherche du type d'hébergeur… »).
3. « Adresse » : celle de l'instance (`https://codeberg.org`, `https://gitea.com`, `https://git.acme.dev`). Seul https est accepté (http seulement vers cette machine).
4. « Créer un jeton » ouvre `<instance>/user/settings/applications`. Crée un jeton d'accès avec les droits que le formulaire rappelle : `write:repository`, `write:issue`, `read:user`. Copie-le.

Le jeton agit en ton nom : ce qu'il ouvre ou publie l'est sous ton compte.

## 3. Connecter

Colle le jeton dans « Jeton » et clique sur « Connecter ». Escouade demande à l'instance qui tu es. La ligne dit « Connecté · @ton-login » et « Pas de tickets : PR et CI seulement ».

Gitea et Forgejo n'ont pas de CLI dont Escouade prendrait le jeton : le champ doit être rempli.

Les PR n'ont rien à lier : chaque projet trouve son compte par son remote.

## Ce qu'Escouade garde, et où

- Le jeton va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul, et la ligne le dit.
- L'adresse de l'instance et ton login sont dans `~/.escouade/integrations.json`.

« Déconnecter » oublie le compte et son jeton. Sans compte, rien n'est bloqué : la validation d'un ticket pousse sa branche et ouvre dans ton navigateur la page de création de la PR, remplie.

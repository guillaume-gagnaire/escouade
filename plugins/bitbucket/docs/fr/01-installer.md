# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer le jeton chez Atlassian

Bitbucket Cloud se connecte avec un jeton d'API Atlassian, à droits choisis, et l'e-mail du compte Atlassian auquel il appartient.

1. Dans Escouade, ouvre Réglages › Services connectés › « Hébergeurs de code », puis « Connecter un hébergeur… », et choisis Bitbucket dans « Hébergeur ».
2. « Créer un jeton » ouvre `https://id.atlassian.com/manage-profile/security/api-tokens`. Crée un jeton d'API **avec des droits** (scopes), pour Bitbucket, et donne-lui ceux que le formulaire rappelle : `read:user:bitbucket`, `read:repository:bitbucket`, `read:pullrequest:bitbucket`, `write:pullrequest:bitbucket`, `read:pipeline:bitbucket`.
3. Copie le jeton.

Le jeton agit en ton nom : ce qu'il ouvre ou publie l'est sous ton compte.

## 3. Connecter

1. « Adresse » : vide, pour bitbucket.org.
2. « E-mail » : l'e-mail de ton compte Atlassian.
3. « Jeton » : le jeton que tu viens de créer.
4. Clique sur « Connecter ». Escouade demande à Bitbucket qui tu es. La ligne dit « Connecté · @ton-nom » et « Pas de tickets : PR et CI seulement ».

Un dépôt dont le remote est sur bitbucket.org sans compte propose aussi « Connecter » suivi de son hôte, dans « Hébergeurs de code ».

Les PR n'ont rien à lier : chaque projet trouve son compte par son remote.

## Ce qu'Escouade garde, et où

- Le jeton va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul, et la ligne le dit.
- L'e-mail, l'adresse et ton nom sont dans `~/.escouade/integrations.json`.

« Déconnecter » oublie le compte et son jeton. Sans compte, rien n'est bloqué : la validation d'un ticket pousse sa branche et ouvre dans ton navigateur la page de création de la PR, remplie.

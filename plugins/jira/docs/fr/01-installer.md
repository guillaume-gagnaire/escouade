# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer le jeton d'API chez Atlassian

1. Dans Escouade, ouvre Réglages › Services connectés. Dans « Sources de tickets », groupe « Comptes connectés », clique sur « Connecter… » à la ligne Jira.
2. Clique sur « Créer un jeton d'API Atlassian ». Ton navigateur ouvre `https://id.atlassian.com/manage-profile/security/api-tokens`. Crée un jeton d'API et copie-le dans le champ « Jeton d'API ».

Le jeton agit en ton nom, avec les droits de ton compte sur le site : il voit les projets et les tickets que tu vois.

## 3. Remplir le formulaire

| Champ | À renseigner |
| --- | --- |
| « Site » | L'adresse de ton site Jira Cloud, par exemple `atlas.atlassian.net`. Sans `https://`, Escouade l'ajoute. Seul https est accepté (http seulement vers cette machine). |
| « E-mail » | L'e-mail du compte Atlassian auquel appartient le jeton, par exemple `ada@atlas.dev`. |
| « Jeton d'API » | Le jeton que tu viens de créer. |

## 4. Connecter

Clique sur « Connecter ». Le bouton dit « Vérification… » le temps qu'Escouade demande à Jira qui tu es. Jira accepte : la ligne dit « Connecté · ada@atlas.dev · atlas.atlassian.net ». Jira refuse : sa réponse s'affiche en rouge sous le formulaire (voir **Dépannage**). Un champ vide est signalé avant tout appel : « Indique l'adresse du site. », « Indique l'e-mail. » ou « Indique le jeton. ».

La connexion est gardée tout de suite : elle ne passe pas par « Enregistrer ».

## 5. Lier un projet Jira à un projet Escouade

Dans Réglages › Projets › Tickets importés, le groupe « Sources liées à `<projet>` » propose la liste « Projet Jira » : les projets de ton site, rangés par clé, avec leur nom. Choisis-en un. Escouade charge ses statuts et pré-remplit la « Correspondance des statuts » (voir **Utiliser**). Enregistre les réglages.

Tu peux aussi partir du Kanban : « Importer » sans source liée affiche « Aucune source liée à ce projet » et propose « Lier une source », qui ouvre ce même onglet.

## Ce qu'Escouade garde, et où

- Le jeton va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Il y est rangé sous le nom du plugin, `plugin:dev.escouade.jira:account:jira`, et sous l'ancien nom `jira`, que lit une version d'Escouade d'avant les plugins.
- Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul. Le compte le dit : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »
- Le site, l'e-mail et ton identifiant Jira sont dans `~/.escouade/integrations.json`. Le projet lié et la correspondance des statuts sont dans les réglages d'Escouade, avec ceux du projet.

« Déconnecter » oublie le compte et son jeton. Les projets gardent leur projet Jira lié, inutilisé jusqu'à une nouvelle connexion.

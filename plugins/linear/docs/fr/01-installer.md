# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer la clé d'API chez Linear

1. Dans Escouade, ouvre Réglages › Services connectés. Dans « Sources de tickets », groupe « Comptes connectés », clique sur « Connecter… » à la ligne Linear.
2. Clique sur « Créer une clé d'API Linear ». Ton navigateur ouvre `https://linear.app/settings/account/security`. Crée une clé d'API personnelle et copie-la.
3. Colle-la dans le champ « Clé d'API ».

La clé agit en ton nom : elle voit les équipes et les issues que ton compte voit.

## 3. Connecter

Clique sur « Connecter ». Le bouton dit « Vérification… » le temps qu'Escouade demande à Linear qui tu es. Linear accepte : la ligne dit « Connecté · » suivi de ton e-mail chez Linear. Linear refuse : sa réponse s'affiche en rouge sous le formulaire (voir **Dépannage**). Un champ vide est signalé avant tout appel : « Indique le jeton. ».

La connexion est gardée tout de suite : elle ne passe pas par « Enregistrer ».

## 4. Lier une équipe à un projet

Dans Réglages › Projets › Tickets importés, le groupe « Sources liées à `<projet>` » propose la liste « Équipe Linear » : tes équipes, avec leur clé (« Moteur (ENG) »). Choisis-en une. Escouade charge ses états et pré-remplit la « Correspondance des statuts » (voir **Utiliser**). Enregistre les réglages.

Tu peux aussi partir du Kanban : « Importer » sans source liée affiche « Aucune source liée à ce projet » et propose « Lier une source », qui ouvre ce même onglet.

## Ce qu'Escouade garde, et où

- La clé d'API va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Elle y est rangée sous le nom du plugin, `plugin:dev.escouade.linear:account:linear`, et sous l'ancien nom `linear`, que lit une version d'Escouade d'avant les plugins.
- Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, elle reste dans `~/.escouade/integrations.json`, lisible par toi seul. Le compte le dit : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »
- Le reste du compte (ton e-mail et ton identifiant Linear) est dans `~/.escouade/integrations.json`. L'équipe liée et la correspondance des statuts sont dans les réglages d'Escouade, avec ceux du projet.

« Déconnecter » oublie le compte et sa clé. Les projets gardent leur équipe liée, inutilisée jusqu'à une nouvelle connexion.

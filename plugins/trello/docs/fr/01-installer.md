# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer la clé et le jeton chez Trello

Trello demande deux choses : une clé d'API, puis un jeton donné à cette clé.

1. Dans Escouade, ouvre Réglages › Services connectés. Dans « Sources de tickets », groupe « Comptes connectés », clique sur « Connecter… » à la ligne Trello.
2. Clique sur « Obtenir une clé d'API ». Ton navigateur ouvre `https://trello.com/power-ups/admin`. Crée un Power-Up si tu n'en as pas (son nom n'a pas d'importance), puis génère sa clé d'API. Copie-la dans le champ « Clé d'API ».
3. Clique sur « Obtenir un jeton pour cette clé » (le lien reste grisé tant que la clé est vide). Trello ouvre sa page d'autorisation au nom d'« Escouade » : accès en lecture et en écriture à ton compte, sans expiration. Accepte, puis copie le jeton affiché dans le champ « Jeton ».

Le jeton agit en ton nom sur tous les tableaux auxquels ton compte a accès. Tu peux le révoquer à tout moment depuis les réglages de ton compte Trello.

## 3. Connecter

Clique sur « Connecter ». Le bouton dit « Vérification… » le temps qu'Escouade demande à Trello qui tu es. Trello accepte : la ligne dit « Connecté · @ton-identifiant ». Trello refuse : sa réponse s'affiche en rouge sous le formulaire (voir **Dépannage**). Un champ vide est signalé avant tout appel : « Indique la clé d'API. » ou « Indique le jeton. ».

La connexion est gardée tout de suite : elle ne passe pas par « Enregistrer ».

## 4. Lier un tableau à un projet

Dans Réglages › Projets › Tickets importés, le groupe « Sources liées à `<projet>` » propose la liste « Tableau Trello » : tes tableaux ouverts. Choisis-en un. Escouade charge alors ses listes et pré-remplit la « Correspondance des statuts » (voir **Utiliser**). Enregistre les réglages.

Tu peux aussi partir du Kanban : « Importer » sans source liée affiche « Aucune source liée à ce projet » et propose « Lier une source », qui ouvre ce même onglet.

## Ce qu'Escouade garde, et où

- La clé et le jeton vont dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Ils y sont rangés sous le nom du plugin, `plugin:dev.escouade.trello:account:trello`, et sous l'ancien nom `trello`, que lit une version d'Escouade d'avant les plugins.
- Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, ils restent dans `~/.escouade/integrations.json`, lisible par toi seul. Le compte le dit : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »
- Le reste du compte (ton identifiant Trello) est dans `~/.escouade/integrations.json`. Le tableau lié et la correspondance des statuts sont dans les réglages d'Escouade, avec ceux du projet.

« Déconnecter » oublie le compte et ses secrets. Les projets gardent leur tableau lié, inutilisé jusqu'à une nouvelle connexion.

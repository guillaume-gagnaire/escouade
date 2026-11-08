# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Créer la clé d'API chez ClickUp

1. Dans Escouade, ouvre Réglages › Services connectés. Dans « Sources de tickets », groupe « Comptes connectés », clique sur « Connecter… » à la ligne ClickUp.
2. Clique sur « Créer une clé d'API ClickUp ». Ton navigateur ouvre `https://app.clickup.com/settings/apps`, la page « Apps » de tes réglages ClickUp. Génère ta clé d'API personnelle (elle commence par `pk_`) et copie-la.
3. Colle-la dans le champ « Clé d'API ».

La clé agit en ton nom sur tous les espaces de travail auxquels ton compte a accès.

La connexion d'une application ClickUp par OAuth n'est pas proposée : la clé personnelle suffit.

## 3. Connecter

Clique sur « Connecter ». Le bouton dit « Vérification… » le temps qu'Escouade demande à ClickUp qui tu es. ClickUp accepte : la ligne dit « Connecté · » suivi de ton nom chez ClickUp (ton e-mail s'il n'y en a pas). ClickUp refuse : sa réponse s'affiche en rouge sous le formulaire (voir **Dépannage**). Un champ vide est signalé avant tout appel : « Indique le jeton. ».

La connexion est gardée tout de suite : elle ne passe pas par « Enregistrer ».

## 4. Lier une liste à un projet

Dans Réglages › Projets › Tickets importés, le groupe « Sources liées à `<projet>` » propose la liste « Liste ClickUp ». Elle range les listes de tous tes espaces de travail par espace et par dossier (« Produit › Sprints »), comme la barre latérale de ClickUp : les listes hors dossier d'abord, sans les archivées. Le nom de l'espace de travail vient devant quand tu en as plusieurs. Menu ouvert, taper les premières lettres d'un nom y va.

Choisis une liste. Escouade charge ses statuts et pré-remplit la « Correspondance des statuts » (voir **Utiliser**). Enregistre les réglages.

Tu peux aussi partir du Kanban : « Importer » sans source liée affiche « Aucune source liée à ce projet » et propose « Lier une source », qui ouvre ce même onglet.

## Ce qu'Escouade garde, et où

- La clé d'API va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Elle y est rangée sous le nom du plugin, `plugin:dev.escouade.clickup:account:clickup`, et sous l'ancien nom `clickup`, que lit une version d'Escouade d'avant les plugins.
- Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, elle reste dans `~/.escouade/integrations.json`, lisible par toi seul. Le compte le dit : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »
- Le reste du compte (ton nom et ton identifiant ClickUp) est dans `~/.escouade/integrations.json`. La liste liée et la correspondance des statuts sont dans les réglages d'Escouade, avec ceux du projet.

« Déconnecter » oublie le compte et sa clé. Les projets gardent leur liste liée, inutilisée jusqu'à une nouvelle connexion.

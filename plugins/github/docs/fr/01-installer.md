# Installer

## 1. L'avoir dans Escouade

Ouvre la vue Plugins (son icône est dans la barre des projets), puis « Installés ». Escouade donne ce plugin avec l'app : selon ta version, il est rangé sous « Livrés avec Escouade » ou sous « Proposés par Escouade » (il est dans l'installeur, rien à télécharger). S'il y est marqué « Pas encore installé », ou si tu l'as désinstallé, « Installer » le remet.

Sa page, dans la même vue, a un interrupteur pour toute l'app et un onglet « Projets » pour l'activer ou le couper projet par projet.

## 2. Choisir un jeton

Deux façons :

- **La CLI `gh`.** Si `gh` est installée et connectée (`gh auth login`), laisse le champ du jeton vide : Escouade demande son jeton à `gh` (`gh auth token`) quand il le faut, et n'en garde aucun. Le compte s'affiche alors avec « Jeton de gh ».
- **Un jeton personnel.** Crée-le chez GitHub :
  - un jeton « classic » avec le droit `repo` ;
  - ou un jeton « fine-grained » sur les dépôts voulus, avec en écriture : Contents, Pull requests et Issues ; en lecture : Checks, Commit statuses, Actions et Deployments. Pour relancer des jobs, Actions doit être en écriture.

Le jeton agit en ton nom : ce qu'il ouvre ou publie l'est sous ton compte.

## 3. Connecter

1. Dans Escouade, ouvre Réglages › Services connectés › « Hébergeurs de code », puis « Connecter un hébergeur… ». Un dépôt dont le remote est sur une instance sans compte propose aussi « Connecter » suivi de son hôte.
2. « Hébergeur » : GitHub.
3. « Adresse » : vide pour github.com ; sinon celle de ton instance (`https://acme.ghe.com`, `https://github.acme.dev`). Seul https est accepté (http seulement vers cette machine).
4. « Jeton (vide : celui de gh) » : colle ton jeton, ou laisse vide pour celui de `gh`. Le formulaire rappelle « Droits à donner au jeton : repo » et « Créer un jeton » ouvre la page des jetons de l'instance.
5. Clique sur « Connecter ». Escouade demande à GitHub qui tu es. La ligne dit « Connecté · @ton-login ». Pour un jeton classic, GitHub dit ses droits : s'il manque `repo`, la ligne dit « Droits manquants : repo ».

Le compte de github.com sert aussi à GitHub Issues : sa ligne dit « Sert aussi à GitHub Issues ». Un GitHub Enterprise sert aux PR et à la CI seulement.

Tu peux aussi connecter GitHub Issues depuis Réglages › Services connectés › « Sources de tickets » : sa ligne a un champ « Jeton (vide : celui de gh) » et le lien « Créer un jeton GitHub ». C'est le même compte que celui de github.com dans « Hébergeurs de code ».

## 4. Lier un dépôt à un projet pour ses issues

Dans le groupe « Sources liées à `<projet>` », la liste « Dépôt GitHub Issues » propose tes dépôts ; celui du projet, quand son remote est sur github.com, vient en premier (« (dépôt du projet) »). Choisis-le, puis enregistre les réglages. Depuis le Kanban, « Importer » sans source liée propose directement « Lier acme/api (GitHub Issues) ».

Les PR, elles, n'ont rien à lier : chaque projet trouve son compte par son remote.

## Ce qu'Escouade garde, et où

- Ton jeton va dans le trousseau du système : le Gestionnaire d'identification sous Windows, le Trousseau d'accès sous macOS. Sous Linux, qui n'a pas de trousseau pris en charge, ou quand le trousseau refuse l'écriture, il reste dans `~/.escouade/integrations.json`, lisible par toi seul, et la ligne le dit.
- Pour un compte qui passe par `gh`, Escouade ne garde aucun jeton : elle redemande celui de `gh` quand il le faut.
- Le reste du compte (l'adresse de l'instance, ton login) est dans `~/.escouade/integrations.json`.

« Déconnecter » oublie le compte et son jeton. Une instance sans compte n'empêche rien : la validation d'un ticket pousse sa branche et ouvre dans ton navigateur la page de création de la PR, remplie.

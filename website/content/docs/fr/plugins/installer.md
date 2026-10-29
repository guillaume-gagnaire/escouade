# Installer un plugin

Un plugin ajoute à Escouade un thème, un harness, une source du Kanban, un webhook ou des outils. Cette page dit comment l’installer, l’activer projet par projet, lui retirer une permission, lire son journal, le mettre à jour et le désinstaller.

## La vue Plugins

L’icône en forme de pièce de puzzle de la barre des projets, sous les statistiques, ouvre la vue Plugins en plein écran ; un second clic revient au projet. Réglages › Plugins mène au même endroit.

Sa barre latérale :

- **Store** : le catalogue de store.escouade.dev, qui n’est pas encore lu dans l’app ; en attendant, on installe depuis un dossier ou un dépôt git.
- **Installés** : tous les plugins, en deux groupes, « Ajoutés » et « Livrés avec Escouade ». Une ligne par plugin : sa version, « Non vérifié » ou « En développement », la mise à jour trouvée, d’où il vient, les projets où il est actif (une pastille par projet, un cadenas pour un projet privé) et son interrupteur pour toute l’app.
- **Mises à jour** : les versions plus récentes trouvées, leur nombre en ambre.
- **En développement** : les dossiers chargés en place.
- **Les catégories** : Harness, Sources du Kanban, Hébergeurs de code, Thèmes, Notifications, Autorisations, Outils, Langues, tirées de ce que chaque plugin ajoute.

## Installer

Deux boutons en tête de la vue :

- **Depuis un dossier…** : le dossier du plugin, celui de son `escouade-plugin.json`. Escouade le copie dans ses données (`~/.escouade/plugins/<id>`), sans son historique git, et le fixe par l’empreinte SHA-256 de ses fichiers : une copie modifiée ensuite se charge désactivée, et le dit.
- **Depuis un dépôt git…** : l’adresse du dépôt et une étiquette (`v1.2.0`). Escouade le clone à cette étiquette seule et le fixe par son commit.

Rien n’est installé tout de suite : le plugin est lu, mis de côté, et la fenêtre d’installation s’ouvre.

## La fenêtre d’installation

Elle dit, avant que rien ne soit installé :

- **qui l’a fait** : son nom, son auteur, sa version, sa licence et son id ;
- **d’où il vient** : le dossier, ou le dépôt et l’étiquette ; sa version et son commit ; son empreinte ; sa signature, « Aucune » hors du store. Un plugin hors du store est **non vérifié** : personne ne l’a relu, et la fenêtre le dit en tête ;
- **ce qu’il ajoute** : ses thèmes, ses harness, ses webhooks, ses réglages…, et où les trouver ;
- **ce qu’il pourra faire**, permission par permission, la plus forte d’abord :

| Permission | Ce que dit la fenêtre |
| --- | --- |
| `run` | « Lance un programme sur ton ordinateur, avec tes droits », en gras |
| `permissions.decide` | « Décide des autorisations de tes agents », en rouge |
| `permissions.commands` | « Lance des commandes », et lesquelles |
| `permissions.events` | « Reçoit ces événements », et lesquels |
| `permissions.network` | « Se connecte à », et l’hôte : déclaré, Escouade ne filtre pas encore le réseau d’un programme |
| `permissions.secrets` | « Garde un secret : », et son nom |

« Installer quand même » installe le plugin ; « Annuler » efface ce qui avait été mis de côté. Un manifeste qui ne tient pas n’ouvre pas la fenêtre : la raison s’affiche en tête de la vue, champ par champ (voir [le manifeste](plugins/manifest)).

## Activer pour l’app ou par projet

- **Pour toute l’app** : l’interrupteur de sa ligne, ou « Désactiver » sur sa page. Désactivé, rien de lui ne tourne et aucun projet ne le reçoit ; son process est arrêté.
- **Par projet** : l’onglet Projets de sa page, une ligne par projet ouvert. Un plugin installé est actif dans chaque projet, **sauf les projets privés** : un projet privé est coupé d’office, et la ligne le dit (« rien ne lui parvient »). L’y activer est un choix fait un par un ; la ligne rappelle alors qu’Escouade ne peut pas empêcher un programme de lire le disque.

> [!NOTE]
> Un projet qui propose un plugin dans son `.escouade.yml` ne l’installe ni ne l’active jamais seul : c’est toujours la fenêtre d’installation, et ton accord.

## Retirer une permission

L’onglet Permissions de sa page liste ce qu’il peut faire, avec un interrupteur par permission. En retirer une arrête son process ; le suivant démarre sans elle, et le plugin continue, ou s’arrête s’il ne peut pas. Un secret retiré ne lui est plus donné : `secrets/get` lui répond une erreur pour cette clé. Le programme lui-même ne se retire pas : pour l’arrêter, désactive le plugin.

## Ses réglages et ses secrets

L’onglet Réglages montre les champs qu’il déclare (`contributes.settings`, `contributes.secrets`). Un réglage est gardé avec la liste des plugins installés et lui est donné par `settings/get`. Un secret va au trousseau du système, sous un nom à lui (`plugin:<id>:<clé>`) : l’onglet dit seulement s’il est gardé, et « Oublier » l’efface.

## Le journal

L’onglet Journal montre où en est son process (prêt, en marche, relancé après un plantage, arrêté après trois plantages, arrêté pour sa mémoire…) et les 500 dernières lignes de son journal : ses démarrages, ses arrêts et leur raison, ses plantages, sa sortie d’erreur, ses messages, les appels restés sans réponse et ce qui lui a été refusé. Il est relu toutes les deux secondes, se filtre par genre et se copie en entier. Les secrets qu’il a reçus y sont masqués.

- **Démarrer** lance un plugin qui n’a pas encore tourné (il démarre seul à sa première activation).
- **Relancer** relance un plugin arrêté, ses plantages oubliés.

Le fichier du journal est `~/.escouade/plugin-logs/<id>.log` ([le protocole](plugins/api#le-journal)).

## Les mises à jour

Au démarrage, Escouade cherche une version plus récente de chaque plugin : une étiquette plus haute de son dépôt git, une version plus haute dans le dossier d’où il a été copié. Ce qu’il trouve est **proposé, jamais installé seul** : « v1.3.0 disponible » sur sa ligne, « Mettre à jour vers la v1.3.0 » sur sa page, la liste dans Mises à jour, et « Tout mettre à jour » en tête des installés.

Une mise à jour passe par la fenêtre d’installation, de la version installée à la nouvelle. **Si elle demande plus**, la fenêtre le dit (« ton accord est redemandé ») et marque « nouveau » chaque permission en plus. Tes choix restent : ses projets, les permissions retirées, ses réglages et ses secrets. « Tout mettre à jour » ouvre la fenêtre de chacune, l’une après l’autre.

## Désinstaller

« Désinstaller », sur sa page, arrête son process et efface son dossier, ses réglages, ses secrets et son journal. La case « Garder ses réglages et ses secrets pour une prochaine installation » les garde : réinstallé, le plugin les retrouve. Un dossier en développement reste où il est.

## Le mode développeur

En développement, « Charger un dossier… » lit le dossier d’un plugin où il est, sans copie ni empreinte, et le relit dès qu’un fichier change. Son journal se lit en direct dans l’onglet Journal. Voir [le manifeste](plugins/manifest) et [le protocole](plugins/api) pour l’écrire.
